#![cfg(target_os = "macos")]

mod cocoa;

use objc2::msg_send;
use objc2::runtime::AnyObject;
use std::os::raw::c_void;

use crate::error::Result;
use crate::ffi::property_size;
use crate::types::*;

pub struct AuEditor {
    view: *mut AnyObject,
    unit: AudioUnit,
}

unsafe impl Send for AuEditor {}

impl AuEditor {
    /// # Safety
    /// `unit` must be a valid, initialized AudioUnit handle.
    /// `parent` (if non-null) must be a valid NSView pointer.
    pub unsafe fn open(unit: AudioUnit, parent: *mut c_void) -> Result<Self> {
        let view = cocoa::create_view(unit)?;

        if !parent.is_null() {
            let parent_obj = parent as *mut AnyObject;
            let _: () = msg_send![parent_obj, addSubview: view];
        }

        Ok(Self { view, unit })
    }

    pub fn has_editor(unit: AudioUnit) -> bool {
        let size = unsafe {
            property_size(
                unit,
                K_AUDIO_UNIT_PROPERTY_COCOA_UI,
                K_AUDIO_UNIT_SCOPE_GLOBAL,
                0,
            )
        };
        matches!(size, Ok(n) if n > 0)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::*;

    #[test]
    fn test_has_editor() {
        let desc = AudioComponentDescription {
            component_type: K_AUDIO_UNIT_TYPE_EFFECT,
            component_sub_type: u32::from_be_bytes(*b"dely"),
            component_manufacturer: u32::from_be_bytes(*b"appl"),
            component_flags: 0,
            component_flags_mask: 0,
        };
        let comp = find_component(&desc).expect("AUDelay should be present");
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
