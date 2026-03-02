//! AuInstance: Audio Unit lifecycle, processing, and parameter control.
//!
//! Wraps a single AudioComponentInstance (AUv2 plugin) with a safe Rust API.

#[cfg(target_os = "macos")]
use crate::types::*;
#[cfg(target_os = "macos")]
use crate::component::AuType;

use std::fmt;

/// Error type for AU operations.
#[derive(Debug, Clone)]
pub enum AuError {
    /// An AudioToolbox call returned a non-zero OSStatus.
    OsStatus {
        function: &'static str,
        code: i32,
    },
    /// The component handle was null.
    NullComponent,
    /// The AU instance is not initialized.
    NotInitialized,
    /// Invalid buffer configuration.
    InvalidBuffer(String),
}

impl fmt::Display for AuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuError::OsStatus { function, code } => {
                write!(f, "AudioUnit error in {}: OSStatus {}", function, code)
            }
            AuError::NullComponent => write!(f, "Null AudioComponent handle"),
            AuError::NotInitialized => write!(f, "AudioUnit not initialized"),
            AuError::InvalidBuffer(msg) => write!(f, "Invalid buffer: {}", msg),
        }
    }
}

impl std::error::Error for AuError {}

pub type Result<T> = std::result::Result<T, AuError>;

/// Check an OSStatus return value.
#[cfg(target_os = "macos")]
fn check(function: &'static str, status: OSStatus) -> Result<()> {
    if status == NO_ERR {
        Ok(())
    } else {
        Err(AuError::OsStatus {
            function,
            code: status,
        })
    }
}

/// Information about a single AU parameter.
#[derive(Debug, Clone)]
pub struct AuParameterInfo {
    pub id: u32,
    pub name: String,
    pub min_value: f32,
    pub max_value: f32,
    pub default_value: f32,
    pub unit: u32,
    pub flags: u32,
}

