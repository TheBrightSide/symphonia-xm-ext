use std::rc::Rc;
use crate::{effect, fixed_length_string, instrument, note};

use bitfield_struct::bitfield;
use nom::{error::ParseError, sequence::tuple, IResult, Parser};

const XM_PATTERN_HEADER_SIZE: usize = 9;

type XmPatternOrderTableRaw<'a> = &'a [u8];

#[derive(Debug)]
pub struct XmPatternHeader {
    pub header_length: u32,
    pub packing_type: u8,
    pub rows_num: u16,
    pub packed_data_size: u16,
}

pub struct XmPatternRow(pub Vec<XmPatternSlot>);

pub struct XmPatternRows(pub Vec<XmPatternRow>);

#[bitfield(u8)]
pub struct XmNoteFlags {
    pub note_follows: bool,
    pub instrument_follows: bool,
    pub volume_column_byte_follows: bool,
    pub effect_type_follows: bool,
    pub effect_parameter_follows: bool,

    #[bits(3)]
    __: u8,
}

pub struct XmPatternSlot {
    note: note::XmNote,
    instrument: Option<instrument::XmInstrumentHeader>,
    volume_column: Option<effect::XmVolumeColumn>,
    effect: Option<effect::XmEffect>,
}

pub(crate) fn parse_order_table_raw<'a>(
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

fn parse_header<'a>(data: &'a [u8]) -> IResult<&'a [u8], (XmPatternHeader, &'a [u8])> {
    let (input, (header_length, packing_type, rows_num, packed_data_size)) = tuple((
        nom::number::complete::le_u32, // Pattern header length
        nom::number::complete::u8,     // Packing type
        nom::combinator::verify(nom::number::complete::le_u16, |e| (1..=256).contains(e)), // Number of rows in pattern
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

fn parse_slot<'a>(data: &'a [u8]) -> IResult<&'a [u8], XmPatternSlot> {
    let (input, note_or_flags) = nom::number::complete::u8(data)?;
    let is_flags = ((note_or_flags & (0x1 << 7)) >> 7) == 1;

    if is_flags {
        let flags = XmNoteFlags(note_or_flags);

        let (input, (note, instrument, volume_column)) = tuple((
            nom::combinator::cond(flags.note_follows(), note::parse_xm_note)
                .map(|e| e.unwrap_or(note::XmNote::NoNote)),
            nom::combinator::cond(flags.instrument_follows(), nom::number::complete::u8),
            nom::combinator::cond(flags.volume_column_byte_follows(), effect::parse_volume_column),
        ))(input)?;

        let (input, effect) = effect::parse_effect(
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
            effect::parse_volume_column,
            effect::parse_effect(true, true),
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

fn parse_row<'a>(
    channels_num: u16,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], XmPatternRow> {
    move |data| {
        nom::multi::count(parse_slot, channels_num as usize)
            .map(|e| XmPatternRow(e))
            .parse(data)
    }
}

pub(crate) fn parse<'a>(
    channels_num: u16,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], (XmPatternHeader, XmPatternRows, &'a [u8])> {
    move |data| {
        let (input, (header, excess)) = parse_header(data)?;

        let (input, notes) =
            nom::multi::count(parse_row(channels_num), header.rows_num as usize)
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

