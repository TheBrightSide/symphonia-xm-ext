use log::warn;

use crate::{
    instrument::XmInstrumentHeader, pattern::XmPatternSlot, XmModule, XmPattern, XmSample,
};

#[derive(Clone)]
pub struct XmInstrumentState<'a> {
    instrument: &'a XmInstrumentHeader,
    sample: &'a XmSample,
    sample_position: f32,

    period: f32,
    frequency: f32,
    step: f32,
    ping: bool,
}

#[derive(Clone)]
pub struct XmChannelContext<'a> {
    /// this property is in Hz
    fine_tune: f32,

    /// if it is `None`, no instrument is being executed/played
    /// everytime this is `Some(_)` it will get read and played
    instrument_state: Option<XmInstrumentState<'a>>,

    pattern_slot_state: Option<XmPatternSlot>,
    volume: f32,
    panning: f32,
}

impl<'a> Default for XmChannelContext<'a> {
    fn default() -> Self {
        Self {
            fine_tune: 0.0,
            instrument_state: None,
            pattern_slot_state: None,
            volume: 1.0,
            panning: 0.5,
        }
    }
}

impl<'a> XmInstrumentState<'a> {
    fn advance(&mut self) -> bool {
        if self.sample.1.len() == 0 {
            return true;
        }

        match self.sample.0.kind.loop_type() {
            crate::instrument::XmSampleLoopType::NoLoop
            | crate::instrument::XmSampleLoopType::Unknown => {
                self.sample_position += self.step;

                if self.sample_position as usize >= self.sample.1.len() {
                    // change instrument to None since we're done executing/playing it
                    // and its type of of `NoLoop`
                    true
                } else {
                    false
                }
            }
            crate::instrument::XmSampleLoopType::ForwardLoop => {
                self.sample_position += self.step;

                let loop_end = self.sample.0.loop_start + self.sample.0.loop_length;
                if self.sample_position >= loop_end as f32 {
                    self.sample_position = self.sample.0.loop_start as f32;
                }

                false
            }
            crate::instrument::XmSampleLoopType::BidirectionalLoop => {
                if self.ping {
                    self.sample_position += self.step;
                } else {
                    self.sample_position -= self.step;
                };

                let loop_end = self.sample.0.loop_start + self.sample.0.loop_length;
                if self.ping {
                    if self.sample_position >= loop_end as f32 {
                        self.ping = false;
                        self.sample_position = loop_end as f32;
                    }
                } else {
                    if self.sample_position <= self.sample.0.loop_start as f32 {
                        self.ping = true;
                        self.sample_position = self.sample.0.loop_start as f32;
                    }
                }

                false
            }
        }
    }

    fn sample(&self) -> f32 {
        if self.sample.1.len() == 0 {
            // nothing to generate since there is no sample
            return 0.0;
        }

        // TODO: change resampling type argument
        let sample = || {
            self.sample.1.get_interpolated(
                self.sample_position,
                false,
                crate::instrument::XmResamplingType::LinearInterpolation,
            )
        };

        let reversed_sample = || {
            self.sample.1.get_interpolated(
                self.sample_position,
                true,
                crate::instrument::XmResamplingType::LinearInterpolation,
            )
        };

        let sample = match self.sample.0.kind.loop_type() {
            crate::instrument::XmSampleLoopType::NoLoop
            | crate::instrument::XmSampleLoopType::ForwardLoop
            // TODO: do something different for unknown type
            | crate::instrument::XmSampleLoopType::Unknown => sample(),
            crate::instrument::XmSampleLoopType::BidirectionalLoop => {
                if self.ping {
                    sample()
                } else {
                    reversed_sample()
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
    fn advance(&mut self) {
        todo!();
    }

    fn sample(&self) -> f32 {
        todo!();
    }
}

pub struct XmPlaybackContext<'a> {
    module: XmModule,
    sample_rate: u32,

    tempo: u16,
    bpm: u16,

    current_order: u32,
    current_row: u32,
    current_tick: u32,
    left_samples_in_tick: f32,

    jump_dest: Option<u8>,
    jump_row: Option<u8>,

    extra_ticks: u16,

    // if a channel is None, then it is muted
    channels: Vec<Option<XmChannelContext<'a>>>,
}

impl<'a> XmPlaybackContext<'a> {
    pub fn new(module: XmModule, sample_rate: u32) -> Self {
        Self {
            sample_rate,
            tempo: module.header.default_tempo,
            bpm: module.header.default_bpm,

            current_order: 0,
            current_row: 0,
            current_tick: 0,
            left_samples_in_tick: Self::samples_in_tick(sample_rate, module.header.default_bpm),

            jump_dest: None,
            jump_row: None,

            extra_ticks: 0,

            channels: vec![Some(XmChannelContext::default()); module.header.channels_num.into()],

            module,
        }
    }

    fn samples_in_tick(sample_rate: u32, bpm: u16) -> f32 {
        sample_rate as f32 / bpm as f32 * 0.4
    }

    fn volume(sample: f32, volume: f32) -> f32 {
        sample * volume
    }

    fn pan(sample: f32, pan_ratio: f32) -> (f32, f32) {
        let left_vol = (1.0 - pan_ratio).sqrt();
        let right_vol = pan_ratio.sqrt();

        (sample * left_vol, sample * right_vol)
    }

    fn tick(&mut self) {
        // FT2 manual says number of ticks / second = BPM * 0.4
        self.left_samples_in_tick += Self::samples_in_tick(self.sample_rate, self.bpm);

        todo!();
    }

    fn advance(&mut self) {
        if self.left_samples_in_tick <= 0.0 {
            self.tick();
        }

        self.left_samples_in_tick -= 1.0;
    }

    fn sample(&self) -> (f32, f32) {
        let mut out_left = 0.0f32;
        let mut out_right = 0.0f32;

        for (i, channel) in self.channels.iter().enumerate() {
            let Some(channel) = channel else { continue };

            let ch_sample = channel.sample();

        }

        todo!();
    }
}
