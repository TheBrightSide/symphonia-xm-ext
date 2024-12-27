use crate::note::{XmNote, XmSignal, XmTone, XM_MAX_OCTAVE, XM_TONE_COUNT};

fn note_to_raw(note: XmNote) -> u8 {
    let XmNote { tone, octave } = note;

    let tone_raw: u8 = match tone {
        XmTone::C => 1,
        XmTone::CS => 2,
        XmTone::D => 3,
        XmTone::DS => 4,
        XmTone::E => 5,
        XmTone::F => 6,
        XmTone::FS => 7,
        XmTone::G => 8,
        XmTone::GS => 9,
        XmTone::A => 10,
        XmTone::AS => 11,
        XmTone::B => 12,
    };

    let note = tone_raw + (octave.clamp(1, XM_MAX_OCTAVE) - 1) * XM_TONE_COUNT;

    note
}

pub enum XmFrequencyType {
    Amiga,
    Linear,
}

pub trait FrequencyCalculator {
    fn period(note: XmNote) -> f32;
    fn frequency(period: f32) -> f32;
}

pub struct Amiga;

pub struct Linear;

// don't ask me about these numbers, look at libxm
impl FrequencyCalculator for Linear {
    fn period(note: XmNote) -> f32 {
        let note = note_to_raw(note) as f32;
        7680.0 - note * 64.0
    }

    fn frequency(period: f32) -> f32 {
        8363.0 * 2.0_f32.powf((4608.0 - period) / 768.0)
    }
}

// how do i even begin implementing amiga's
impl FrequencyCalculator for Amiga {
    fn period(note: XmNote) -> f32 {
        todo!();
    }

    fn frequency(period: f32) -> f32 {
        todo!();
    }
}
