use bitfield_struct::bitfield;
use either::Either;
use nom::{error::ParseError, sequence::tuple, IResult};

const XM_INSTRUMENT_HEADER_SIZE: usize = 29;
const XM_INSTRUMENT_HEADER_SIZE_W_OPTS: usize = 263;

#[bitfield(u8)]
pub struct XmEnvelopeType {
    pub on: bool,
    pub sustain: bool,
    pub loop_: bool,

    #[bits(5)]
    __: u8,
}

#[derive(Clone, Debug)]
pub enum XmVibratoType {
    Sine,
    Square,
    RampUp,
    RampDown,
}

#[derive(Clone, Default, Debug)]
pub struct XmEnvelopePoint {
    pub frame: u16,
    pub value: u16,
}

#[derive(Clone, Debug)]
pub struct XmEnvelope {
    pub points: Vec<XmEnvelopePoint>,
    pub sustain_point: Option<u8>,
    pub loop_start_point: Option<u8>,
    pub loop_end_point: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct XmVibratoOpts {
    pub kind: XmVibratoType,
    pub sweep: u8,
    pub depth: u8,
    pub rate: u8,
}

#[derive(Clone, Debug)]
pub struct XmInstrumentSampleOpts {
    pub sample_header_size: u32,
    pub sample_keymap_assignments: [u8; 96],
    pub volume_envelope: Option<XmEnvelope>,
    pub panning_envelope: Option<XmEnvelope>,
    pub vibrato: XmVibratoOpts,
    pub volume_fadeout: u16,
}

#[derive(Clone, Debug)]
pub struct XmInstrumentHeader {
    pub header_size: u32,
    pub name: String,
    pub kind: u8,
    pub samples_num: u16,
    pub sample_opts: Option<XmInstrumentSampleOpts>,
}

#[repr(u8)]
#[derive(Clone, Debug)]
pub enum XmSampleLoopType {
    NoLoop,
    ForwardLoop,
    BidirectionalLoop,
    Unknown,
}

#[repr(u8)]
#[derive(Clone, Debug)]
pub enum XmSampleBitDepth {
    Bit8,
    Bit16,
    Unknown,
}

#[bitfield(u8)]
pub struct XmSampleType {
    #[bits(2)]
    pub loop_type: XmSampleLoopType,

    #[bits(2)]
    __: u8,

    #[bits(1)]
    pub depth: XmSampleBitDepth,

    #[bits(3)]
    __: u8,
}

#[derive(Clone, Debug)]
pub struct XmSampleHeader {
    pub length: u32,
    pub loop_start: u32,
    pub loop_length: u32,
    pub volume: u8,
    pub finetune: i8,
    pub kind: XmSampleType,
    pub panning: u8,
    pub relative_note_num: i8,
    pub name: String,
}

#[derive(Clone)]
pub enum XmSamplePcmData {
    Bit8Data(Vec<i8>),
    Bit16Data(Vec<i16>)
}

fn parse_envelope_point(data: &[u8]) -> IResult<&[u8], XmEnvelopePoint> {
    let (input, (x, y)) =
        tuple((nom::number::complete::le_u16, nom::number::complete::le_u16))(data)?;

    Ok((input, XmEnvelopePoint { frame: x, value: y }))
}

fn parse_envelope_points<'a>(
    length: usize,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], Vec<XmEnvelopePoint>> {
    nom::multi::count(parse_envelope_point, length)
}

fn parse_envelope_type(data: &[u8]) -> IResult<&[u8], XmEnvelopeType> {
    let (input, byte) = nom::number::complete::u8(data)?;

    Ok((input, XmEnvelopeType(byte)))
}

