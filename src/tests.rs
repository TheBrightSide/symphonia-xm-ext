// use context::XmPlaybackContext;

use std::{fs::File, io::Write, rc::Rc};

use bytemuck::cast_slice;
use context::{XmChannelContext, XmInstrumentState, XmStepSpec};
use frequency::FrequencyCalculator;
use note::XmNote;

use super::*;

#[test]
fn test_parse_xm_header_first() {
    let (_input, format) = parse(include_bytes!("test_xms/test_w_mpt_ext_amen.xm")).unwrap();

    println!("{}", format.patterns.get(0).unwrap().1);

    let sample = Rc::new(format.instruments.get(2).unwrap().1.get(0).unwrap().clone());
    println!("{:#?}", sample.header());

    // let add_cents = |orig_freq: f32, semitones: f32| orig_freq * (2f32).powf(semitones / 12.);
    // println!("{}", add_cents(
    //     frequency::Linear::frequency(frequency::Linear::period(XmNote { octave: 6, tone: note::XmTone::G })),
    //     0.
    // ));

    let mut channel_context = XmInstrumentState::new(&XmStepSpec {
        note: XmNote { octave: 6, tone: note::XmTone::C },
        sample_rate: 44100,
        frequency_type: frequency::XmFrequencyType::Linear,
        finetune: 0
    }, sample);

    let mut pcm_file = File::create("./audio.raw").unwrap();
    let mut buf = vec![];
    loop {
        buf.push(channel_context.sample());
        if channel_context.advance() {
            break;
        }
    }

    pcm_file.write(cast_slice(buf.as_slice())).unwrap();
    pcm_file.flush().unwrap();
    // let mut context = XmPlaybackContext::new(format, 44100);
}
