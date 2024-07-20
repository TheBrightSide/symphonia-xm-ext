use nom::{bytes::complete::take, combinator::map_res, error::ParseError, IResult};

mod effect;
mod header;
mod instrument;
mod note;
mod pattern;

#[cfg(test)]
mod tests;

pub struct XmFormat {
    header: header::XmHeader,
    patterns: Vec<(pattern::XmPatternHeader, Vec<pattern::XmPatternSlot>)>,
    instruments: Vec<(
        instrument::XmInstrumentHeader,
        Vec<(instrument::XmSampleHeader, instrument::XmSampleData)>,
    )>,
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
