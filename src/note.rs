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

    NoteOff,
}

pub const XM_TONE_COUNT: u8 = 12;
pub const XM_MAX_OCTAVE: u8 = 7;
pub const XM_NO_NOTE: u8 = XmNoteRaw::NoNote as u8;
pub const XM_NOTE_OFF: u8 = XmNoteRaw::NoteOff as u8;

pub const XM_SMALLEST_RELATIVE_NOTE: i8 = -48;
pub const XM_MAX_OCTAVE_RELATIVE_NOTE: u8 = 9;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
pub struct XmNote {
    pub tone: XmTone,
    pub octave: u8,
}

#[derive(Clone, Copy)]
pub enum XmSignal {
    Note(XmNote),
    NoNote,
    NoteOff,
}

impl XmNote {
    pub fn index(&self) -> u8 {
        self.octave * XM_TONE_COUNT + self.tone as u8
    }

    // pub fn from_relative_note_num(relative_note_num: i8) -> Self {
    //     return 
    // }
}

impl Default for XmSignal {
    fn default() -> Self {
        Self::NoNote
    }
}

impl XmSignal {
    pub fn is_note_off(&self) -> bool {
        matches!(self, Self::NoteOff)
    }

    pub fn is_no_note(&self) -> bool {
        matches!(self, Self::NoNote)
    }

    pub fn note(&self) -> Option<XmNote> {
        match self {
            Self::Note(note) => Some(*note),
            _ => None,
        }
    }
}

impl std::fmt::Display for XmSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoNote => write!(f, "..."),
            Self::NoteOff => write!(f, "== "),
            Self::Note(XmNote { tone, octave }) => write!(f, "{}{}", tone, octave),
        }
    }
}

pub fn parse_xm_note(input: u8, is_relative_note: bool) -> Option<XmNote> {
    let octave = input / XM_TONE_COUNT;

    if is_relative_note && octave > XM_MAX_OCTAVE_RELATIVE_NOTE {
        return None;
    } else if !is_relative_note && octave > XM_MAX_OCTAVE {
        return None;
    }

    let tone_raw = input as u16 - (octave as u16 * XM_TONE_COUNT as u16);
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
        _ => {
            return None;
        }
    };

    Some(XmNote { tone, octave: octave + 1 })
}

pub fn parse_xm_signal(input: &[u8]) -> IResult<&[u8], XmSignal> {
    let (input, value) = nom::number::complete::u8(input)?;

    match value {
        XM_NOTE_OFF => return Ok((input, XmSignal::NoteOff)),
        XM_NO_NOTE => return Ok((input, XmSignal::NoNote)),
        _ => {}
    }

    // we subtract 1, discarding the NoNote scenario, since we already checked for that
    let value = value - 1;

    match parse_xm_note(value, false) {
        Some(d) => Ok((input, XmSignal::Note(d))),
        _ => Err(nom::Err::Error(nom::error::Error::from_error_kind(
            input,
            nom::error::ErrorKind::Verify,
        ))),
    }
}

pub fn parse_relative_note_num(input: &[u8]) -> IResult<&[u8], XmNote> {
    let (input, value) = nom::number::complete::i8(input)?;
    let converted = ((value as i16) + XM_SMALLEST_RELATIVE_NOTE.abs() as i16) as u8;

    match parse_xm_note(converted, true) {
        None => Err(nom::Err::Error(nom::error::Error::from_error_kind(
            input,
            nom::error::ErrorKind::Verify,
        ))),
        Some(d) => Ok((input, d))
    }
}
