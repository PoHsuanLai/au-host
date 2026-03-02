//! Audio Unit type definitions and FFI bindings to AudioToolbox.
//!
//! This module defines the raw C types and constants needed for AUv2 hosting.
//! All types match Apple's AudioToolbox headers.

#![allow(non_upper_case_globals, non_camel_case_types, dead_code)]

use std::os::raw::c_void;

// ── Opaque pointer types ──

/// Opaque handle to an Audio Component (factory).
pub type AudioComponent = *mut c_void;

/// Opaque handle to an Audio Component Instance (instantiated AU).
/// This is the same as `AudioUnit` in Apple's headers.
pub type AudioComponentInstance = *mut c_void;

/// Alias: Apple headers define `AudioUnit` = `AudioComponentInstance`.
pub type AudioUnit = AudioComponentInstance;

// ── OSStatus ──

pub type OSStatus = i32;

pub const K_AUDIO_UNIT_ERR_INVALID_PROPERTY: OSStatus = -10879;
pub const K_AUDIO_UNIT_ERR_INVALID_PARAMETER: OSStatus = -10878;
pub const K_AUDIO_UNIT_ERR_INVALID_ELEMENT: OSStatus = -10877;
pub const K_AUDIO_UNIT_ERR_NO_CONNECTION: OSStatus = -10876;
pub const K_AUDIO_UNIT_ERR_FAILED_INITIALIZATION: OSStatus = -10875;
pub const K_AUDIO_UNIT_ERR_TOO_MANY_FRAMES_TO_PROCESS: OSStatus = -10874;
pub const K_AUDIO_UNIT_ERR_INVALID_FILE: OSStatus = -10871;
pub const K_AUDIO_UNIT_ERR_UNKNOWN_FILE_TYPE: OSStatus = -10870;
pub const K_AUDIO_UNIT_ERR_FILE_NOT_SPECIFIED: OSStatus = -10869;
pub const K_AUDIO_UNIT_ERR_FORMAT_NOT_SUPPORTED: OSStatus = -10868;
pub const K_AUDIO_UNIT_ERR_UNINITIALIZED: OSStatus = -10867;
pub const K_AUDIO_UNIT_ERR_INVALID_SCOPE: OSStatus = -10866;
pub const K_AUDIO_UNIT_ERR_PROPERTY_NOT_WRITABLE: OSStatus = -10865;
pub const K_AUDIO_UNIT_ERR_CANNOT_DO_IN_CURRENT_CONTEXT: OSStatus = -10863;
pub const K_AUDIO_UNIT_ERR_INVALID_PROPERTY_VALUE: OSStatus = -10851;
pub const K_AUDIO_UNIT_ERR_PROPERTY_NOT_IN_USE: OSStatus = -10850;
pub const K_AUDIO_UNIT_ERR_INITIALIZED: OSStatus = -10849;
pub const K_AUDIO_UNIT_ERR_INVALID_OFFLINE_RENDER: OSStatus = -10848;
pub const K_AUDIO_UNIT_ERR_UNAUTHORIZED: OSStatus = -10847;

pub const NO_ERR: OSStatus = 0;

// ── AudioComponentDescription ──

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioComponentDescription {
    pub component_type: u32,
    pub component_sub_type: u32,
    pub component_manufacturer: u32,
    pub component_flags: u32,
    pub component_flags_mask: u32,
}

// AU component types (four-char codes as u32)
pub const K_AUDIO_UNIT_TYPE_OUTPUT: u32 = u32::from_be_bytes(*b"auou");
pub const K_AUDIO_UNIT_TYPE_MUSIC_DEVICE: u32 = u32::from_be_bytes(*b"aumu");
pub const K_AUDIO_UNIT_TYPE_MUSIC_EFFECT: u32 = u32::from_be_bytes(*b"aumf");
pub const K_AUDIO_UNIT_TYPE_FORMAT_CONVERTER: u32 = u32::from_be_bytes(*b"aufc");
pub const K_AUDIO_UNIT_TYPE_EFFECT: u32 = u32::from_be_bytes(*b"aufx");
pub const K_AUDIO_UNIT_TYPE_MIXER: u32 = u32::from_be_bytes(*b"aumx");
pub const K_AUDIO_UNIT_TYPE_PANNER: u32 = u32::from_be_bytes(*b"aupn");
pub const K_AUDIO_UNIT_TYPE_GENERATOR: u32 = u32::from_be_bytes(*b"augn");
pub const K_AUDIO_UNIT_TYPE_OFFLINE_EFFECT: u32 = u32::from_be_bytes(*b"auol");
pub const K_AUDIO_UNIT_TYPE_MIDI_PROCESSOR: u32 = u32::from_be_bytes(*b"aumi");

