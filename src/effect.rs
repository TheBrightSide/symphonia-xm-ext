use bitfield_struct::bitfield;
use nom::{combinator::cond, error::ParseError, sequence::tuple, IResult};

#[bitfield(u8, order = Lsb)]
pub struct DoubleU4 {
    #[bits(4)]
    pub x: u8,

    #[bits(4)]
    pub y: u8,
}

#[derive(Clone)]
pub enum XmEffect {
    Arpeggio(DoubleU4),                  // 0 0x00(xy)
    PortamentoUp(u8),                    // 1 0x01(xx)
    PortamentoDown(u8),                  // 2 0x02(xx)
    TonePortamento(u8),                  // 3 0x03(xx)
    Vibrato(DoubleU4),                   // 4 0x04(xy)
    VolumeSlideTonePortamento(DoubleU4), // 5 0x05(xy)
    VolumeSlideVibrato(DoubleU4),        // 6 0x06(xy)
    Tremolo(DoubleU4),                   // 7 0x07(xy)
    SetPanningFine(u8),                  // 8 0x08(xx)
    SampleOffset(u8),                    // 9 0x09(xx)
    VolumeSlide(DoubleU4),               // A 0x0A(xy)
    PositionJump(u8),                    // B 0x0B(xx)
    SetVolume(u8),                       // C 0x0C(xx)
    PatternBreak(u8),                    // D 0x0D(xx)
    FinePortamentoUp(u8),                // E 0x0E(1x)
    FinePortamentoDown(u8),              // E 0x0E(2x)
    GlissandoControl(u8),                // E 0x0E(3x)
    SetVibratoWaveform(u8),              // E 0x0E(4x)
    SetFinetune(u8),                     // E 0x0E(5x)
    PatternLoopStart,                    // E 0x0E(60)
    PatternLoop(u8),                     // E 0x0E(6x)
    SetTremoloWaveform(u8),              // E 0x0E(7x)
    SetPanning(u8),                      // E 0x0E(8x)
    Retrigger(u8),                       // E 0x0E(9x)
    FineVolumeSlideUp(u8),               // E 0x0E(Ax)
    FineVolumeSlideDown(u8),             // E 0x0E(Bx)
    NoteCut(u8),                         // E 0x0E(Cx)
    NoteDelay(u8),                       // E 0x0E(Dx)
    PatternDelay(u8),                    // E 0x0E(Ex)
    SetActiveMacro(u8),                  // E 0x0E(Fx) NOTE: ModPlug hack
    SetTempo(u8),                        // F 0x0F(xx)
    SetGlobalVolume(u8),                 // G 0x10(xx)
    GlobalVolumeSlide(DoubleU4),         // H 0x11(xy)
    KeyOff(u8),                          // K 0x14(xx)
    SetEnvelopePosition(u8),             // L 0x15(xx)
    PanningSlide(DoubleU4),              // P 0x19(xy)
    RetriggerWithVolume(DoubleU4),       // R 0x1B(xy)
    Tremor(DoubleU4),                    // T 0x1D(xy)
    ExtraFinePortamentoUp(u8),           // X 0x21(1x) NOTE: ModPlug hack
    ExtraFinePortamentoDown(u8),         // X 0x21(2x) NOTE: ModPlug hack
    SetPanbrelloWaveform(u8),            // X 0x21(5x) NOTE: ModPlug hack
    FinePatternDelay(u8),                // X 0x21(6x) NOTE: ModPlug hack
    SoundControl(u8),                    // X 0x21(9x) NOTE: ModPlug hack
    HighOffset(u8),                      // X 0x21(Ax) NOTE: ModPlug hack
    Panbrello(DoubleU4),                 // Y 0x22(xy) NOTE: ModPlug hack
    MidiMacro(u8),                       // Z 0x23(xx) NOTE: ModPlug hack
    SmoothMidiMacro(u8),                 // \ 0x24(xx) NOTE: ModPlug hack
}

#[derive(Clone)]
pub struct XmVolumeColumn(u8);

