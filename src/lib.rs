use std::{num::NonZeroU16, rc::Rc, time::Duration};

use bitfield_struct::bitfield;
use effect::{parse_xm_volume_column, XmVolumeColumn};
use nom::{
    bytes::complete::take,
    combinator::{cond, map_res, verify},
    error::ParseError,
    sequence::tuple,
    IResult, Parser,
};

mod effect;
mod note;

#[cfg(test)]
mod tests;

pub struct XmFormat {
    header: XmHeader,
    patterns: Vec<(XmPatternHeader, Vec<XmPatternSlot>)>,
    instruments: Vec<(XmInstrument, Vec<(XmSampleHeader, XmSampleData)>)>,
}

#[derive(Debug)]
pub struct XmHeader {
    module_name: String,
    tracker_name: String,
    version: u16,
    // pattern order table size
    song_length: u16,
    restart_pos: u16,
    channels_num: u16,
    patterns_num: u16,
    instruments_num: u16,
    is_amiga: bool,
    default_tempo: u16,
    default_bpm: u16,
}

type XmPatternOrderTableRaw<'a> = &'a [u8];

struct XmPatternOrderTable(Vec<Rc<XmInstrument>>);

#[derive(Debug)]
pub struct XmPatternHeader {
    header_length: u32,
    packing_type: u8,
    rows_num: u16,
    packed_data_size: u16,
}

pub struct XmPatternRow(pub Vec<XmPatternSlot>);

pub struct XmPatternRows(pub Vec<XmPatternRow>);

#[bitfield(u8)]
pub struct XmNoteFlags {
    note_follows: bool,
    instrument_follows: bool,
    volume_column_byte_follows: bool,
    effect_type_follows: bool,
    effect_parameter_follows: bool,

    #[bits(3)]
    __: u8,
}

pub struct XmPatternSlot {
    note: note::XmNote,
    instrument: Option<XmInstrument>,
    volume_column: Option<effect::XmVolumeColumn>,
    effect: Option<effect::XmEffect>,
}

#[bitfield(u8)]
pub struct XmEnvelopeType {
    on: bool,
    sustain: bool,
    loop_: bool,

    #[bits(5)]
    __: u8,
}

#[derive(Debug)]
pub enum XmVibratoType {
    Sine,
    Square,
    RampUp,
    RampDown,
}

#[derive(Default, Debug)]
pub struct XmEnvelopePoint {
    frame: u16,
    value: u16,
}

#[derive(Debug)]
pub struct XmEnvelope {
    points: Vec<XmEnvelopePoint>,
    sustain_point: Option<u8>,
    loop_start_point: Option<u8>,
    loop_end_point: Option<u8>,
}

#[derive(Debug)]
pub struct XmVibratoOpts {
    vibrato_type: XmVibratoType,
    vibrato_sweep: u8,
    vibrato_depth: u8,
    vibrato_rate: u8,
}

#[derive(Debug)]
pub struct XmInstrumentSampleOpts {
    sample_header_size: u32,
    sample_keymap_assignments: [u8; 96],
    volume_envelope: Option<XmEnvelope>,
    panning_envelope: Option<XmEnvelope>,
    vibrato: XmVibratoOpts,
    volume_fadeout: u16,
}

#[derive(Debug)]
pub struct XmInstrument {
    header_size: u32,
    name: String,
    kind: u8,
    samples_num: u16,
    sample_opts: Option<XmInstrumentSampleOpts>,
}

#[repr(u8)]
#[derive(Debug)]
pub enum XmSampleLoopType {
    NoLoop,
    ForwardLoop,
    BidirectionalLoop,
    Unknown,
}

impl XmSampleLoopType {
    const fn from_bits(value: u8) -> Self {
        match value {
            0 => XmSampleLoopType::NoLoop,
            1 => XmSampleLoopType::ForwardLoop,
            2 => XmSampleLoopType::BidirectionalLoop,
            _ => XmSampleLoopType::Unknown,
        }
    }

    const fn into_bits(self) -> u8 {
        self as _
    }
}

#[repr(u8)]
#[derive(Debug)]
pub enum XmSampleBitRate {
    Bit8,
    Bit16,
    Unknown,
}

impl XmSampleBitRate {
    const fn from_bits(value: u8) -> Self {
        match value {
            0 => XmSampleBitRate::Bit8,
            1 => XmSampleBitRate::Bit16,
            _ => XmSampleBitRate::Unknown,
        }
    }

    const fn into_bits(self) -> u8 {
        self as _
    }
}

#[bitfield(u8)]
pub struct XmSampleType {
    #[bits(2)]
    loop_type: XmSampleLoopType,