// ── AudioStreamBasicDescription ──

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AudioStreamBasicDescription {
    pub sample_rate: f64,
    pub format_id: u32,
    pub format_flags: u32,
    pub bytes_per_packet: u32,
    pub frames_per_packet: u32,
    pub bytes_per_frame: u32,
    pub channels_per_frame: u32,
    pub bits_per_channel: u32,
    pub reserved: u32,
}

impl Default for AudioStreamBasicDescription {
    fn default() -> Self {
        Self {
            sample_rate: 44100.0,
            format_id: K_AUDIO_FORMAT_LINEAR_PCM,
            format_flags: K_AUDIO_FORMAT_FLAG_IS_FLOAT
                | K_AUDIO_FORMAT_FLAG_IS_NON_INTERLEAVED
                | K_AUDIO_FORMAT_FLAG_IS_PACKED,
            bytes_per_packet: 4,
            frames_per_packet: 1,
            bytes_per_frame: 4,
            channels_per_frame: 2,
            bits_per_channel: 32,
            reserved: 0,
        }
    }
}

impl AudioStreamBasicDescription {
    /// Create a standard 32-bit float non-interleaved ASBD.
    pub fn float32(sample_rate: f64, channels: u32) -> Self {
        Self {
            sample_rate,
            format_id: K_AUDIO_FORMAT_LINEAR_PCM,
            format_flags: K_AUDIO_FORMAT_FLAG_IS_FLOAT
                | K_AUDIO_FORMAT_FLAG_IS_NON_INTERLEAVED
                | K_AUDIO_FORMAT_FLAG_IS_PACKED,
            bytes_per_packet: 4,
            frames_per_packet: 1,
            bytes_per_frame: 4,
            channels_per_frame: channels,
            bits_per_channel: 32,
            reserved: 0,
        }
    }
}

// Audio format IDs
pub const K_AUDIO_FORMAT_LINEAR_PCM: u32 = u32::from_be_bytes(*b"lpcm");

// Audio format flags
pub const K_AUDIO_FORMAT_FLAG_IS_FLOAT: u32 = 1 << 0;
pub const K_AUDIO_FORMAT_FLAG_IS_BIG_ENDIAN: u32 = 1 << 1;
pub const K_AUDIO_FORMAT_FLAG_IS_SIGNED_INTEGER: u32 = 1 << 2;
pub const K_AUDIO_FORMAT_FLAG_IS_PACKED: u32 = 1 << 3;
pub const K_AUDIO_FORMAT_FLAG_IS_ALIGNED_HIGH: u32 = 1 << 4;
pub const K_AUDIO_FORMAT_FLAG_IS_NON_INTERLEAVED: u32 = 1 << 5;
pub const K_AUDIO_FORMAT_FLAGS_NATIVE_FLOAT_PACKED: u32 =
    K_AUDIO_FORMAT_FLAG_IS_FLOAT | K_AUDIO_FORMAT_FLAG_IS_PACKED;

// ── AudioBuffer / AudioBufferList ──

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AudioBuffer {
    pub number_channels: u32,
    pub data_byte_size: u32,
    pub data: *mut c_void,
}

impl Default for AudioBuffer {
    fn default() -> Self {
        Self {
            number_channels: 0,
            data_byte_size: 0,
            data: std::ptr::null_mut(),
        }
    }
}

/// AudioBufferList with a flexible array member.
///
/// In C this is:
/// ```c
/// struct AudioBufferList {
///     UInt32 mNumberBuffers;
///     AudioBuffer mBuffers[1]; // variable length
/// };
/// ```
///
/// We represent the fixed header here. For multi-buffer lists, allocate extra
/// memory and use pointer arithmetic.
#[repr(C)]
#[derive(Debug)]
pub struct AudioBufferList {
    pub number_buffers: u32,
    /// First buffer (additional buffers follow in memory).
    pub buffers: [AudioBuffer; 1],
}

// ── AudioUnitRenderActionFlags ──

pub type AudioUnitRenderActionFlags = u32;

pub const K_AUDIO_UNIT_RENDER_ACTION_PRE_RENDER: AudioUnitRenderActionFlags = 1 << 2;
pub const K_AUDIO_UNIT_RENDER_ACTION_POST_RENDER: AudioUnitRenderActionFlags = 1 << 3;
pub const K_AUDIO_UNIT_RENDER_ACTION_OUTPUT_IS_SILENCE: AudioUnitRenderActionFlags = 1 << 4;

