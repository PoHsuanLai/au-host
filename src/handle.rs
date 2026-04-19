#![cfg(target_os = "macos")]

use crate::cf::CfString;
use crate::component::AuType;
use crate::error::{AuError, Result};
use crate::ffi::check;
use crate::types::*;

pub struct AuHandle {
    instance: AudioComponentInstance,
    component: AudioComponent,
    au_type: AuType,
}

unsafe impl Send for AuHandle {}

impl AuHandle {
    pub unsafe fn new(component: AudioComponent) -> Result<Self> {
        if component.is_null() {
            return Err(AuError::NullComponent);
        }

        let mut instance: AudioComponentInstance = std::ptr::null_mut();
        check(
            "AudioComponentInstanceNew",
            AudioComponentInstanceNew(component, &mut instance),
        )?;

        let mut desc = AudioComponentDescription::default();
        let _ = AudioComponentGetDescription(component, &mut desc);
        let au_type = AuType::from_raw(desc.component_type);

        Ok(Self {
            instance,
            component,
            au_type,
        })
    }

    pub fn raw_unit(&self) -> AudioUnit {
        self.instance
    }

    pub fn component(&self) -> AudioComponent {
        self.component
    }

    pub fn au_type(&self) -> AuType {
        self.au_type
    }

    pub fn get_name(&self) -> String {
        unsafe {
            let mut name_ref: core_foundation_sys::string::CFStringRef = std::ptr::null();
            let status = AudioComponentCopyName(self.component, &mut name_ref);
            if status != NO_ERR {
                return String::from("<unknown>");
            }
            CfString::from_copied(name_ref)
                .map(|s| s.to_string())
                .unwrap_or_else(|| String::from("<unknown>"))
        }
    }
}

impl Drop for AuHandle {
    fn drop(&mut self) {
        unsafe {
            AudioComponentInstanceDispose(self.instance);
        }
    }
}