#[repr(u8)]
pub enum XmVolumeColumnCommand {
    SetVolume,       // 0x00..0x50 axx
    VolumeSlideUp,   // 0x60..0x6F bxx
    VolumeSlideDown, // 0x70..0x7F cxx
    FineVolumeDown,  // 0x80..0x8F dxx
    FineVolumeUp,    // 0x90..0x9F gxx
    VibratoSpeed,    // 0xA0..0xAF hxx
    VibratoDepth,    // 0xB0..0xBF lxx
    SetPanning,      // 0xC0..0xCF pxx
    PanSlideLeft,    // 0xD0..0xDF rxx
    PanSlideRight,   // 0xE0..0xEF uxx
    TonePortamento,  // 0xF0..0xFF vxx
    Unknown,
}

pub(crate) fn parse_volume_column(data: &[u8]) -> IResult<&[u8], XmVolumeColumn> {
    let (input, byte) = nom::number::complete::u8(data)?;
    let byte = XmVolumeColumn::from(byte);

    if let XmVolumeColumnCommand::Unknown = byte.command() {
        Err(nom::Err::Error(nom::error::Error::from_error_kind(
            data,
            nom::error::ErrorKind::Verify,
        )))
    } else {
        Ok((input, byte))
    }
}

pub(crate) fn parse_effect(
    effect_type_follows: bool,
    effect_parameter_follows: bool,
) -> impl FnMut(&[u8]) -> IResult<&[u8], Option<XmEffect>> {
    move |data| {
        let (input, (command, parameter)) = tuple((
            cond(effect_type_follows, nom::number::complete::u8),
            cond(effect_parameter_follows, nom::number::complete::u8),
        ))(data)?;

        let Some((command, parameter)) = (match (command, parameter) {
            (Some(c), Some(p)) => Some((c, p)),
            (Some(c), None) => Some((c, 0)),
            (None, Some(c)) => Some((0, c)),
            (None, None) => None,
        }) else {
            return Ok((input, None));
        };

        let high_ord_nibble_param = parameter >> 4;

        match (command, parameter, high_ord_nibble_param) {
            (0x00, a, _) => Ok((input, Some(XmEffect::Arpeggio(DoubleU4(a))))), // 0 0x00(xy)
            (0x01, a, _) => Ok((input, Some(XmEffect::PortamentoUp(a)))),       // 1 0x01(xx)
            (0x02, a, _) => Ok((input, Some(XmEffect::PortamentoDown(a)))),     // 2 0x02(xx)
            (0x03, a, _) => Ok((input, Some(XmEffect::TonePortamento(a)))),     // 3 0x03(xx)
            (0x04, a, _) => Ok((input, Some(XmEffect::Vibrato(DoubleU4(a))))),  // 4 0x04(xy)
            (0x05, a, _) => Ok((
                input,
                Some(XmEffect::VolumeSlideTonePortamento(DoubleU4(a))),
            )), // 5 0x05(xy)
            (0x06, a, _) => Ok((input, Some(XmEffect::VolumeSlideVibrato(DoubleU4(a))))), // 6 0x06(xy)
            (0x07, a, _) => Ok((input, Some(XmEffect::Tremolo(DoubleU4(a))))), // 7 0x07(xy)
            (0x08, a, _) => Ok((input, Some(XmEffect::SetPanningFine(a)))),    // 8 0x08(xx)
            (0x09, a, _) => Ok((input, Some(XmEffect::SampleOffset(a)))),      // 9 0x09(xx)
            (0x0A, a, _) => Ok((input, Some(XmEffect::VolumeSlide(DoubleU4(a))))), // A 0x0A(xy)
            (0x0B, a, _) => Ok((input, Some(XmEffect::PositionJump(a)))),      // B 0x0B(xx)
            (0x0C, a, _) => Ok((input, Some(XmEffect::SetVolume(a)))),         // C 0x0C(xx)
            (0x0D, a, _) => Ok((input, Some(XmEffect::PatternBreak(a)))),      // D 0x0D(xx)
            (0x0E, a, 0x1) => Ok((input, Some(XmEffect::FinePortamentoUp(a & 0b1111)))), // E 0x0E(1x)
            (0x0E, a, 0x2) => Ok((input, Some(XmEffect::FinePortamentoDown(a & 0b1111)))), // E 0x0E(2x)
            (0x0E, a, 0x3) => Ok((input, Some(XmEffect::GlissandoControl(a & 0b1111)))), // E 0x0E(3x)
            (0x0E, a, 0x4) => Ok((input, Some(XmEffect::SetVibratoWaveform(a & 0b1111)))), // E 0x0E(4x)
            (0x0E, a, 0x5) => Ok((input, Some(XmEffect::SetFinetune(a & 0b1111)))), // E 0x0E(5x)
            (0x0E, 0x60, _) => Ok((input, Some(XmEffect::PatternLoopStart))),       // E 0x0E(60)
            (0x0E, a, 0x6) => Ok((input, Some(XmEffect::PatternLoop(a & 0b1111)))), // E 0x0E(6x)
            (0x0E, a, 0x7) => Ok((input, Some(XmEffect::SetTremoloWaveform(a & 0b1111)))), // E 0x0E(7x)
            (0x0E, a, 0x8) => Ok((input, Some(XmEffect::SetPanning(a & 0b1111)))), // E 0x0E(8x)
            (0x0E, a, 0x9) => Ok((input, Some(XmEffect::Retrigger(a & 0b1111)))),  // E 0x0E(9x)
            (0x0E, a, 0xa) => Ok((input, Some(XmEffect::FineVolumeSlideUp(a & 0b1111)))), // E 0x0E(Ax)
            (0x0E, a, 0xb) => Ok((input, Some(XmEffect::FineVolumeSlideDown(a & 0b1111)))), // E 0x0E(Bx)
            (0x0E, a, 0xc) => Ok((input, Some(XmEffect::NoteCut(a & 0b1111)))), // E 0x0E(Cx)
            (0x0E, a, 0xd) => Ok((input, Some(XmEffect::NoteDelay(a & 0b1111)))), // E 0x0E(Dx)
            (0x0E, a, 0xe) => Ok((input, Some(XmEffect::PatternDelay(a & 0b1111)))), // E 0x0E(Ex)
            (0x0E, a, 0xf) => Ok((input, Some(XmEffect::SetActiveMacro(a & 0b1111)))), // E 0x0E(Fx) NOTE: ModPlug hack
            (0x0F, a, _) => Ok((input, Some(XmEffect::SetTempo(a)))),                  // F 0x0F(xx)
            (0x10, a, _) => Ok((input, Some(XmEffect::SetGlobalVolume(a)))),           // G 0x10(xx)
            (0x11, a, _) => Ok((input, Some(XmEffect::GlobalVolumeSlide(DoubleU4(a))))), // H 0x11(xy)
            (0x14, a, _) => Ok((input, Some(XmEffect::KeyOff(a)))), // K 0x14(xx)
            (0x15, a, _) => Ok((input, Some(XmEffect::SetEnvelopePosition(a)))), // L 0x15(xx)
            (0x19, a, _) => Ok((input, Some(XmEffect::PanningSlide(DoubleU4(a))))), // P 0x19(xy)
            (0x1B, a, _) => Ok((input, Some(XmEffect::RetriggerWithVolume(DoubleU4(a))))), // R 0x1B(xy)
            (0x1D, a, _) => Ok((input, Some(XmEffect::Tremor(DoubleU4(a))))), // T 0x1D(xy)
            (0x21, a, 0x1) => Ok((input, Some(XmEffect::ExtraFinePortamentoUp(a & 0b1111)))), // X 0x21(1x) NOTE: ModPlug hack
            (0x21, a, 0x2) => Ok((input, Some(XmEffect::ExtraFinePortamentoDown(a & 0b1111)))), // X 0x21(2x) NOTE: ModPlug hack
            (0x21, a, 0x5) => Ok((input, Some(XmEffect::SetPanbrelloWaveform(a & 0b1111)))), // X 0x21(5x) NOTE: ModPlug hack
            (0x21, a, 0x6) => Ok((input, Some(XmEffect::FinePatternDelay(a & 0b1111)))), // X 0x21(6x) NOTE: ModPlug hack
            (0x21, a, 0x9) => Ok((input, Some(XmEffect::SoundControl(a & 0b1111)))), // X 0x21(9x) NOTE: ModPlug hack
            (0x21, a, 0xa) => Ok((input, Some(XmEffect::HighOffset(a & 0b1111)))), // X 0x21(Ax) NOTE: ModPlug hack
            (0x22, a, _) => Ok((input, Some(XmEffect::Panbrello(DoubleU4(a))))), // Y 0x22(xy) NOTE: ModPlug hack
            (0x23, a, _) => Ok((input, Some(XmEffect::MidiMacro(a)))), // Z 0x23(xx) NOTE: ModPlug hack
            (0x24, a, _) => Ok((input, Some(XmEffect::SmoothMidiMacro(a)))), // \ 0x24(xx) NOTE: ModPlug hack
            (_, _a, _) => Err(nom::Err::Error(nom::error::Error::from_error_kind(
                data,
                nom::error::ErrorKind::Verify,
            ))),
        }
    }
}