// ── AudioTimeStamp ──

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AudioTimeStamp {
    pub sample_time: f64,
    pub host_time: u64,
    pub rate_scalar: f64,
    pub word_clock_time: u64,
    pub smpte_time: SMPTETime,
    pub flags: u32,
    pub reserved: u32,
}

impl Default for AudioTimeStamp {
    fn default() -> Self {
        Self {
            sample_time: 0.0,
            host_time: 0,
            rate_scalar: 1.0,
            word_clock_time: 0,
            smpte_time: SMPTETime::default(),
            flags: K_AUDIO_TIME_STAMP_SAMPLE_TIME_VALID,
            reserved: 0,
        }
    }
}

impl AudioTimeStamp {
    pub fn with_sample_time(sample_time: f64) -> Self {
        Self {
            sample_time,
            flags: K_AUDIO_TIME_STAMP_SAMPLE_TIME_VALID,
            ..Default::default()
        }
    }
}

pub const K_AUDIO_TIME_STAMP_SAMPLE_TIME_VALID: u32 = 1 << 0;
pub const K_AUDIO_TIME_STAMP_HOST_TIME_VALID: u32 = 1 << 1;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SMPTETime {
    pub subframes: i16,
    pub subframe_divisor: i16,
    pub counter: u32,
    pub time_type: u32,
    pub flags: u32,
    pub hours: i16,
    pub minutes: i16,
    pub seconds: i16,
    pub frames: i16,
}

// ── AudioUnit property/scope/element constants ──

// Scopes
pub const K_AUDIO_UNIT_SCOPE_GLOBAL: u32 = 0;
pub const K_AUDIO_UNIT_SCOPE_INPUT: u32 = 1;
pub const K_AUDIO_UNIT_SCOPE_OUTPUT: u32 = 2;

// Properties
pub const K_AUDIO_UNIT_PROPERTY_CLASS_INFO: u32 = 0;
pub const K_AUDIO_UNIT_PROPERTY_MAKE_CONNECTION: u32 = 1;
pub const K_AUDIO_UNIT_PROPERTY_SAMPLE_RATE: u32 = 2;
pub const K_AUDIO_UNIT_PROPERTY_PARAMETER_LIST: u32 = 3;
pub const K_AUDIO_UNIT_PROPERTY_PARAMETER_INFO: u32 = 4;
pub const K_AUDIO_UNIT_PROPERTY_STREAM_FORMAT: u32 = 8;
pub const K_AUDIO_UNIT_PROPERTY_ELEMENT_COUNT: u32 = 11;
pub const K_AUDIO_UNIT_PROPERTY_LATENCY: u32 = 12;
pub const K_AUDIO_UNIT_PROPERTY_SUPPORTED_NUM_CHANNELS: u32 = 13;
pub const K_AUDIO_UNIT_PROPERTY_MAXIMUM_FRAMES_PER_SLICE: u32 = 14;
pub const K_AUDIO_UNIT_PROPERTY_SET_RENDER_CALLBACK: u32 = 23;
pub const K_AUDIO_UNIT_PROPERTY_FACTORY_PRESETS: u32 = 24;
pub const K_AUDIO_UNIT_PROPERTY_RENDER_QUALITY: u32 = 26;
pub const K_AUDIO_UNIT_PROPERTY_HOST_CALLBACKS: u32 = 27;
pub const K_AUDIO_UNIT_PROPERTY_IN_PLACE_PROCESSING: u32 = 29;
pub const K_AUDIO_UNIT_PROPERTY_ELEMENT_NAME: u32 = 30;
pub const K_AUDIO_UNIT_PROPERTY_BYPASS_EFFECT: u32 = 21;
pub const K_AUDIO_UNIT_PROPERTY_LAST_RENDER_ERROR: u32 = 22;
pub const K_AUDIO_UNIT_PROPERTY_PRESENT_PRESET: u32 = 36;
pub const K_AUDIO_UNIT_PROPERTY_COCOA_UI: u32 = 31;

// ── AudioUnitCocoaViewInfo ──

/// Returned by kAudioUnitProperty_CocoaUI. The struct has a variable-length
/// array of class names; we declare it with [CFStringRef; 1] and use pointer
/// arithmetic if more than one view class is advertised (rare).
#[repr(C)]
pub struct AudioUnitCocoaViewInfo {
    pub bundle_url: core_foundation_sys::url::CFURLRef,
    pub class_name: [core_foundation_sys::string::CFStringRef; 1],
}

