use std::rc::Rc;

use crate::{
    frequency::{FrequencyCalculator, XmFrequencyType}, header::XmHeader, instrument::{XmInstrumentHeader, XmSample}, note::XmNote, XmModule, XmPattern
};

pub struct XmInstrumentRef(Rc<XmInstrumentHeader>, Vec<Rc<XmSample>>);

#[derive(Clone)]
pub struct XmInstrumentState {
    sample: Rc<XmSample>,
    sample_position: f32,

    step: f32,
    ping: bool,
}

pub struct XmStepSpec {
    pub note: crate::note::XmNote,
    pub sample_rate: u32,
    pub frequency_type: XmFrequencyType,
    pub finetune: i8,
}

#[derive(Clone)]
pub struct XmChannelContext {
    /// if it is `None`, no instrument is being executed/played
    /// everytime this is `Some(_)` it will get read and played
    instrument_state: Option<XmInstrumentState>,

    volume: f32,
    panning: f32,
}

impl<'a> Default for XmChannelContext {
    fn default() -> Self {
        Self {
            instrument_state: None,
            volume: 1.0,
            panning: 0.5,
        }
    }
}

pub struct XmPlaybackContext {
    module_header: XmHeader,
    instruments: Vec<XmInstrumentRef>,
    patterns: Vec<Rc<XmPattern>>,
    order_table: Vec<Rc<XmPattern>>,

    sample_rate: u32,

    tempo: u16,
    bpm: u16,
    volume: f32,

    current_order: usize,
    current_row: usize,
    current_tick: u32,
    left_samples_in_tick: f32,

    jump_dest: Option<u8>,
    jump_row: Option<u8>,

    extra_ticks: u16,

    // if a channel is None, then it is muted
    channels: Vec<XmChannelContext>,
}

impl XmInstrumentRef {
    pub fn header(&self) -> Rc<XmInstrumentHeader> {
        self.0.clone()
    }

    pub fn sample_for_note(&self, note: crate::note::XmNote) -> Option<Rc<XmSample>> {
        let Some(ref sample_opts) = self.0.sample_opts else {
            return None;
        };

        let note_index = note.index();

        let Some(sample_index) = sample_opts
            .sample_keymap_assignments
            .get(note_index as usize)
            .copied()
        else {
            return None;
        };

        self.1.get(sample_index as usize).cloned()
    }
}

impl XmInstrumentState {
    pub fn new(
        step_spec: &XmStepSpec,
        sample: Rc<XmSample>,
    ) -> Self {
        let sample_finetune = sample.header().finetune;
        let sample_relative_note = sample.header().relative_note_num;

        Self {
            sample,
            sample_position: 0.0,
            step: Self::calc_step(step_spec, sample_relative_note, sample_finetune),
            ping: true,
        }
    }

    fn calc_step(step_spec: &XmStepSpec, sample_relative_note: XmNote, sample_finetune: i8) -> f32 {
        let add_cents = |orig_freq: f32, semitones: f32| orig_freq * (2f32).powf(semitones / 12.);
        let finetune = ((sample_finetune as f32) / 128.) + 
                    ((step_spec.finetune as f32) / 128.);
        // let sample_relative_note = ((sample_relative_note.index() as i16) - 128);
        // let note = (step_spec.note.index() as i16) + sample_relative_note as i16;

        match step_spec.frequency_type {
            XmFrequencyType::Linear => {
                add_cents(
                    crate::frequency::Linear::frequency(crate::frequency::Linear::period(
                        step_spec.note,
                    ))
                , finetune + 6.) / step_spec.sample_rate as f32
            }
            XmFrequencyType::Amiga => todo!(),
        }
    }

    pub fn set_step(&mut self, step_spec: &XmStepSpec) {
        self.step = Self::calc_step(step_spec, self.sample.header().relative_note_num, self.sample.header().finetune);
    }

    pub fn advance(&mut self) -> bool {
        if self.sample.data().len() == 0 {
            return true;
        }

        match self.sample.header().kind.loop_type() {
            crate::instrument::XmSampleLoopType::NoLoop
            | crate::instrument::XmSampleLoopType::Unknown => {
                if self.sample_position as usize >= self.sample.data().len() {
                    // change instrument to None since we're done executing/playing it
                    // and its type is of `NoLoop`
                    true
                } else {
                    self.sample_position += self.step;
                    false
                }
            }
            crate::instrument::XmSampleLoopType::ForwardLoop => {
                self.sample_position += self.step;

                let loop_end = self.sample.header().loop_start + self.sample.header().loop_length;
                if self.sample_position >= loop_end as f32 {
                    self.sample_position = self.sample.header().loop_start as f32;
                }

                false
            }
            crate::instrument::XmSampleLoopType::BidirectionalLoop => {
                if self.ping {
                    self.sample_position += self.step;
                } else {
                    self.sample_position -= self.step;
                };

                let loop_end = self.sample.header().loop_start + self.sample.header().loop_length;
                if self.ping {
                    if self.sample_position >= loop_end as f32 {
                        self.ping = false;
                        self.sample_position = loop_end as f32;
                    }
                } else {
                    if self.sample_position <= self.sample.header().loop_start as f32 {
                        self.ping = true;
                        self.sample_position = self.sample.header().loop_start as f32;
                    }
                }

                false
            }
        }
    }