impl std::fmt::Display for XmEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmEffect::Arpeggio(a) => write!(f, "0{:02X}", a.into_bits()),
            XmEffect::PortamentoUp(a) => write!(f, "1{:02X}", a),
            XmEffect::PortamentoDown(a) => write!(f, "2{:02X}", a),
            XmEffect::TonePortamento(a) => write!(f, "3{:02X}", a),
            XmEffect::Vibrato(a) => write!(f, "4{:02X}", a.into_bits()),
            XmEffect::VolumeSlideTonePortamento(a) => write!(f, "5{:02X}", a.into_bits()),
            XmEffect::VolumeSlideVibrato(a) => write!(f, "6{:02X}", a.into_bits()),
            XmEffect::Tremolo(a) => write!(f, "7{:02X}", a.into_bits()),
            XmEffect::SetPanningFine(a) => write!(f, "8{:02X}", a),
            XmEffect::SampleOffset(a) => write!(f, "9{:02X}", a),
            XmEffect::VolumeSlide(a) => write!(f, "A{:02X}", a.into_bits()),
            XmEffect::PositionJump(a) => write!(f, "B{:02X}", a),
            XmEffect::SetVolume(a) => write!(f, "C{:02X}", a),
            XmEffect::PatternBreak(a) => write!(f, "D{:02X}", a),
            XmEffect::FinePortamentoUp(a) => write!(f, "E1{:01X}", a),
            XmEffect::FinePortamentoDown(a) => write!(f, "E2{:01X}", a),
            XmEffect::GlissandoControl(a) => write!(f, "E3{:01X}", a),
            XmEffect::SetVibratoWaveform(a) => write!(f, "E4{:01X}", a),
            XmEffect::SetFinetune(a) => write!(f, "E5{:01X}", a),
            XmEffect::PatternLoopStart => write!(f, "E60"),
            XmEffect::PatternLoop(a) => write!(f, "E6{:01X}", a),
            XmEffect::SetTremoloWaveform(a) => write!(f, "E7{:01X}", a),
            XmEffect::SetPanning(a) => write!(f, "E8{:01X}", a),
            XmEffect::Retrigger(a) => write!(f, "E9{:01X}", a),
            XmEffect::FineVolumeSlideUp(a) => write!(f, "EA{:01X}", a),
            XmEffect::FineVolumeSlideDown(a) => write!(f, "EB{:01X}", a),
            XmEffect::NoteCut(a) => write!(f, "EC{:01X}", a),
            XmEffect::NoteDelay(a) => write!(f, "ED{:01X}", a),
            XmEffect::PatternDelay(a) => write!(f, "EE{:01X}", a),
            XmEffect::SetActiveMacro(a) => write!(f, "EF{:01X}", a),
            XmEffect::SetTempo(a) => write!(f, "F{:02X}", a),
            XmEffect::SetGlobalVolume(a) => write!(f, "G{:02X}", a),
            XmEffect::GlobalVolumeSlide(a) => write!(f, "H{:02X}", a.into_bits()),
            XmEffect::KeyOff(a) => write!(f, "K{:02X}", a),
            XmEffect::SetEnvelopePosition(a) => write!(f, "L{:02X}", a),
            XmEffect::PanningSlide(a) => write!(f, "P{:02X}", a.into_bits()),
            XmEffect::RetriggerWithVolume(a) => write!(f, "R{:02X}", a.into_bits()),
            XmEffect::Tremor(a) => write!(f, "T{:02X}", a.into_bits()),
            XmEffect::ExtraFinePortamentoUp(a) => write!(f, "X1{:01X}", a),
            XmEffect::ExtraFinePortamentoDown(a) => write!(f, "X2{:01X}", a),
            XmEffect::SetPanbrelloWaveform(a) => write!(f, "X5{:01X}", a),
            XmEffect::FinePatternDelay(a) => write!(f, "X6{:01X}", a),
            XmEffect::SoundControl(a) => write!(f, "X9{:01X}", a),
            XmEffect::HighOffset(a) => write!(f, "XA{:01X}", a),
            XmEffect::Panbrello(a) => write!(f, "Y{:02X}", a.into_bits()),
            XmEffect::MidiMacro(a) => write!(f, "Z{:02X}", a),
            XmEffect::SmoothMidiMacro(a) => write!(f, "\\{:02X}", a),
        }
    }
}