    #[bits(2)]
    __: u8,

    #[bits(1)]
    bitrate: XmSampleBitRate,

    #[bits(3)]
    __: u8,
}

#[repr(u8)]
pub enum XmSampleDataKind {
    RegularDelta = 0x00,
    ADPCM4Bit = 0xAD,
}

pub struct XmSampleHeader {
    length: u32,
    loop_start: u32,
    loop_length: u32,
    volume: u8,
    finetune: i8,
    kind: XmSampleType,
    panning: u8,
    relative_note_num: i8,
    data_kind: XmSampleDataKind,
    name: String,
}

pub type XmSampleData = Vec<u8>;

pub struct XmParseError<'a>(Vec<(&'a [u8], nom::error::ErrorKind)>);

impl<'a> ParseError<&'a [u8]> for XmParseError<'a> {
    fn from_error_kind(input: &'a [u8], kind: nom::error::ErrorKind) -> Self {
        Self(vec![(input, kind)])
    }

    fn append(input: &'a [u8], kind: nom::error::ErrorKind, other: Self) -> Self {
        let mut tmp = other.0;
        tmp.push((input, kind));
        Self(tmp)
    }
}

fn fixed_length_string<'a>(length: usize) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], String> {
    move |input| {
        map_res(take(length), |bytes: &[u8]| {
            std::str::from_utf8(bytes)
                .map(|s| s.trim_end_matches('\0').to_string())
                .map_err(|_| {
                    nom::Err::Error(nom::error::Error::from_error_kind(
                        input,
                        nom::error::ErrorKind::MapRes,
                    ))
                })
        })(input)
        .map(|e| (e.0, e.1.trim().to_owned()))
    }
}

fn parse_xm_envelope_point<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmEnvelopePoint> {
    let (input, (x, y)) =
        tuple((nom::number::complete::le_u16, nom::number::complete::le_u16))(data)?;

    Ok((input, XmEnvelopePoint { frame: x, value: y }))
}

fn parse_xm_envelope_points<'a>(
    length: usize,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], Vec<XmEnvelopePoint>> {
    nom::multi::count(parse_xm_envelope_point, length)
}

fn parse_xm_envelope_type<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmEnvelopeType> {
    let (input, byte) = nom::number::complete::u8(data)?;

    Ok((input, XmEnvelopeType(byte)))
}

fn parse_xm_vibrato_type<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmVibratoType> {
    let (input, byte) = nom::number::complete::u8(data)?;

    match byte {
        0 => Ok((input, XmVibratoType::Sine)),
        1 => Ok((input, XmVibratoType::Square)),
        2 => Ok((input, XmVibratoType::RampDown)),
        3 => Ok((input, XmVibratoType::RampUp)),
        _ => Err(nom::Err::Error(nom::error::Error::from_error_kind(
            input,
            nom::error::ErrorKind::Verify,
        ))),
    }
}

fn parse_xm_vibrato_opts<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmVibratoOpts> {
    let (input, (vibrato_type, vibrato_sweep, vibrato_depth, vibrato_rate)) = tuple((
        parse_xm_vibrato_type,
        nom::number::complete::u8,
        nom::number::complete::u8,
        nom::number::complete::u8,
    ))(data)?;

    Ok((
        input,
        XmVibratoOpts {
            vibrato_type,
            vibrato_sweep,
            vibrato_depth,
            vibrato_rate,
        },
    ))
}