impl AuParameterInfo {
    /// Whether this parameter can be read.
    pub fn is_readable(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.flags & K_AUDIO_UNIT_PARAMETER_FLAG_IS_READABLE != 0
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    /// Whether this parameter can be written.
    pub fn is_writable(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.flags & K_AUDIO_UNIT_PARAMETER_FLAG_IS_WRITABLE != 0
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
}

/// A hosted Audio Unit instance.
///
/// Manages the full lifecycle: instantiation, initialization, processing,
/// parameter control, and state management.
#[cfg(target_os = "macos")]
pub struct AuInstance {
    /// The raw AudioComponentInstance handle.
    component_instance: AudioComponentInstance,
    /// The AudioComponent this was instantiated from (for name queries).
    component: AudioComponent,
    /// Whether AudioUnitInitialize has been called.
    initialized: bool,
    /// Current sample rate.
    sample_rate: f64,
    /// Maximum frames per render call.
    block_size: u32,
    /// Number of input channels.
    num_inputs: u32,
    /// Number of output channels.
    num_outputs: u32,
    /// AU type category.
    au_type: AuType,
    /// Pre-allocated AudioBufferList for rendering (one buffer per output channel).
    render_buffer_list: Vec<u8>,
    /// Pre-allocated sample buffers for output (one Vec<f32> per channel).
    output_sample_buffers: Vec<Vec<f32>>,
    /// Pre-allocated sample buffers for input render callback.
    input_sample_buffers: Vec<Vec<f32>>,
    /// Current sample position for AudioTimeStamp.
    sample_position: f64,
}

#[cfg(target_os = "macos")]
unsafe impl Send for AuInstance {}

#[cfg(target_os = "macos")]
impl AuInstance {
    /// Create a new AuInstance from an AudioComponent.
    ///
    /// This instantiates but does NOT initialize the AU. Call `initialize()`
    /// before processing.
    ///
    /// # Safety
    /// `component` must be a valid, non-null `AudioComponent` handle obtained
    /// from `AudioComponentFindNext` or equivalent API.
    pub unsafe fn new(
        component: AudioComponent,
        sample_rate: f64,
        block_size: u32,
    ) -> Result<Self> {
        if component.is_null() {
            return Err(AuError::NullComponent);
        }

        let mut instance: AudioComponentInstance = std::ptr::null_mut();
        check(
            "AudioComponentInstanceNew",
            unsafe { AudioComponentInstanceNew(component, &mut instance) },
        )?;

        // Determine AU type
        let mut desc = AudioComponentDescription::default();
        let _ = unsafe { AudioComponentGetDescription(component, &mut desc) };
        let au_type = AuType::from_raw(desc.component_type);

        let mut au = Self {
            component_instance: instance,
            component,
            initialized: false,
            sample_rate,
            block_size,
            num_inputs: 0,
            num_outputs: 2,
            au_type,
            render_buffer_list: Vec::new(),
            output_sample_buffers: Vec::new(),
            input_sample_buffers: Vec::new(),
            sample_position: 0.0,
        };

        // Configure stream format and block size before initialization
        au.configure_pre_init()?;

        Ok(au)
    }

    /// Pre-initialization configuration: set stream format and max frames.
    fn configure_pre_init(&mut self) -> Result<()> {
        let unit = self.component_instance;

        // Set maximum frames per slice
        let max_frames = self.block_size;
        check(
            "SetProperty(MaxFramesPerSlice)",
            unsafe {
                AudioUnitSetProperty(
                    unit,
                    K_AUDIO_UNIT_PROPERTY_MAXIMUM_FRAMES_PER_SLICE,
                    K_AUDIO_UNIT_SCOPE_GLOBAL,
                    0,
                    &max_frames as *const u32 as *const std::os::raw::c_void,
                    std::mem::size_of::<u32>() as u32,
                )
            },
        )?;

        // Query the default output stream format to discover channel count
        let mut asbd = AudioStreamBasicDescription::default();
        let mut size = std::mem::size_of::<AudioStreamBasicDescription>() as u32;
        let status = unsafe {
            AudioUnitGetProperty(
                unit,
                K_AUDIO_UNIT_PROPERTY_STREAM_FORMAT,
                K_AUDIO_UNIT_SCOPE_OUTPUT,
                0,
                &mut asbd as *mut AudioStreamBasicDescription as *mut std::os::raw::c_void,
                &mut size,
            )
        };
        if status == NO_ERR {
            self.num_outputs = asbd.channels_per_frame;
        }

        // Query input channels (may fail for instruments/generators with no input)
        let mut input_asbd = AudioStreamBasicDescription::default();
        let mut isize = std::mem::size_of::<AudioStreamBasicDescription>() as u32;
        let status = unsafe {
            AudioUnitGetProperty(
                unit,
                K_AUDIO_UNIT_PROPERTY_STREAM_FORMAT,
                K_AUDIO_UNIT_SCOPE_INPUT,
                0,
                &mut input_asbd as *mut AudioStreamBasicDescription as *mut std::os::raw::c_void,
                &mut isize,
            )
        };
        if status == NO_ERR {
            self.num_inputs = input_asbd.channels_per_frame;
        }

        // Set our desired stream format on the output scope
        let out_asbd = AudioStreamBasicDescription::float32(self.sample_rate, self.num_outputs.max(2));
        self.num_outputs = out_asbd.channels_per_frame;
        let _ = unsafe {
            AudioUnitSetProperty(
                unit,
                K_AUDIO_UNIT_PROPERTY_STREAM_FORMAT,
                K_AUDIO_UNIT_SCOPE_OUTPUT,
                0,
                &out_asbd as *const AudioStreamBasicDescription as *const std::os::raw::c_void,
                std::mem::size_of::<AudioStreamBasicDescription>() as u32,
            )
        };

        // Set input stream format if we have inputs
        if self.num_inputs > 0 {
            let in_asbd =
                AudioStreamBasicDescription::float32(self.sample_rate, self.num_inputs);
            let _ = unsafe {
                AudioUnitSetProperty(
                    unit,
                    K_AUDIO_UNIT_PROPERTY_STREAM_FORMAT,
                    K_AUDIO_UNIT_SCOPE_INPUT,
                    0,
                    &in_asbd as *const AudioStreamBasicDescription
                        as *const std::os::raw::c_void,
                    std::mem::size_of::<AudioStreamBasicDescription>() as u32,
                )
            };
        }

        Ok(())
    }

    /// Initialize the Audio Unit. Must be called before processing.
    pub fn initialize(&mut self) -> Result<()> {
        check(
            "AudioUnitInitialize",
            unsafe { AudioUnitInitialize(self.component_instance) },
        )?;
        self.initialized = true;

        // Allocate render buffers now that we know the format
        self.allocate_buffers();

        Ok(())
    }

    /// Uninitialize the Audio Unit.
    pub fn uninitialize(&mut self) -> Result<()> {
        if self.initialized {
            check(
                "AudioUnitUninitialize",
                unsafe { AudioUnitUninitialize(self.component_instance) },
            )?;
            self.initialized = false;
        }
        Ok(())
    }

    /// Allocate pre-sized render buffers.
    fn allocate_buffers(&mut self) {
        let num_channels = self.num_outputs as usize;
        let buffer_size = self.block_size as usize;

        // Allocate per-channel output buffers
        self.output_sample_buffers.clear();
        for _ in 0..num_channels {
            self.output_sample_buffers
                .push(vec![0.0f32; buffer_size]);
        }

        // Allocate per-channel input buffers
        let num_in = self.num_inputs as usize;
        self.input_sample_buffers.clear();
        for _ in 0..num_in.max(num_channels) {
            self.input_sample_buffers
                .push(vec![0.0f32; buffer_size]);
        }

        // Allocate the AudioBufferList in raw bytes
        // Layout: u32 (number_buffers) + N * AudioBuffer
        let abl_size = std::mem::size_of::<u32>()
            + num_channels * std::mem::size_of::<AudioBuffer>();
        self.render_buffer_list = vec![0u8; abl_size];
    }

    /// Set up the render buffer list to point to our pre-allocated sample buffers.
    fn setup_render_buffer_list(&mut self, num_frames: u32) -> *mut AudioBufferList {
        let num_channels = self.num_outputs as usize;
        let ptr = self.render_buffer_list.as_mut_ptr();

        unsafe {
            // Write number_buffers
            let abl = ptr as *mut AudioBufferList;
            (*abl).number_buffers = num_channels as u32;

            // Write each AudioBuffer, pointing to our sample buffers
            let buffers_ptr = &mut (*abl).buffers[0] as *mut AudioBuffer;
            for ch in 0..num_channels {
                let buf = &mut *buffers_ptr.add(ch);
                buf.number_channels = 1; // non-interleaved
                buf.data_byte_size = num_frames * std::mem::size_of::<f32>() as u32;
                buf.data = self.output_sample_buffers[ch].as_mut_ptr()
                    as *mut std::os::raw::c_void;
            }

            abl
        }
    }

    /// Process audio through the Audio Unit.
    ///
    /// `input`: slice of channel slices (may be empty for instruments/generators).
    /// `output`: mutable slice of channel slices to write into.
    /// `num_frames`: number of frames to process (must be <= block_size).
    pub fn process(
        &mut self,
        input: &[&[f32]],
        output: &mut [&mut [f32]],
        num_frames: u32,
    ) -> Result<()> {
        if !self.initialized {
            return Err(AuError::NotInitialized);
        }

        if num_frames > self.block_size {
            return Err(AuError::InvalidBuffer(format!(
                "num_frames ({}) > block_size ({})",
                num_frames, self.block_size
            )));
        }

        // Copy input data into our input buffers (for the render callback)
        for (ch, src) in input.iter().enumerate() {
            if ch < self.input_sample_buffers.len() {
                let len = (num_frames as usize).min(src.len());
                self.input_sample_buffers[ch][..len].copy_from_slice(&src[..len]);
            }
        }

        // If we have inputs, set up a render callback so the AU can pull input
        if !input.is_empty() {
            self.set_input_callback()?;
        }

        // Set up the output AudioBufferList
        let abl = self.setup_render_buffer_list(num_frames);

        // Create timestamp
        let timestamp = AudioTimeStamp::with_sample_time(self.sample_position);

        // Render
        let mut action_flags: AudioUnitRenderActionFlags = 0;
        check(
            "AudioUnitRender",
            unsafe {
                AudioUnitRender(
                    self.component_instance,
                    &mut action_flags,
                    &timestamp,
                    0, // output bus 0
                    num_frames,
                    abl,
                )
            },
        )?;

        // Advance sample position
        self.sample_position += num_frames as f64;

        // Copy rendered output to caller's buffers
        let num_out_ch = self.num_outputs as usize;
        for (ch, dst) in output.iter_mut().enumerate() {
            if ch < num_out_ch {
                let len = (num_frames as usize).min(dst.len());
                dst[..len].copy_from_slice(&self.output_sample_buffers[ch][..len]);
            }
        }

        Ok(())
    }

    /// Set the input render callback (for effect-type AUs that pull input).
    fn set_input_callback(&mut self) -> Result<()> {
        let callback = AURenderCallbackStruct {
            input_proc: au_input_render_callback,
            input_proc_ref_con: self as *mut AuInstance as *mut std::os::raw::c_void,
        };

        check(
            "SetProperty(RenderCallback)",
            unsafe {
                AudioUnitSetProperty(
                    self.component_instance,
                    K_AUDIO_UNIT_PROPERTY_SET_RENDER_CALLBACK,
                    K_AUDIO_UNIT_SCOPE_INPUT,
                    0,
                    &callback as *const AURenderCallbackStruct as *const std::os::raw::c_void,
                    std::mem::size_of::<AURenderCallbackStruct>() as u32,
                )
            },
        )
    }

    /// Set a parameter value.
    pub fn set_parameter(&mut self, id: u32, value: f32) -> Result<()> {
        check(
            "AudioUnitSetParameter",
            unsafe {
                AudioUnitSetParameter(
                    self.component_instance,
                    id,
                    K_AUDIO_UNIT_SCOPE_GLOBAL,
                    0,
                    value,
                    0,
                )
            },
        )
    }

    /// Get a parameter value.
    pub fn get_parameter(&self, id: u32) -> Result<f32> {
        let mut value: f32 = 0.0;
        check(
            "AudioUnitGetParameter",
            unsafe {
                AudioUnitGetParameter(
                    self.component_instance,
                    id,
                    K_AUDIO_UNIT_SCOPE_GLOBAL,
                    0,
                    &mut value,
                )
            },
        )?;
        Ok(value)
    }

    /// Get the list of all parameters.
    pub fn get_parameter_list(&self) -> Result<Vec<AuParameterInfo>> {
        let unit = self.component_instance;
        let mut results = Vec::new();

        // First, get the parameter ID list
        let mut data_size: u32 = 0;
        let mut writable: i32 = 0;
        let status = unsafe {
            AudioUnitGetPropertyInfo(
                unit,
                K_AUDIO_UNIT_PROPERTY_PARAMETER_LIST,
                K_AUDIO_UNIT_SCOPE_GLOBAL,
                0,
                &mut data_size,
                &mut writable,
            )
        };
        if status != NO_ERR || data_size == 0 {
            return Ok(results);
        }

        let num_params = data_size as usize / std::mem::size_of::<u32>();
        let mut param_ids = vec![0u32; num_params];
        let mut actual_size = data_size;
        let status = unsafe {
            AudioUnitGetProperty(
                unit,
                K_AUDIO_UNIT_PROPERTY_PARAMETER_LIST,
                K_AUDIO_UNIT_SCOPE_GLOBAL,
                0,
                param_ids.as_mut_ptr() as *mut std::os::raw::c_void,
                &mut actual_size,
            )
        };
        if status != NO_ERR {
            return Ok(results);
        }

        // Now get info for each parameter
        for &param_id in &param_ids {
            if let Ok(info) = self.get_parameter_info(param_id) {
                results.push(info);
            }
        }

        Ok(results)
    }

    /// Get info about a single parameter.
    fn get_parameter_info(&self, param_id: u32) -> Result<AuParameterInfo> {
        let unit = self.component_instance;
        let mut info: AudioUnitParameterInfo = unsafe { std::mem::zeroed() };
        let mut size = std::mem::size_of::<AudioUnitParameterInfo>() as u32;

        check(
            "GetProperty(ParameterInfo)",
            unsafe {
                AudioUnitGetProperty(
                    unit,
                    K_AUDIO_UNIT_PROPERTY_PARAMETER_INFO,
                    K_AUDIO_UNIT_SCOPE_GLOBAL,
                    param_id,
                    &mut info as *mut AudioUnitParameterInfo as *mut std::os::raw::c_void,
                    &mut size,
                )
            },
        )?;

        // Extract name: prefer CFString name, fall back to C char array
        let name = if info.flags & K_AUDIO_UNIT_PARAMETER_FLAG_HAS_CF_NAME_STRING != 0
            && !info.name_string.is_null()
        {
            let s = unsafe { cfstring_to_string(info.name_string) };
            // Release the CFString (it's owned by us per the API contract)
            unsafe {
                core_foundation_sys::base::CFRelease(
                    info.name_string as *const std::os::raw::c_void,
                );
            }
            s
        } else {
            // Read from the fixed char array
            let end = info.name.iter().position(|&b| b == 0).unwrap_or(info.name.len());
            String::from_utf8_lossy(&info.name[..end]).to_string()
        };

        Ok(AuParameterInfo {
            id: param_id,
            name,
            min_value: info.min_value,
            max_value: info.max_value,
            default_value: info.default_value,
            unit: info.unit,
            flags: info.flags,
        })
    }

    /// Get the plugin's reported latency in frames.
    pub fn get_latency(&self) -> Result<u32> {
        let unit = self.component_instance;
        let mut latency: f64 = 0.0;
        let mut size = std::mem::size_of::<f64>() as u32;

        let status = unsafe {
            AudioUnitGetProperty(
                unit,
                K_AUDIO_UNIT_PROPERTY_LATENCY,
                K_AUDIO_UNIT_SCOPE_GLOBAL,
                0,
                &mut latency as *mut f64 as *mut std::os::raw::c_void,
                &mut size,
            )
        };

        if status != NO_ERR {
            return Ok(0);
        }

        // Latency is in seconds; convert to frames
        Ok((latency * self.sample_rate) as u32)
    }

    /// Save the AU's state as a property list (binary plist).
    pub fn save_state(&self) -> Result<Vec<u8>> {
        let unit = self.component_instance;
        let mut class_info: core_foundation_sys::dictionary::CFDictionaryRef =
            std::ptr::null();
        let mut size =
            std::mem::size_of::<core_foundation_sys::dictionary::CFDictionaryRef>() as u32;

        check(
            "GetProperty(ClassInfo)",
            unsafe {
                AudioUnitGetProperty(
                    unit,
                    K_AUDIO_UNIT_PROPERTY_CLASS_INFO,
                    K_AUDIO_UNIT_SCOPE_GLOBAL,
                    0,
                    &mut class_info as *mut core_foundation_sys::dictionary::CFDictionaryRef
                        as *mut std::os::raw::c_void,
                    &mut size,
                )
            },
        )?;

        if class_info.is_null() {
            return Ok(Vec::new());
        }

        // Serialize to binary plist
        let data = unsafe {
            let plist_data = core_foundation_sys::propertylist::CFPropertyListCreateData(
                std::ptr::null(),
                class_info as core_foundation_sys::propertylist::CFPropertyListRef,
                core_foundation_sys::propertylist::kCFPropertyListBinaryFormat_v1_0,
                0,
                std::ptr::null_mut(),
            );
            core_foundation_sys::base::CFRelease(class_info as *const std::os::raw::c_void);

            if plist_data.is_null() {
                return Ok(Vec::new());
            }

            let len = core_foundation_sys::data::CFDataGetLength(plist_data) as usize;
            let ptr = core_foundation_sys::data::CFDataGetBytePtr(plist_data);
            let bytes = std::slice::from_raw_parts(ptr, len).to_vec();
            core_foundation_sys::base::CFRelease(plist_data as *const std::os::raw::c_void);
            bytes
        };

        Ok(data)
    }

    /// Restore the AU's state from a binary plist.
    pub fn load_state(&mut self, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let unit = self.component_instance;

        unsafe {
            let cf_data = core_foundation_sys::data::CFDataCreate(
                std::ptr::null(),
                data.as_ptr(),
                data.len() as isize,
            );
            if cf_data.is_null() {
                return Err(AuError::InvalidBuffer("Failed to create CFData".into()));
            }

            let plist = core_foundation_sys::propertylist::CFPropertyListCreateWithData(
                std::ptr::null(),
                cf_data,
                core_foundation_sys::propertylist::kCFPropertyListImmutable,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            core_foundation_sys::base::CFRelease(cf_data as *const std::os::raw::c_void);

            if plist.is_null() {
                return Err(AuError::InvalidBuffer(
                    "Failed to deserialize plist".into(),
                ));
            }

            let result = check(
                "SetProperty(ClassInfo)",
                AudioUnitSetProperty(
                    unit,
                    K_AUDIO_UNIT_PROPERTY_CLASS_INFO,
                    K_AUDIO_UNIT_SCOPE_GLOBAL,
                    0,
                    &plist as *const core_foundation_sys::propertylist::CFPropertyListRef
                        as *const std::os::raw::c_void,
                    std::mem::size_of::<core_foundation_sys::propertylist::CFPropertyListRef>()
                        as u32,
                ),
            );
            core_foundation_sys::base::CFRelease(plist);

            result
        }
    }

    /// Get the plugin name from its AudioComponent.
    pub fn get_name(&self) -> Result<String> {
        let mut name_ref: core_foundation_sys::string::CFStringRef = std::ptr::null();
        let status = unsafe { AudioComponentCopyName(self.component, &mut name_ref) };
        if status != NO_ERR || name_ref.is_null() {
            return Ok(String::from("<unknown>"));
        }
        let name = unsafe { cfstring_to_string(name_ref) };
        unsafe {
            core_foundation_sys::base::CFRelease(name_ref as *const std::os::raw::c_void);
        }
        Ok(name)
    }

    /// Raw AudioUnit handle, for use with `parameters` and `editor` modules.
    pub fn raw_unit(&self) -> AudioUnit {
        self.component_instance
    }

    /// Get the AU type category.
    pub fn au_type(&self) -> AuType {
        self.au_type
    }

    /// Get the number of input channels.
    pub fn num_inputs(&self) -> u32 {
        self.num_inputs
    }

    /// Get the number of output channels.
    pub fn num_outputs(&self) -> u32 {
        self.num_outputs
    }

    /// Get the sample rate.
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Whether `initialize()` has been called.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Update the sample rate. Requires un-initialize/re-initialize.
    pub fn set_sample_rate(&mut self, rate: f64) -> Result<()> {
        let was_initialized = self.initialized;
        if was_initialized {
            self.uninitialize()?;
        }
        self.sample_rate = rate;
        self.configure_pre_init()?;
        if was_initialized {
            self.initialize()?;
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl Drop for AuInstance {
    fn drop(&mut self) {
        if self.initialized {
            let _ = unsafe { AudioUnitUninitialize(self.component_instance) };
        }
        unsafe {
            AudioComponentInstanceDispose(self.component_instance);
        }
    }
}

/// Render callback that provides input data to effect-type AUs.
///
/// This is called by the AU during `AudioUnitRender` when it needs input audio.
/// We copy data from the AuInstance's pre-loaded input buffers.
///
/// # Safety
/// `in_ref_con` must be a valid pointer to an `AuInstance`.
#[cfg(target_os = "macos")]
unsafe extern "C" fn au_input_render_callback(
    in_ref_con: *mut std::os::raw::c_void,
    _io_action_flags: *mut AudioUnitRenderActionFlags,
    _in_time_stamp: *const AudioTimeStamp,
    _in_bus_number: u32,
    in_number_frames: u32,
    io_data: *mut AudioBufferList,
) -> OSStatus {
    if in_ref_con.is_null() || io_data.is_null() {
        return -1;
    }

    let au = &*(in_ref_con as *const AuInstance);
    let abl = &mut *io_data;
    let num_buffers = abl.number_buffers as usize;
    let buffers_ptr = &mut abl.buffers[0] as *mut AudioBuffer;

    for ch in 0..num_buffers {
        let buf = &mut *buffers_ptr.add(ch);
        if ch < au.input_sample_buffers.len() {
            let frames = (in_number_frames as usize).min(au.input_sample_buffers[ch].len());
            let dst = std::slice::from_raw_parts_mut(buf.data as *mut f32, frames);
            dst.copy_from_slice(&au.input_sample_buffers[ch][..frames]);
            buf.data_byte_size = (frames * std::mem::size_of::<f32>()) as u32;
        } else {
            // Zero-fill if we don't have enough input channels
            let frames = in_number_frames as usize;
            let dst = std::slice::from_raw_parts_mut(buf.data as *mut f32, frames);
            dst.iter_mut().for_each(|s| *s = 0.0);
        }
    }

    NO_ERR
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;
    use crate::component::*;

    fn find_apple_delay() -> Option<AudioComponent> {
        let desc = AudioComponentDescription {
            component_type: K_AUDIO_UNIT_TYPE_EFFECT,
            component_sub_type: u32::from_be_bytes(*b"dely"),
            component_manufacturer: u32::from_be_bytes(*b"appl"),
            component_flags: 0,
            component_flags_mask: 0,
        };
        find_component(&desc)
    }

    #[test]
    fn test_au_instance_new() {
        let component = find_apple_delay().expect("AUDelay should be present");
        let instance = unsafe { AuInstance::new(component, 44100.0, 512) };
        assert!(instance.is_ok(), "Failed to create AuInstance: {:?}", instance.err());
    }

    #[test]
    fn test_au_instance_initialize_uninitialize() {
        let component = find_apple_delay().expect("AUDelay should be present");
        let mut instance = unsafe { AuInstance::new(component, 44100.0, 512) }.unwrap();

        assert!(!instance.is_initialized());
        instance.initialize().expect("initialize should succeed");
        assert!(instance.is_initialized());

        instance.uninitialize().expect("uninitialize should succeed");
        assert!(!instance.is_initialized());
    }

    #[test]
    fn test_au_instance_get_name() {
        let component = find_apple_delay().expect("AUDelay should be present");
        let instance = unsafe { AuInstance::new(component, 44100.0, 512) }.unwrap();
        let name = instance.get_name().unwrap();
        assert!(!name.is_empty(), "Name should not be empty");
        eprintln!("AUDelay name: {}", name);
    }

    #[test]
    fn test_au_instance_parameter_list() {
        let component = find_apple_delay().expect("AUDelay should be present");
        let mut instance = unsafe { AuInstance::new(component, 44100.0, 512) }.unwrap();
        instance.initialize().unwrap();

        let params = instance.get_parameter_list().unwrap();
        assert!(!params.is_empty(), "AUDelay should have parameters");

        for p in &params {
            eprintln!(
                "  param id={}, name='{}', range=[{}, {}], default={}",
                p.id, p.name, p.min_value, p.max_value, p.default_value
            );
        }
    }

    #[test]
    fn test_au_instance_get_set_parameter() {
        let component = find_apple_delay().expect("AUDelay should be present");
        let mut instance = unsafe { AuInstance::new(component, 44100.0, 512) }.unwrap();
        instance.initialize().unwrap();

        let params = instance.get_parameter_list().unwrap();
        assert!(!params.is_empty());

        let p = &params[0];
        let mid = (p.min_value + p.max_value) / 2.0;
        instance.set_parameter(p.id, mid).unwrap();

        let val = instance.get_parameter(p.id).unwrap();
        assert!(
            (val - mid).abs() < 0.01,
            "Expected ~{}, got {}",
            mid,
            val
        );
    }

    #[test]
    fn test_au_instance_process_silence() {
        let component = find_apple_delay().expect("AUDelay should be present");
        let mut instance = unsafe { AuInstance::new(component, 44100.0, 512) }.unwrap();
        instance.initialize().unwrap();

        let num_frames = 512u32;
        let input = vec![vec![0.0f32; num_frames as usize]; 2];
        let mut output = vec![vec![0.0f32; num_frames as usize]; 2];

        let input_slices: Vec<&[f32]> = input.iter().map(|v| v.as_slice()).collect();
        let mut output_slices: Vec<&mut [f32]> =
            output.iter_mut().map(|v| v.as_mut_slice()).collect();

        instance
            .process(&input_slices, &mut output_slices, num_frames)
            .expect("process should succeed");
    }

    #[test]
    fn test_au_instance_process_audio() {
        let component = find_apple_delay().expect("AUDelay should be present");
        let mut instance = unsafe { AuInstance::new(component, 44100.0, 512) }.unwrap();
        instance.initialize().unwrap();

        let num_frames = 512u32;
        // Feed a sine wave
        let input: Vec<Vec<f32>> = (0..2)
            .map(|_| {
                (0..num_frames)
                    .map(|i| {
                        (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.5
                    })
                    .collect()
            })
            .collect();
        let mut output = vec![vec![0.0f32; num_frames as usize]; 2];

        let input_slices: Vec<&[f32]> = input.iter().map(|v| v.as_slice()).collect();
        let mut output_slices: Vec<&mut [f32]> =
            output.iter_mut().map(|v| v.as_mut_slice()).collect();

        instance
            .process(&input_slices, &mut output_slices, num_frames)
            .expect("process should succeed");

        // AUDelay with default delay time should pass through some audio
        // (may not produce non-zero on first block due to delay, but should not crash)
    }

    #[test]
    fn test_au_instance_latency() {
        let component = find_apple_delay().expect("AUDelay should be present");
        let mut instance = unsafe { AuInstance::new(component, 44100.0, 512) }.unwrap();
        instance.initialize().unwrap();
        let latency = instance.get_latency().unwrap();
        eprintln!("AUDelay latency: {} frames", latency);
        // Latency is implementation-defined, just verify no crash
    }

    #[test]
    fn test_au_instance_save_load_state() {
        let component = find_apple_delay().expect("AUDelay should be present");
        let mut instance = unsafe { AuInstance::new(component, 44100.0, 512) }.unwrap();
        instance.initialize().unwrap();

        let state = instance.save_state().unwrap();
        assert!(!state.is_empty(), "Saved state should not be empty");

        // Should be able to load it back
        instance
            .load_state(&state)
            .expect("load_state should succeed");
    }

    #[test]
    fn test_au_instance_set_sample_rate() {
        let component = find_apple_delay().expect("AUDelay should be present");
        let mut instance = unsafe { AuInstance::new(component, 44100.0, 512) }.unwrap();
        instance.initialize().unwrap();
        assert!(instance.is_initialized());

        instance.set_sample_rate(48000.0).unwrap();
        assert_eq!(instance.sample_rate(), 48000.0);
        assert!(instance.is_initialized());
    }
}