impl XmVolumeColumnCommand {
    const fn into_bits(self) -> u8 {
        self as _
    }

    const fn from_bits(value: u8) -> Self {
        match value {
            (0x0..=0x5) => XmVolumeColumnCommand::SetVolume,
            0x6 => XmVolumeColumnCommand::VolumeSlideDown,
            0x7 => XmVolumeColumnCommand::VolumeSlideUp,
            0x8 => XmVolumeColumnCommand::FineVolumeDown,
            0x9 => XmVolumeColumnCommand::FineVolumeUp,
            0xa => XmVolumeColumnCommand::VibratoSpeed,
            0xb => XmVolumeColumnCommand::VibratoDepth,
            0xc => XmVolumeColumnCommand::SetPanning,
            0xd => XmVolumeColumnCommand::PanSlideLeft,
            0xe => XmVolumeColumnCommand::PanSlideRight,
            0xf => XmVolumeColumnCommand::TonePortamento,
            _ => XmVolumeColumnCommand::Unknown,
        }
    }
}

impl From<u8> for XmVolumeColumn {
    fn from(value: u8) -> Self {
        Self::new(value)
    }
}

impl XmVolumeColumn {
    pub fn new(v: u8) -> Self {
        Self(v)
    }

    pub fn argument(&self) -> u8 {
        let value = self.0 & 0b0000_1111;

        match self.command() {
            XmVolumeColumnCommand::SetVolume => (value) | ((self.command_raw() - 1) << 4),
            XmVolumeColumnCommand::VolumeSlideDown => value,
            XmVolumeColumnCommand::VolumeSlideUp => value,
            XmVolumeColumnCommand::FineVolumeDown => value,
            XmVolumeColumnCommand::FineVolumeUp => value,
            XmVolumeColumnCommand::VibratoSpeed => value,
            XmVolumeColumnCommand::VibratoDepth => value,
            XmVolumeColumnCommand::SetPanning => 2 + value * 4,
            XmVolumeColumnCommand::PanSlideLeft => value,
            XmVolumeColumnCommand::PanSlideRight => value,
            XmVolumeColumnCommand::TonePortamento => value,
            XmVolumeColumnCommand::Unknown => value,
        }
    }

