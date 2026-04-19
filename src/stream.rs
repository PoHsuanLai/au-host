#![cfg(target_os = "macos")]

use crate::error::Result;
use crate::ffi::{get_property, set_property};
use crate::handle::AuHandle;
use crate::types::*;

#[derive(Debug, Clone, Copy)]
pub struct ChannelLayout {
    pub inputs: u32,
    pub outputs: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct StreamConfig {
    pub sample_rate: f64,
    pub block_size: u32,
    pub channels: ChannelLayout,
}

impl StreamConfig {
    pub fn new(sample_rate: f64, block_size: u32, channels: ChannelLayout) -> Self {
        Self {
            sample_rate,
            block_size,
            channels,
        }
    }

    pub(crate) fn probe(handle: &AuHandle) -> ChannelLayout {
        let unit = handle.raw_unit();
        let outputs = unsafe {
            get_property::<AudioStreamBasicDescription>(
                unit,
                K_AUDIO_UNIT_PROPERTY_STREAM_FORMAT,
                K_AUDIO_UNIT_SCOPE_OUTPUT,
                0,
            )
        }
        .map(|asbd| asbd.channels_per_frame)
        .unwrap_or(2);

        let inputs = unsafe {
            get_property::<AudioStreamBasicDescription>(
                unit,
                K_AUDIO_UNIT_PROPERTY_STREAM_FORMAT,
                K_AUDIO_UNIT_SCOPE_INPUT,
                0,
            )
        }
        .map(|asbd| asbd.channels_per_frame)
        .unwrap_or(0);

        ChannelLayout { inputs, outputs }
    }

    pub(crate) fn apply(&self, handle: &AuHandle) -> Result<()> {
        let unit = handle.raw_unit();

        unsafe {
            set_property(
                unit,
                K_AUDIO_UNIT_PROPERTY_MAXIMUM_FRAMES_PER_SLICE,
                K_AUDIO_UNIT_SCOPE_GLOBAL,
                0,
                &self.block_size,
            )?;

            let out_asbd =
                AudioStreamBasicDescription::float32(self.sample_rate, self.channels.outputs.max(2));
            let _ = set_property(
                unit,
                K_AUDIO_UNIT_PROPERTY_STREAM_FORMAT,
                K_AUDIO_UNIT_SCOPE_OUTPUT,
                0,
                &out_asbd,
            );

            if self.channels.inputs > 0 {
                let in_asbd =
                    AudioStreamBasicDescription::float32(self.sample_rate, self.channels.inputs);
                let _ = set_property(
                    unit,
                    K_AUDIO_UNIT_PROPERTY_STREAM_FORMAT,
                    K_AUDIO_UNIT_SCOPE_INPUT,
                    0,
                    &in_asbd,
                );
            }
        }

        Ok(())
    }
}
