use bitfield_struct::bitfield;
use nom::{error::ParseError, sequence::tuple, IResult};

// const XM_SAMPLE_HEADER_SIZE: usize =

#[bitfield(u8)]
pub struct XmEnvelopeType {
    pub on: bool,
    pub sustain: bool,
    pub loop_: bool,

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
    pub frame: u16,
    pub value: u16,
}

#[derive(Debug)]
pub struct XmEnvelope {
    pub points: Vec<XmEnvelopePoint>,
    pub sustain_point: Option<u8>,
    pub loop_start_point: Option<u8>,
    pub loop_end_point: Option<u8>,
}

#[derive(Debug)]
pub struct XmVibratoOpts {
    pub kind: XmVibratoType,
    pub sweep: u8,
    pub depth: u8,
    pub rate: u8,
}

#[derive(Debug)]
pub struct XmInstrumentSampleOpts {
    pub sample_header_size: u32,
    pub sample_keymap_assignments: [u8; 96],
    pub volume_envelope: Option<XmEnvelope>,
    pub panning_envelope: Option<XmEnvelope>,
    pub vibrato: XmVibratoOpts,
    pub volume_fadeout: u16,
}

#[derive(Debug)]
pub struct XmInstrumentHeader {
    pub header_size: u32,
    pub name: String,
    pub kind: u8,
    pub samples_num: u16,
    pub sample_opts: Option<XmInstrumentSampleOpts>,
}

#[repr(u8)]
#[derive(Debug)]
pub enum XmSampleLoopType {
    NoLoop,
    ForwardLoop,
    BidirectionalLoop,
    Unknown,
}

#[repr(u8)]
#[derive(Debug)]
pub enum XmSampleBitRate {
    Bit8,
    Bit16,
    Unknown,
}

#[bitfield(u8)]
pub struct XmSampleType {
    #[bits(2)]
    loop_type: XmSampleLoopType,

    #[bits(2)]
    __: u8,

    #[bits(1)]
    bitdepth: XmSampleBitRate,

    #[bits(3)]
    __: u8,
}

#[repr(u8)]
pub enum XmSampleDataType {
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
    data_kind: XmSampleDataType,
    name: String,
}

pub type XmSampleData = Vec<u8>;

fn parse_envelope_point<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmEnvelopePoint> {
    let (input, (x, y)) =
        tuple((nom::number::complete::le_u16, nom::number::complete::le_u16))(data)?;

    Ok((input, XmEnvelopePoint { frame: x, value: y }))
}

fn parse_envelope_points<'a>(
    length: usize,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], Vec<XmEnvelopePoint>> {
    nom::multi::count(parse_envelope_point, length)
}

fn parse_envelope_type<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmEnvelopeType> {
    let (input, byte) = nom::number::complete::u8(data)?;

    Ok((input, XmEnvelopeType(byte)))
}

fn parse_vibrato_type<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmVibratoType> {
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

fn parse_vibrato_opts<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmVibratoOpts> {
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

fn parse_instrument_sample_opts<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmInstrumentSampleOpts> {
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

fn parse_instrument_header<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmInstrumentHeader> {
    let (input, (header_size, name, kind, samples_num)) = tuple((
        nom::number::complete::le_u32,
        crate::fixed_length_string(22),
        nom::number::complete::u8,
        nom::number::complete::le_u16,
    ))(data)?;

    let (input, sample_opts) =
        nom::combinator::cond(samples_num > 0, parse_instrument_sample_opts)(input)?;

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

fn parse_sample_data_type<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmSampleDataType> {
    let (input, byte) = nom::number::complete::u8(data)?;

    match byte {
        0x00 => Ok((input, XmSampleDataType::RegularDelta)),
        0xAD => Ok((input, XmSampleDataType::ADPCM4Bit)),
        _ => Err(nom::Err::Error(nom::error::Error::from_error_kind(
            input,
            nom::error::ErrorKind::Verify,
        ))),
    }
}

// fn parse_sample_header<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmSampleHeader> {
//     let (input, (
//         length,
//         loop_start,
//         loop_length,
//         volume,
//         finetune,
//         kind,
//         panning,
//         relative_note_num,
//         data_kind,
//         name
//     )) = tuple((
//         nom::number::complete::le_u32, // Sample length
//         nom::number::complete::le_u32, // Sample loop start
//         nom::number::complete::le_u32, // Sample loop length
//         nom::number::complete::u8, // Volume
//         nom::number::complete::i8, // Finetune
//         nom::combinator::map(nom::number::complete::u8, |v| XmSampleType), // Type
//         nom::number::complete::u8, // Panning
//         nom::number::complete::i8, // Relative note number
//         parse_sample_data_type, // Sample data type
//         crate::fixed_length_string(22) // Sample name
//     ))(data)?;

//     Ok()
// }

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
