use nom::{sequence::tuple, IResult};

#[derive(Debug)]
pub struct XmHeader {
    pub module_name: String,
    pub tracker_name: String,
    pub version: u16,
    // pattern order table size
    pub song_length: u16,
    pub restart_pos: u16,
    pub channels_num: u16,
    pub patterns_num: u16,
    pub instruments_num: u16,
    pub is_amiga: bool,
    pub default_tempo: u16,
    pub default_bpm: u16,
}

pub(crate) fn parse<'a>(data: &'a [u8]) -> IResult<&'a [u8], (XmHeader, String, u8, u32)> {
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
        crate::fixed_length_string(17),       // ID Text
        crate::fixed_length_string(20),       // Module name
        nom::number::complete::u8,     // 0x1A
        crate::fixed_length_string(20),       // Tracker name
        nom::number::complete::le_u16, // Version number
        nom::number::complete::le_u32, // Header size
        nom::combinator::verify(nom::number::complete::le_u16, |e| (1..=256u16).contains(e)), // Song length
        nom::number::complete::le_u16, // Restart position
        nom::combinator::verify(nom::number::complete::le_u16, |e| (0..128).contains(e)), // Number of channels (OpenMPT allows a max of 127)
        nom::combinator::verify(nom::number::complete::le_u16, |e| (1..=256).contains(e)), // Number of patterns
        nom::combinator::verify(nom::number::complete::le_u16, |e| (0..=128).contains(e)), // Number of instruments
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