    pub fn command(&self) -> XmVolumeColumnCommand {
        XmVolumeColumnCommand::from_bits(self.command_raw())
    }

    fn command_raw(&self) -> u8 {
        (self.0 & 0b1111_0000) >> 4
    }
}

impl std::fmt::Display for XmVolumeColumnCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmVolumeColumnCommand::SetVolume => write!(f, "v"),
            XmVolumeColumnCommand::VolumeSlideDown => write!(f, "d"),
            XmVolumeColumnCommand::VolumeSlideUp => write!(f, "c"),
            XmVolumeColumnCommand::FineVolumeDown => write!(f, "b"),
            XmVolumeColumnCommand::FineVolumeUp => write!(f, "a"),
            XmVolumeColumnCommand::VibratoSpeed => write!(f, "u"),
            XmVolumeColumnCommand::VibratoDepth => write!(f, "h"),
            XmVolumeColumnCommand::SetPanning => write!(f, "p"),
            XmVolumeColumnCommand::PanSlideLeft => write!(f, "l"),
            XmVolumeColumnCommand::PanSlideRight => write!(f, "r"),
            XmVolumeColumnCommand::TonePortamento => write!(f, "g"),
            XmVolumeColumnCommand::Unknown => write!(f, "_"),
        }
    }
}

impl std::fmt::Display for XmVolumeColumn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{:0>2}", self.command(), self.argument())
    }
}