// ── NSRect / NSSize (LP64 macOS ABI) ──

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NSPoint {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NSSize {
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NSRect {
    pub origin: NSPoint,
    pub size: NSSize,
}

// objc2 Encode impls so these types can be passed through msg_send.
unsafe impl objc2::encode::Encode for NSPoint {
    const ENCODING: objc2::encode::Encoding = objc2::encode::Encoding::Struct(
        "CGPoint",
        &[
            objc2::encode::Encoding::Double,
            objc2::encode::Encoding::Double,
        ],
    );
}

unsafe impl objc2::encode::Encode for NSSize {
    const ENCODING: objc2::encode::Encoding = objc2::encode::Encoding::Struct(
        "CGSize",
        &[
            objc2::encode::Encoding::Double,
            objc2::encode::Encoding::Double,
        ],
    );
}

unsafe impl objc2::encode::Encode for NSRect {
    const ENCODING: objc2::encode::Encoding = objc2::encode::Encoding::Struct(
        "CGRect",
        &[NSPoint::ENCODING, NSSize::ENCODING],
    );
}

// ── AudioUnitParameterInfo ──

#[repr(C)]
#[derive(Debug, Clone)]
pub struct AudioUnitParameterInfo {
    pub name: [u8; 52],
    pub unit_name: core_foundation_sys::string::CFStringRef,
    pub clump_id: u32,
    pub name_string: core_foundation_sys::string::CFStringRef,
    pub unit: u32,
    pub min_value: f32,
    pub max_value: f32,
    pub default_value: f32,
    pub flags: u32,
}

// Parameter unit types
pub const K_AUDIO_UNIT_PARAMETER_UNIT_GENERIC: u32 = 0;
pub const K_AUDIO_UNIT_PARAMETER_UNIT_BOOLEAN: u32 = 2;
pub const K_AUDIO_UNIT_PARAMETER_UNIT_PERCENT: u32 = 3;
pub const K_AUDIO_UNIT_PARAMETER_UNIT_SECONDS: u32 = 4;
pub const K_AUDIO_UNIT_PARAMETER_UNIT_HERTZ: u32 = 6;
pub const K_AUDIO_UNIT_PARAMETER_UNIT_DECIBELS: u32 = 7;
pub const K_AUDIO_UNIT_PARAMETER_UNIT_LINEAR_GAIN: u32 = 8;

// Parameter flags
pub const K_AUDIO_UNIT_PARAMETER_FLAG_IS_READABLE: u32 = 1 << 30;
pub const K_AUDIO_UNIT_PARAMETER_FLAG_IS_WRITABLE: u32 = 1 << 31;
pub const K_AUDIO_UNIT_PARAMETER_FLAG_HAS_NAME: u32 = 1 << 2;
pub const K_AUDIO_UNIT_PARAMETER_FLAG_HAS_CF_NAME_STRING: u32 = 1 << 29;

// Render callback type
pub type AURenderCallback = unsafe extern "C" fn(
    in_ref_con: *mut c_void,
    io_action_flags: *mut AudioUnitRenderActionFlags,
    in_time_stamp: *const AudioTimeStamp,
    in_bus_number: u32,
    in_number_frames: u32,
    io_data: *mut AudioBufferList,
) -> OSStatus;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AURenderCallbackStruct {
    pub input_proc: AURenderCallback,
    pub input_proc_ref_con: *mut c_void,
}

// ── AudioToolbox framework FFI ──

#[cfg(target_os = "macos")]
#[link(name = "AudioToolbox", kind = "framework")]
extern "C" {
    pub fn AudioComponentFindNext(
        in_component: AudioComponent,
        in_desc: *const AudioComponentDescription,
    ) -> AudioComponent;

    pub fn AudioComponentInstanceNew(
        in_component: AudioComponent,
        out_instance: *mut AudioComponentInstance,
    ) -> OSStatus;

    pub fn AudioComponentInstanceDispose(in_instance: AudioComponentInstance) -> OSStatus;

    pub fn AudioUnitInitialize(in_unit: AudioUnit) -> OSStatus;

    pub fn AudioUnitUninitialize(in_unit: AudioUnit) -> OSStatus;

    pub fn AudioUnitRender(
        in_unit: AudioUnit,
        io_action_flags: *mut AudioUnitRenderActionFlags,
        in_time_stamp: *const AudioTimeStamp,
        in_output_bus_number: u32,
        in_number_frames: u32,
        io_data: *mut AudioBufferList,
    ) -> OSStatus;

    pub fn AudioUnitSetProperty(
        in_unit: AudioUnit,
        in_id: u32,
        in_scope: u32,
        in_element: u32,
        in_data: *const c_void,
        in_data_size: u32,
    ) -> OSStatus;

    pub fn AudioUnitGetProperty(
        in_unit: AudioUnit,
        in_id: u32,
        in_scope: u32,
        in_element: u32,
        out_data: *mut c_void,
        io_data_size: *mut u32,
    ) -> OSStatus;

    pub fn AudioUnitGetPropertyInfo(
        in_unit: AudioUnit,
        in_id: u32,
        in_scope: u32,
        in_element: u32,
        out_data_size: *mut u32,
        out_writable: *mut i32,
    ) -> OSStatus;

    pub fn AudioUnitSetParameter(
        in_unit: AudioUnit,
        in_id: u32,
        in_scope: u32,
        in_element: u32,
        in_value: f32,
        in_buffer_offset_in_frames: u32,
    ) -> OSStatus;

    pub fn AudioUnitGetParameter(
        in_unit: AudioUnit,
        in_id: u32,
        in_scope: u32,
        in_element: u32,
        out_value: *mut f32,
    ) -> OSStatus;

    pub fn AudioComponentCopyName(
        in_component: AudioComponent,
        out_name: *mut core_foundation_sys::string::CFStringRef,
    ) -> OSStatus;

    pub fn AudioComponentGetDescription(
        in_component: AudioComponent,
        out_desc: *mut AudioComponentDescription,
    ) -> OSStatus;

    pub fn AudioComponentCount(in_desc: *const AudioComponentDescription) -> u32;
}

// ── Helpers ──

/// Convert a four-char code to a human-readable string.
pub fn fourcc_to_string(code: u32) -> String {
    let bytes = code.to_be_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

/// Convert a CFStringRef to a Rust String, returning empty string on null.
///
/// # Safety
/// The caller must ensure `cf_str` is either null or a valid CFStringRef.
#[cfg(target_os = "macos")]
pub unsafe fn cfstring_to_string(
    cf_str: core_foundation_sys::string::CFStringRef,
) -> String {
    if cf_str.is_null() {
        return String::new();
    }
    use core_foundation::string::CFString;
    use core_foundation::base::TCFType;
    // Retain + wrap so CFString doesn't over-release
    let s: CFString = TCFType::wrap_under_get_rule(cf_str);
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fourcc_to_string() {
        assert_eq!(fourcc_to_string(K_AUDIO_UNIT_TYPE_EFFECT), "aufx");
        assert_eq!(fourcc_to_string(K_AUDIO_UNIT_TYPE_MUSIC_DEVICE), "aumu");
        assert_eq!(fourcc_to_string(K_AUDIO_UNIT_TYPE_GENERATOR), "augn");
    }

    #[test]
    fn test_asbd_default() {
        let asbd = AudioStreamBasicDescription::default();
        assert_eq!(asbd.sample_rate, 44100.0);
        assert_eq!(asbd.channels_per_frame, 2);
        assert_eq!(asbd.bits_per_channel, 32);
    }

    #[test]
    fn test_asbd_float32() {
        let asbd = AudioStreamBasicDescription::float32(48000.0, 1);
        assert_eq!(asbd.sample_rate, 48000.0);
        assert_eq!(asbd.channels_per_frame, 1);
        assert_eq!(asbd.bits_per_channel, 32);
        assert_ne!(asbd.format_flags & K_AUDIO_FORMAT_FLAG_IS_FLOAT, 0);
        assert_ne!(
            asbd.format_flags & K_AUDIO_FORMAT_FLAG_IS_NON_INTERLEAVED,
            0
        );
    }

    #[test]
    fn test_timestamp_default() {
        let ts = AudioTimeStamp::default();
        assert_eq!(ts.sample_time, 0.0);
        assert_ne!(ts.flags & K_AUDIO_TIME_STAMP_SAMPLE_TIME_VALID, 0);
    }

    #[test]
    fn test_timestamp_with_sample_time() {
        let ts = AudioTimeStamp::with_sample_time(1024.0);
        assert_eq!(ts.sample_time, 1024.0);
    }

    #[test]
    fn test_audio_component_description_default() {
        let desc = AudioComponentDescription::default();
        assert_eq!(desc.component_type, 0);
        assert_eq!(desc.component_sub_type, 0);
        assert_eq!(desc.component_manufacturer, 0);
    }
}