fn parse_xm_instrument_sample_opts<'a>(
    data: &'a [u8],
) -> IResult<&'a [u8], XmInstrumentSampleOpts> {
    let (
        input,
        (
            sample_header_size,
            sample_keymap_assignments,
            mut vol_envelope_points,
            mut pan_envelope_points,
            vol_points_num,
            pan_points_num,
            vol_sustain_point,
            vol_loop_start_point,
            vol_loop_end_point,
            pan_sustain_point,
            pan_loop_start_point,
            pan_loop_end_point,
            vol_type,
            pan_type,
            vibrato_opts,
            volume_fadeout,
            _reserved,
        ),
    ) = tuple((
        nom::number::complete::le_u32,       // Sample header size
        nom::bytes::complete::take(96usize), // Sample keymap assignments
        parse_xm_envelope_points(12),        // Points for volume envelope
        parse_xm_envelope_points(12),        // Points for panning envelope
        nom::number::complete::u8,           // Number of volume points
        nom::number::complete::u8,           // Number of panning points
        nom::number::complete::u8,           // Volume sustain point
        nom::number::complete::u8,           // Volume loop start point
        nom::number::complete::u8,           // Volume loop end point
        nom::number::complete::u8,           // Panning sustain point
        nom::number::complete::u8,           // Panning loop start point
        nom::number::complete::u8,           // Panning loop end point
        parse_xm_envelope_type,              // Volume type
        parse_xm_envelope_type,              // Panning type
        parse_xm_vibrato_opts,               // Vibrato options
        nom::number::complete::le_u16,       // Volume fadeout
        nom::bytes::complete::take(22usize), // Reserved data
    ))(data)?;

    let sample_keymap_assignments = <[u8; 96]>::try_from(sample_keymap_assignments)
        .expect("size of the sample keymap assignments should always be 96");

    if vol_points_num > 12 || pan_points_num > 12 {
        return Err(nom::Err::Error(nom::error::Error::from_error_kind(
            input,
            nom::error::ErrorKind::Verify,
        )));
    }

    vol_envelope_points.resize_with(vol_points_num as usize, Default::default);
    pan_envelope_points.resize_with(pan_points_num as usize, Default::default);

    Ok((
        input,
        XmInstrumentSampleOpts {
            sample_header_size,
            sample_keymap_assignments,
            volume_envelope: if vol_type.on() {
                Some(XmEnvelope {
                    points: vol_envelope_points,
                    sustain_point: if vol_type.sustain() {
                        Some(vol_sustain_point)
                    } else {
                        None
                    },
                    loop_start_point: if vol_type.loop_() {
                        Some(vol_loop_start_point)
                    } else {
                        None
                    },
                    loop_end_point: if vol_type.loop_() {
                        Some(vol_loop_end_point)
                    } else {
                        None
                    },
                })
            } else {
                None
            },
            panning_envelope: if pan_type.on() {
                Some(XmEnvelope {
                    points: pan_envelope_points,
                    sustain_point: if pan_type.sustain() {
                        Some(pan_sustain_point)
                    } else {
                        None
                    },
                    loop_start_point: if pan_type.loop_() {
                        Some(pan_loop_start_point)
                    } else {
                        None
                    },
                    loop_end_point: if pan_type.loop_() {
                        Some(pan_loop_end_point)
                    } else {
                        None
                    },
                })
            } else {
                None
            },
            vibrato: vibrato_opts,
            volume_fadeout,
        },
    ))
}

fn parse_xm_instrument<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmInstrument> {
    let (input, (header_size, name, kind, samples_num)) = tuple((
        nom::number::complete::le_u32,
        fixed_length_string(22),
        nom::number::complete::u8,
        nom::number::complete::le_u16,
    ))(data)?;

    let (input, sample_opts) = cond(samples_num > 0, parse_xm_instrument_sample_opts)(input)?;

    Ok((
        input,
        XmInstrument {
            header_size,
            name,
            kind,
            samples_num,
            sample_opts,
        },
    ))
}

fn parse_xm_header<'a>(data: &'a [u8]) -> IResult<&'a [u8], (XmHeader, String, u8, u32)> {
    let (
        input,
        (
            id_text,
            module_name,
            ox1a,
            tracker_name,
            version,
            header_size,
            song_length,
            restart_pos,
            channels_num,
            patterns_num,
            instruments_num,
            flags,
            default_tempo,
            default_bpm,
        ),
    ) = tuple((
        fixed_length_string(17),       // ID Text
        fixed_length_string(20),       // Module name
        nom::number::complete::u8,     // 0x1A
        fixed_length_string(20),       // Tracker name
        nom::number::complete::le_u16, // Version number
        nom::number::complete::le_u32, // Header size
        verify(nom::number::complete::le_u16, |e| (1..=256u16).contains(e)), // Song length
        nom::number::complete::le_u16, // Restart position
        verify(nom::number::complete::le_u16, |e| (0..128).contains(e)), // Number of channels (OpenMPT allows a max of 127)
        verify(nom::number::complete::le_u16, |e| (1..=256).contains(e)), // Number of patterns
        verify(nom::number::complete::le_u16, |e| (0..=128).contains(e)), // Number of instruments
        nom::number::complete::le_u16,                                   // Flags
        nom::number::complete::le_u16,                                   // Default tempo
        nom::number::complete::le_u16,                                   // Default BPM
    ))(data)?;

    let is_amiga = (flags & 0x1) == 0;

    Ok((
        input,
        (
            XmHeader {
                module_name,
                tracker_name,
                version,
                song_length,
                restart_pos,
                channels_num,
                patterns_num,
                instruments_num,
                is_amiga,
                default_tempo,
                default_bpm,
            },
            id_text,
            ox1a,
            header_size - 20,
        ),
    ))
}

