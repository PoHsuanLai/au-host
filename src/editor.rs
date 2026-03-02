use crate::instance::{AuError, Result};
use crate::types::*;
use objc2::msg_send;
use objc2::runtime::AnyObject;
use std::ffi::CString;
use std::os::raw::c_void;

/// Wraps an AU's Cocoa NSView for GUI hosting.
pub struct AuEditor {
    view: *mut AnyObject,
    unit: AudioUnit,
}

unsafe impl Send for AuEditor {}

impl AuEditor {
    /// Query the AU for a Cocoa view factory and create the editor NSView.
    ///
    /// `parent` is the host NSView that should contain the plugin GUI.
    /// Pass null to create an unparented view.
    ///
    /// # Safety
    /// `unit` must be a valid, initialized AudioUnit handle.
    /// `parent` (if non-null) must be a valid NSView pointer.
    pub unsafe fn open(unit: AudioUnit, parent: *mut c_void) -> Result<Self> {
        let view = create_cocoa_view(unit)?;

        if !parent.is_null() {
            let parent_obj = parent as *mut AnyObject;
            let _: () = msg_send![parent_obj, addSubview: view];
        }

        Ok(Self { view, unit })
    }

    pub fn has_editor(unit: AudioUnit) -> bool {
        let mut data_size: u32 = 0;
        let mut writable: i32 = 0;
        let status = unsafe {
            AudioUnitGetPropertyInfo(
                unit,
                K_AUDIO_UNIT_PROPERTY_COCOA_UI,
                K_AUDIO_UNIT_SCOPE_GLOBAL,
                0,
                &mut data_size,
                &mut writable,
            )
        };
        status == NO_ERR && data_size > 0
    }

    pub fn close(&mut self) {
        if !self.view.is_null() {
            unsafe {
                let _: () = msg_send![self.view, removeFromSuperview];
                let _: () = msg_send![self.view, release];
            }
            self.view = std::ptr::null_mut();
        }
    }

    /// Returns (width, height) of the editor view's frame in points.
    pub fn get_size(&self) -> (u32, u32) {
        if self.view.is_null() {
            return (0, 0);
        }
        unsafe {
            let frame: NSRect = msg_send![self.view, frame];
            (frame.size.width as u32, frame.size.height as u32)
        }
    }

    pub fn view_ptr(&self) -> *mut c_void {
        self.view as *mut c_void
    }

    pub fn unit(&self) -> AudioUnit {
        self.unit
    }
}

impl Drop for AuEditor {
    fn drop(&mut self) {
        self.close();
    }
}

/// Query the AU for its AudioUnitCocoaViewInfo, load the view factory class,
/// and ask it to create an NSView for the AU instance.
unsafe fn create_cocoa_view(unit: AudioUnit) -> Result<*mut AnyObject> {
    let mut data_size: u32 = 0;
    let mut writable: i32 = 0;
    let status = AudioUnitGetPropertyInfo(
        unit,
        K_AUDIO_UNIT_PROPERTY_COCOA_UI,
        K_AUDIO_UNIT_SCOPE_GLOBAL,
        0,
        &mut data_size,
        &mut writable,
    );
    if status != NO_ERR || data_size == 0 {
        return Err(AuError::OsStatus {
            function: "GetPropertyInfo(CocoaUI)",
            code: if status != NO_ERR {
                status
            } else {
                K_AUDIO_UNIT_ERR_INVALID_PROPERTY
            },
        });
    }

    let mut cocoa_info = vec![0u8; data_size as usize];
    let mut actual_size = data_size;
    let status = AudioUnitGetProperty(
        unit,
        K_AUDIO_UNIT_PROPERTY_COCOA_UI,
        K_AUDIO_UNIT_SCOPE_GLOBAL,
        0,
        cocoa_info.as_mut_ptr() as *mut c_void,
        &mut actual_size,
    );
    if status != NO_ERR {
        return Err(AuError::OsStatus {
            function: "GetProperty(CocoaUI)",
            code: status,
        });
    }

    let info_ptr = cocoa_info.as_ptr() as *const AudioUnitCocoaViewInfo;
    let bundle_url = (*info_ptr).bundle_url;
    let class_name = (*info_ptr).class_name[0];

    if bundle_url.is_null() || class_name.is_null() {
        return Err(AuError::InvalidBuffer(
            "CocoaUI info has null bundle URL or class name".into(),
        ));
    }

    // Load the bundle containing the view factory
    let ns_bundle_class =
        objc2::runtime::AnyClass::get(c"NSBundle").expect("NSBundle class must exist");
    let bundle: *mut AnyObject =
        msg_send![ns_bundle_class, bundleWithURL: bundle_url as *mut c_void];
    if bundle.is_null() {
        core_foundation_sys::base::CFRelease(bundle_url as *const c_void);
        core_foundation_sys::base::CFRelease(class_name as *const c_void);
        return Err(AuError::InvalidBuffer(
            "Failed to load AU view bundle".into(),
        ));
    }

    let factory_name = cfstring_to_string(class_name);

    // Ensure the bundle's executable is loaded
    let _: bool = msg_send![bundle, load];

    let factory_cstr = CString::new(factory_name.clone()).map_err(|_| {
        AuError::InvalidBuffer(format!("Invalid class name: {}", factory_name))
    })?;
    let factory_class = objc2::runtime::AnyClass::get(&factory_cstr);
    core_foundation_sys::base::CFRelease(bundle_url as *const c_void);
    core_foundation_sys::base::CFRelease(class_name as *const c_void);

    let factory_class = factory_class.ok_or_else(|| {
        AuError::InvalidBuffer(format!("ObjC class '{}' not found in bundle", factory_name))
    })?;

    // Instantiate the factory: [[FactoryClass alloc] init]
    let factory: *mut AnyObject = msg_send![factory_class, alloc];
    let factory: *mut AnyObject = msg_send![factory, init];
    if factory.is_null() {
        return Err(AuError::InvalidBuffer(
            "Failed to instantiate AU view factory".into(),
        ));
    }

    // AUCocoaUIBase protocol: -uiViewForAudioUnit:withSize:
    let size = NSSize {
        width: 800.0,
        height: 600.0,
    };
    let view: *mut AnyObject = msg_send![factory, uiViewForAudioUnit: unit, withSize: size];

    let _: () = msg_send![factory, release];

    if view.is_null() {
        return Err(AuError::InvalidBuffer(
            "AU view factory returned null view".into(),
        ));
    }

    // Retain the view so we own it
    let view: *mut AnyObject = msg_send![view, retain];

    Ok(view)
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;
    use crate::component::*;

    fn apple_delay_component() -> AudioComponent {
        let desc = AudioComponentDescription {
            component_type: K_AUDIO_UNIT_TYPE_EFFECT,
            component_sub_type: u32::from_be_bytes(*b"dely"),
            component_manufacturer: u32::from_be_bytes(*b"appl"),
            component_flags: 0,
            component_flags_mask: 0,
        };
        find_component(&desc).expect("AUDelay should be present")
    }

    #[test]
    fn test_has_editor() {
        let comp = apple_delay_component();
        let mut instance: AudioComponentInstance = std::ptr::null_mut();
        let status = unsafe { AudioComponentInstanceNew(comp, &mut instance) };
        assert_eq!(status, NO_ERR);
        unsafe { AudioUnitInitialize(instance) };

        let _has = AuEditor::has_editor(instance);

        unsafe {
            AudioUnitUninitialize(instance);
            AudioComponentInstanceDispose(instance);
        }
    }
}
