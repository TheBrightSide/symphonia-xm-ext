use nom::{bytes::complete::take, combinator::map_res, error::ParseError, IResult};

mod effect;
mod header;
mod instrument;
mod note;
mod pattern;

#[cfg(test)]
mod tests;

type XmSamplePair = (instrument::XmSampleHeader, instrument::XmSamplePcmData);

type XmInstrumentCollection = Vec<(instrument::XmInstrumentHeader, Vec<XmSamplePair>)>;

type XmPattern = (pattern::XmPatternHeader, pattern::XmPatternRows);

type XmPatternCollection = Vec<XmPattern>;

pub struct XmFormat {
    pub header: header::XmHeader,
    pub patterns: XmPatternCollection,
    pub instruments: XmInstrumentCollection,
    pub pattern_order_table: pattern::XmPatternOrderTable,
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
        .map(|e| (e.0, e.1.to_owned()))
    }
}

pub fn parse(data: &[u8]) -> IResult<&[u8], XmFormat> {
    let (input, header) = header::parse(data)?;
    let (input, pattern_order_table) =
        pattern::parse_order_table_raw(input, header.0.song_length as usize, header.3 as usize)?;
    let (input, patterns) = nom::multi::count(
        pattern::parse(header.0.channels_num),
        header.0.patterns_num as usize,
    )(input)?;
    let (input, instruments) =
        nom::multi::count(instrument::parse, header.0.instruments_num as usize)(input)?;

    Ok((
        input,
        XmFormat {
            header: header.0,
            patterns: patterns.into_iter().map(|e| (e.0, e.1)).collect::<Vec<_>>(),
            instruments,
            pattern_order_table,
        },
    ))
}