fn parse_vibrato_type(data: &[u8]) -> IResult<&[u8], XmVibratoType> {
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

fn parse_vibrato_opts(data: &[u8]) -> IResult<&[u8], XmVibratoOpts> {
    let (input, (vibrato_type, vibrato_sweep, vibrato_depth, vibrato_rate)) = tuple((
        parse_vibrato_type,
        nom::number::complete::u8,
        nom::number::complete::u8,
        nom::number::complete::u8,
    ))(data)?;

    Ok((
        input,
        XmVibratoOpts {
            kind: vibrato_type,
            sweep: vibrato_sweep,
            depth: vibrato_depth,
            rate: vibrato_rate,
        },
    ))
}

fn parse_instrument_sample_opts(data: &[u8]) -> IResult<&[u8], XmInstrumentSampleOpts> {
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
        parse_envelope_points(12),           // Points for volume envelope
        parse_envelope_points(12),           // Points for panning envelope
        nom::number::complete::u8,           // Number of volume points
        nom::number::complete::u8,           // Number of panning points
        nom::number::complete::u8,           // Volume sustain point
        nom::number::complete::u8,           // Volume loop start point
        nom::number::complete::u8,           // Volume loop end point
        nom::number::complete::u8,           // Panning sustain point
        nom::number::complete::u8,           // Panning loop start point
        nom::number::complete::u8,           // Panning loop end point
        parse_envelope_type,                 // Volume type
        parse_envelope_type,                 // Panning type
        parse_vibrato_opts,                  // Vibrato options
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

pub(crate) fn parse_instrument_header(data: &[u8]) -> IResult<&[u8], XmInstrumentHeader> {
    let (input, (header_size, name, kind, samples_num)) = tuple((
        nom::number::complete::le_u32,
        crate::fixed_length_string(22),
        nom::number::complete::u8,
        nom::number::complete::le_u16,
    ))(data)?;

    let (input, sample_opts) =
        nom::combinator::cond(samples_num > 0, parse_instrument_sample_opts)(input)?;

    let (input, _excess_data) = match sample_opts {
        Some(_) => nom::combinator::cond(
            header_size as usize > XM_INSTRUMENT_HEADER_SIZE_W_OPTS,
            nom::bytes::complete::take(header_size as usize - XM_INSTRUMENT_HEADER_SIZE_W_OPTS),
        )(input)?,
        None => nom::combinator::cond(
            header_size as usize > XM_INSTRUMENT_HEADER_SIZE,
            nom::bytes::complete::take(header_size as usize - XM_INSTRUMENT_HEADER_SIZE),
        )(input)?,
    };

    Ok((
        input,
        XmInstrumentHeader {
            header_size,
            name,
            kind,
            samples_num,
            sample_opts,
        },
    ))
}

pub(crate) fn parse_sample_header(data: &[u8]) -> IResult<&[u8], XmSampleHeader> {
    let (
        input,
        (
            length,
            loop_start,
            loop_length,
            volume,
            finetune,
            kind,
            panning,
            relative_note_num,
            _reserved,
            name,
        ),
    ) = tuple((
        nom::number::complete::le_u32, // Sample length
        nom::number::complete::le_u32, // Sample loop start
        nom::number::complete::le_u32, // Sample loop length
        nom::number::complete::u8,     // Volume
        nom::number::complete::i8,     // Finetune
        nom::combinator::map(nom::number::complete::u8, XmSampleType), // Type
        nom::number::complete::u8,     // Panning
        nom::number::complete::i8,     // Relative note number
        nom::number::complete::u8,     // Reserved (Sample data type)
        crate::fixed_length_string(22), // Sample name
    ))(data)?;

    Ok((
        input,
        XmSampleHeader {
            length,
            loop_start,
            loop_length,
            volume,
            finetune,
            kind,
            panning,
            relative_note_num,
            name,
        },
    ))
}

fn decode_dpcm_data(
    length: usize,
    depth: XmSampleBitDepth,
) -> impl FnMut(&[u8]) -> IResult<&[u8], XmSamplePcmData> {
    move |data| {
        let mut previous = match depth {
            XmSampleBitDepth::Bit8 => Either::Left(0i8),
            XmSampleBitDepth::Bit16 => Either::Right(0i16),
            XmSampleBitDepth::Unknown => {
                return Err(nom::Err::Error(nom::error::Error::from_error_kind(
                    data,
                    nom::error::ErrorKind::Verify,
                )))
            }
        };

        match previous {
            Either::Left(ref mut previous) => {
                let mut out = vec![];
                let (input, samples) = nom::multi::count(nom::number::complete::i8, length)(data)?;

                for sample in samples {
                    *previous = sample.wrapping_add(*previous);
                    out.push(*previous);
                }

                Ok((input, XmSamplePcmData::Bit8Data(out)))
            }
            Either::Right(ref mut previous) => {
                let mut out = vec![];
                let (input, samples) =
                    nom::multi::count(nom::number::complete::le_i16, length / 2)(data)?;

                for sample in samples {
                    *previous = sample.wrapping_add(*previous);
                    out.push(*previous);
                }

                Ok((input, XmSamplePcmData::Bit16Data(out)))
            }
        }
    }
}

pub(crate) fn parse(
    data: &[u8],
) -> IResult<&[u8], (XmInstrumentHeader, Vec<(XmSampleHeader, XmSamplePcmData)>)> {
    let (input, instr_header) = parse_instrument_header(data)?;
    if instr_header.samples_num == 0 {
        return Ok((input, (instr_header, vec![])));
    }

    let (mut input, sample_headers) =
        nom::multi::count(parse_sample_header, instr_header.samples_num as usize)(input)?;

    let mut sample_data_entries = vec![];
    for mut parser in sample_headers
        .iter()
        .map(|e| decode_dpcm_data(e.length as usize, e.kind.depth()))
    {
        let (input_, sample_data_entry) = parser(input)?;
        input = input_;
        sample_data_entries.push(sample_data_entry);
    }

    Ok((
        input,
        (
            instr_header,
            sample_headers
                .into_iter()
                .zip(sample_data_entries.into_iter())
                .collect::<Vec<_>>(),
        ),
    ))
}

impl XmSampleBitDepth {
    const fn from_bits(value: u8) -> Self {
        match value {
            0 => XmSampleBitDepth::Bit8,
            1 => XmSampleBitDepth::Bit16,
            _ => XmSampleBitDepth::Unknown,
        }
    }

    const fn into_bits(self) -> u8 {
        self as _
    }
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
