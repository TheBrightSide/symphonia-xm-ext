use std::cell::RefCell;

use log::warn;

use crate::{
    frequency::{FrequencyCalculator, XmFrequencyType},
    instrument::{XmInstrumentHeader, XmSample},
    XmModule,
};

#[derive(Clone)]
pub struct XmInstrumentState<'a> {
    instrument: &'a XmInstrumentHeader,
    sample: &'a XmSample,
    sample_position: f32,

    step: f32,
    ping: bool,
}

pub struct XmStepSpec {
    note: crate::note::XmNote,
    sample_rate: u32,
    frequency_type: XmFrequencyType,
    finetune: i8,
}

#[derive(Clone)]
pub struct XmChannelContext<'a> {
    /// if it is `None`, no instrument is being executed/played
    /// everytime this is `Some(_)` it will get read and played
    instrument_state: RefCell<Option<XmInstrumentState<'a>>>,

    volume: f32,
    panning: f32,
}

impl<'a> Default for XmChannelContext<'a> {
    fn default() -> Self {
        Self {
            instrument_state: RefCell::new(None),
            volume: 1.0,
            panning: 0.5,
        }
    }
}

pub struct XmPlaybackContext<'a> {
    module: XmModule,
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
    channels: Vec<XmChannelContext<'a>>,
}

impl<'a> XmInstrumentState<'a> {
    fn new(
        step_spec: &XmStepSpec,
        instrument: &'a XmInstrumentHeader,
        sample: &'a XmSample,
    ) -> Self {
        Self {
            instrument,
            sample,
            sample_position: 0.0,
            step: Self::calc_step(step_spec),
            ping: true,
        }
    }

    fn calc_step(step_spec: &XmStepSpec) -> f32 {
        match step_spec.frequency_type {
            XmFrequencyType::Linear => {
                crate::frequency::Linear::frequency(crate::frequency::Linear::period(
                    step_spec.note,
                )) / step_spec.sample_rate as f32
            }
            XmFrequencyType::Amiga => todo!(),
        }
    }

    fn set_step(&mut self, step_spec: &XmStepSpec) {
        self.step = Self::calc_step(step_spec);
    }

    fn advance(&mut self) -> bool {
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

    fn sample(&self) -> f32 {
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

impl<'a> XmChannelContext<'a> {
    fn set_instrument(
        &self,
        step_spec: &XmStepSpec,
        instrument: &'a XmInstrumentHeader,
        sample: &'a XmSample,
    ) {
        self.instrument_state
            .borrow_mut()
            .replace(XmInstrumentState::<'a>::new(step_spec, instrument, sample));
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

    fn advance(&mut self) {
        let mut instrument_state = self.instrument_state.borrow_mut();
        let is_instrument_done = if let Some(ref mut instr_state) = *instrument_state {
            instr_state.advance()
        } else {
            false
        };

        if is_instrument_done {
            self.instrument_state.borrow_mut().take();
        }
    }

    fn sample(&self) -> (f32, f32) {
        let instrument_state = self.instrument_state.borrow();
        if let Some(ref instr_state) = *instrument_state {
            Self::pan(
                Self::volume(instr_state.sample(), self.volume),
                self.panning,
            )
        } else {
            (0.0, 0.0)
        }
    }
}

impl<'a> XmPlaybackContext<'a> {
    pub fn new(module: XmModule, sample_rate: u32) -> Self {
        Self {
            sample_rate,

            tempo: module.header.default_tempo,
            bpm: module.header.default_bpm,
            volume: 1.0,

            current_order: 0,
            current_row: 0,
            current_tick: 0,
            left_samples_in_tick: Self::samples_in_tick(sample_rate, module.header.default_bpm),

            jump_dest: None,
            jump_row: None,

            extra_ticks: 0,

            channels: vec![XmChannelContext::default(); module.header.channels_num.into()],

            module,
        }
    }

    fn samples_in_tick(sample_rate: u32, bpm: u16) -> f32 {
        // FT2 manual says number of ticks / second = BPM * 0.4
        sample_rate as f32 / bpm as f32 * 0.4
    }

    fn advance_row(&'a self) {
        // TODO: process pattern effects here
        // ...

        let Some(pattern_index) = self
            .module
            .pattern_order_table
            .get(self.current_order)
            .cloned()
        else {
            return;
        };

        let Some(pattern) = self.module.patterns.get(pattern_index as usize) else {
            return;
        };

        let Some(pattern_row) = pattern.1.get(self.current_row) else {
            return;
        };

        for (slot, channel) in pattern_row.iter().zip(self.channels.iter()) {
            let instrument = {
                if let Some(instrument_index) = slot.instrument_index {
                    if let Some(instrument) = self.module.instruments.get(instrument_index as usize)
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
            channel.set_instrument(&step_spec, instrument.header(), sample);
        }
    }

    fn advance_tick(&'a mut self) {
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

    pub fn advance(&'a mut self) {
        if self.left_samples_in_tick <= 0.0 {
            self.advance_tick();
        }

        // for channel in self.channels.iter_mut() {
        //     channel.advance();
        // }

        // self.left_samples_in_tick -= 1.0;
    }

    fn sample(&self) -> (f32, f32) {
        let mut out_left = 0.0f32;
        let mut out_right = 0.0f32;

        for (i, channel) in self.channels.iter().enumerate() {
            let ch_sample = channel.sample();
        }

        todo!();
    }
}