fn parse_xm_pattern_order_table_raw<'a>(
    data: &'a [u8],
    length: usize,
    size: usize,
) -> IResult<&'a [u8], XmPatternOrderTableRaw<'a>> {
    if length > size {
        Err(nom::Err::Error(nom::error::Error::from_error_kind(
            data,
            nom::error::ErrorKind::Verify,
        )))
    } else {
        let (input, out) = nom::bytes::complete::take(length)(data)?;
        let (input, _) = nom::bytes::complete::take(size - length)(input)?;

        Ok((input, out))
    }
}

const XM_PATTERN_HEADER_SIZE: usize = 9;

fn parse_xm_pattern_header<'a>(data: &'a [u8]) -> IResult<&'a [u8], (XmPatternHeader, &'a [u8])> {
    let (input, (header_length, packing_type, rows_num, packed_data_size)) = tuple((
        nom::number::complete::le_u32, // Pattern header length
        nom::number::complete::u8,     // Packing type
        verify(nom::number::complete::le_u16, |e| (1..=256).contains(e)), // Number of rows in pattern
        nom::number::complete::le_u16, // Packed pattern data size
    ))(data)?;

    let (input, excess_data) = if header_length as usize > XM_PATTERN_HEADER_SIZE {
        nom::bytes::complete::take(header_length as usize - XM_PATTERN_HEADER_SIZE)(input)?
    } else {
        (input, &[] as &[u8])
    };

    Ok((
        input,
        (
            XmPatternHeader {
                header_length,
                packing_type,
                rows_num,
                packed_data_size,
            },
            excess_data,
        ),
    ))
}

fn parse_xm_pattern_slot<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmPatternSlot> {
    let (input, note_or_flags) = nom::number::complete::u8(data)?;
    let is_flags = ((note_or_flags & (0x1 << 7)) >> 7) == 1;

    if is_flags {
        let flags = XmNoteFlags(note_or_flags);

        let (input, (note, instrument, volume_column)) = tuple((
            cond(flags.note_follows(), note::parse_xm_note)
                .map(|e| e.unwrap_or(note::XmNote::NoNote)),
            cond(flags.instrument_follows(), nom::number::complete::u8),
            cond(flags.volume_column_byte_follows(), parse_xm_volume_column),
        ))(input)?;

        let (input, effect) = effect::parse_xm_effect(
            flags.effect_type_follows(),
            flags.effect_parameter_follows(),
        )(input)?;

        Ok((
            input,
            XmPatternSlot {
                note,
                instrument: None,
                volume_column,
                effect,
            },
        ))
    } else {
        let (input, (note, instrument, volume_column, effect)) = tuple((
            note::parse_xm_note,
            nom::number::complete::u8,
            parse_xm_volume_column,
            effect::parse_xm_effect(true, true),
        ))(data)?;

        Ok((
            input,
            XmPatternSlot {
                note,
                instrument: None,
                volume_column: Some(volume_column),
                effect,
            },
        ))
    }
}

fn parse_xm_pattern_row<'a>(
    channels_num: u16,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], XmPatternRow> {
    move |data| {
        nom::multi::count(parse_xm_pattern_slot, channels_num as usize)
            .map(|e| XmPatternRow(e))
            .parse(data)
    }
}

fn parse_xm_pattern<'a>(
    channels_num: u16,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], (XmPatternHeader, XmPatternRows, &'a [u8])> {
    move |data| {
        let (input, (header, excess)) = parse_xm_pattern_header(data)?;

        let (input, notes) =
            nom::multi::count(parse_xm_pattern_row(channels_num), header.rows_num as usize)
                .map(|e| XmPatternRows(e))
                .parse(input)?;

        Ok((input, (header, notes, excess)))
    }
}

impl std::fmt::Display for XmPatternSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: add the rest
        let effect_fmt = match self.effect {
            Some(ref v) => format!("{}", v),
            None => "...".to_owned(),
        };

        let volume_col_fmt = match self.volume_column {
            Some(ref v) => format!("{}", v),
            None => "...".to_owned(),
        };

        write!(f, "{}{}{}", self.note, volume_col_fmt, effect_fmt)
    }
}

impl std::fmt::Display for XmPatternRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for note in self.0.iter() {
            write!(f, "|")?;
            write!(f, "{}", note)?;
        }

        write!(f, "|")
    }
}

impl std::fmt::Display for XmPatternRows {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in self.0.iter() {
            writeln!(f, "{}", row)?;
        }

        std::fmt::Result::Ok(())
    }
}
