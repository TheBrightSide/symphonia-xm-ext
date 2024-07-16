use nom::{error::ParseError, IResult};

#[repr(u8)]
pub enum XmNoteRaw {
    NoNote = 0,

    C1,
    CS1,
    D1,
    DS1,
    E1,
    F1,
    FS1,
    G1,
    GS1,
    A1,
    AS1,
    B1,
    
    C2,
    CS2,
    D2,
    DS2,
    E2,
    F2,
    FS2,
    G2,
    GS2,
    A2,
    AS2,
    B2,
    
    C3,
    CS3,
    D3,
    DS3,
    E3,
    F3,
    FS3,
    G3,
    GS3,
    A3,
    AS3,
    B3,
    
    C4,
    CS4,
    D4,
    DS4,
    E4,
    F4,
    FS4,
    G4,
    GS4,
    A4,
    AS4,
    B4,
    
    C5,
    CS5,
    D5,
    DS5,
    E5,
    F5,
    FS5,
    G5,
    GS5,
    A5,
    AS5,
    B5,
    
    C6,
    CS6,
    D6,
    DS6,
    E6,
    F6,
    FS6,
    G6,
    GS6,
    A6,
    AS6,
    B6,
    
    C7,
    CS7,
    D7,
    DS7,
    E7,
    F7,
    FS7,
    G7,
    GS7,
    A7,
    AS7,
    B7,
    
    C8,
    CS8,
    D8,
    DS8,
    E8,
    F8,
    FS8,
    G8,
    GS8,
    A8,
    AS8,
    B8,

    NoteOff
}

pub const XM_TONE_COUNT: u8 = 12;
pub const XM_MAX_OCTAVE: u8 = 8;
pub const XM_NO_NOTE: u8 = XmNoteRaw::NoNote as u8;
pub const XM_NOTE_OFF: u8 = XmNoteRaw::NoteOff as u8;

pub enum XmTone {
    C,
    CS,
    D,
    DS,
    E,
    F,
    FS,
    G,
    GS,
    A,
    AS,
    B,
}

impl std::fmt::Display for XmTone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmTone::C => write!(f, "C-"),
            XmTone::CS => write!(f, "C#"),
            XmTone::D => write!(f, "D-"),
            XmTone::DS => write!(f, "D#"),
            XmTone::E => write!(f, "E-"),
            XmTone::F => write!(f, "F-"),
            XmTone::FS => write!(f, "F#"),
            XmTone::G => write!(f, "G-"),
            XmTone::GS => write!(f, "G#"),
            XmTone::A => write!(f, "A-"),
            XmTone::AS => write!(f, "A#"),
            XmTone::B => write!(f, "B-"),
        }
    }
}

pub enum XmNote {
    Note {
        tone: XmTone,
        octave: u8
    },
    NoNote,
    NoteOff,
}

impl std::fmt::Display for XmNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoNote => write!(f, "..."),
            Self::NoteOff => write!(f, "== "),
            Self::Note { tone, octave } => write!(f, "{}{}", tone, octave)
        }
    }
}

pub fn parse_xm_note<'a>(input: &'a [u8]) -> IResult<&'a [u8], XmNote> {
    let (input, value) = nom::number::complete::u8(input)?;

    match value {
        XM_NOTE_OFF => return Ok((input, XmNote::NoteOff)),
        XM_NO_NOTE => return Ok((input, XmNote::NoNote)),
        _ => {}
    }

    // we subtract 1 so we bring it to 0
    let value = value - 1;
    let octave = value / XM_TONE_COUNT;

    if octave > XM_MAX_OCTAVE {
        return Err(nom::Err::Error(nom::error::Error::from_error_kind(input, nom::error::ErrorKind::Verify)))
    }       

    let tone_raw = value as u16 - (octave as u16 * XM_TONE_COUNT as u16);
    let tone = match tone_raw {
        0 => XmTone::C,
        1 => XmTone::CS,
        2 => XmTone::D,
        3 => XmTone::DS,
        4 => XmTone::E,
        5 => XmTone::F,
        6 => XmTone::FS,
        7 => XmTone::G,
        8 => XmTone::GS,
        9 => XmTone::A,
        10 => XmTone::AS,
        11 => XmTone::B,
        _ => return Err(nom::Err::Error(nom::error::Error::from_error_kind(input, nom::error::ErrorKind::Verify)))
    };

    Ok((input, XmNote::Note { tone, octave: octave + 1 }))
}