    pub fn sample(&self) -> f32 {
        if self.sample.data().len() == 0 {
            // nothing to generate since there is no sample
            return 0.0;
        }

        let sample = |reversed| {
            self.sample.data().get_interpolated(
                self.sample_position,
                reversed,
                crate::instrument::XmResamplingType::LinearInterpolation,
            )
        };

        let sample = match self.sample.header().kind.loop_type() {
            crate::instrument::XmSampleLoopType::NoLoop
            | crate::instrument::XmSampleLoopType::ForwardLoop
            // TODO: do something different for unknown type
            | crate::instrument::XmSampleLoopType::Unknown => sample(false),
            crate::instrument::XmSampleLoopType::BidirectionalLoop => {
                if self.ping {
                    sample(false)
                } else {
                    sample(true)
                }
            }
        };

        match sample {
            Some(v) => v,
            None => 0.0,
        }
    }
}

impl XmChannelContext {
    pub fn set_instrument(
        &mut self,
        step_spec: &XmStepSpec,
        sample: Rc<XmSample>,
    ) {
        self.instrument_state.replace(XmInstrumentState::new(step_spec, sample));
    }

    fn volume(sample: f32, volume: f32) -> f32 {
        sample * volume
    }

    fn pan(sample: f32, pan_ratio: f32) -> (f32, f32) {
        let pan_ratio = pan_ratio.clamp(0., 1.);

        let left_vol = (1.0 - pan_ratio).sqrt();
        let right_vol = pan_ratio.sqrt();

        (sample * left_vol, sample * right_vol)
    }

    pub fn advance(&mut self) {
        let is_instrument_done = if let Some(ref mut instr_state) = self.instrument_state {
            instr_state.advance()
        } else {
            false
        };

        if is_instrument_done {
            self.instrument_state.take();
        }
    }

    pub fn sample(&self) -> (f32, f32) {
        if let Some(ref instr_state) = self.instrument_state {
            Self::pan(
                Self::volume(instr_state.sample(), self.volume),
                self.panning,
            )
        } else {
            (0.0, 0.0)
        }
    }
}

impl XmPlaybackContext {
    // TODO: convert Option<Self> to an error type for XmPlaybackContext
    pub fn new(module: XmModule, sample_rate: u32) -> Option<Self> {
        let module_header = module.header;
        let instruments = module.instruments
            .into_iter()
            .map(|e|
                XmInstrumentRef(Rc::new(e.0), e.1.into_iter().map(|s| Rc::new(s)).collect::<Vec<_>>()))
            .collect::<Vec<_>>();
        let patterns = module.patterns.into_iter().map(|e| Rc::new(e)).collect::<Vec<_>>();
        let Some(order_table) = module.pattern_order_table
            .into_iter()
            .map(|e| patterns.get(e as usize).cloned())
            .collect::<Option<Vec<_>>>() else {
                return None;
            };

        let tempo = module_header.default_tempo;
        let bpm = module_header.default_bpm;
        let channels_num = module_header.channels_num;

        Some(Self {
            module_header,
            instruments,
            patterns,
            order_table,

            sample_rate,

            tempo,
            bpm,
            volume: 1.0,

            current_order: 0,
            current_row: 0,
            current_tick: 0,
            left_samples_in_tick: Self::samples_in_tick(sample_rate, bpm),

            jump_dest: None,
            jump_row: None,

            extra_ticks: 0,

            channels: vec![XmChannelContext::default(); channels_num.into()],
        })
    }

    fn samples_in_tick(sample_rate: u32, bpm: u16) -> f32 {
        // FT2 manual says number of ticks / second = BPM * 0.4
        sample_rate as f32 / bpm as f32 * 0.4
    }

    fn advance_row(&mut self) {
        // TODO: process pattern effects here
        // ...

        let Some(pattern) = self.order_table.get(self.current_order) else {
            return;
        };

        let Some(pattern_row) = pattern.1.get(self.current_row) else {
            return;
        };

        for (slot, channel) in pattern_row.iter().zip(self.channels.iter_mut()) {
            let instrument = {
                if let Some(instrument_index) = slot.instrument_index {
                    if let Some(instrument) = self.instruments.get(instrument_index as usize)
                    {
                        instrument
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            };

            let Some(note) = slot.signal.note() else {
                continue;
            };

            let Some(sample) = instrument.sample_for_note(note) else {
                continue;
            };

            let step_spec = XmStepSpec {
                note,
                sample_rate: self.sample_rate,
                // TODO: i have no idea what i am doing right now, fix this later
                frequency_type: XmFrequencyType::Linear,
                finetune: sample.header().finetune,
            };
            channel.set_instrument(&step_spec, sample);
        }
    }

    fn advance_tick(&mut self) {
        if self.current_tick == 0 {
            self.advance_row()
        }

        // TODO: effect applying goes here
        // ...

        self.current_tick += 1;

        if self.current_tick >= (self.tempo + self.extra_ticks) as u32 {
            self.current_tick = 0;
            self.extra_ticks = 0;
        }

        self.left_samples_in_tick += Self::samples_in_tick(self.sample_rate, self.bpm);

        todo!();
    }

    pub fn advance(&mut self) {
        if self.left_samples_in_tick <= 0.0 {
            self.advance_tick();
        }

        // for channel in self.channels.iter_mut() {
        //     channel.advance();
        // }

        // self.left_samples_in_tick -= 1.0;
    }

    pub fn sample(&self) -> (f32, f32) {
        let mut out_left = 0.0f32;
        let mut out_right = 0.0f32;

        for (i, channel) in self.channels.iter().enumerate() {
            let ch_sample = channel.sample();
        }

        todo!();
    }
}
