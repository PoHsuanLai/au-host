use crate::types::*;

#[derive(Debug, Clone)]
pub struct AuParameter {
    pub id: u32,
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub unit: AudioUnitParameterUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioUnitParameterUnit {
    Generic,
    Boolean,
    Percent,
    Seconds,
    Hertz,
    Decibels,
    LinearGain,
    Unknown(u32),
}

impl AudioUnitParameterUnit {
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            K_AUDIO_UNIT_PARAMETER_UNIT_GENERIC => Self::Generic,
            K_AUDIO_UNIT_PARAMETER_UNIT_BOOLEAN => Self::Boolean,
            K_AUDIO_UNIT_PARAMETER_UNIT_PERCENT => Self::Percent,
            K_AUDIO_UNIT_PARAMETER_UNIT_SECONDS => Self::Seconds,
            K_AUDIO_UNIT_PARAMETER_UNIT_HERTZ => Self::Hertz,
            K_AUDIO_UNIT_PARAMETER_UNIT_DECIBELS => Self::Decibels,
            K_AUDIO_UNIT_PARAMETER_UNIT_LINEAR_GAIN => Self::LinearGain,
            other => Self::Unknown(other),
        }
    }
}

impl std::fmt::Display for AudioUnitParameterUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Generic => write!(f, ""),
            Self::Boolean => write!(f, "bool"),
            Self::Percent => write!(f, "%"),
            Self::Seconds => write!(f, "s"),
            Self::Hertz => write!(f, "Hz"),
            Self::Decibels => write!(f, "dB"),
            Self::LinearGain => write!(f, "gain"),
            Self::Unknown(v) => write!(f, "unit({})", v),
        }
    }
}

/// Enumerate all parameters for the given AU instance (global scope).
pub fn get_parameter_list(unit: AudioUnit) -> Vec<AuParameter> {
    let ids = match get_parameter_ids(unit) {
        Some(ids) => ids,
        None => return Vec::new(),
    };

    let mut params = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(p) = query_parameter_info(unit, id) {
            params.push(p);
        }
    }
    params
}

pub fn get_parameter_value(unit: AudioUnit, param_id: u32) -> f32 {
    let mut value: f32 = 0.0;
    unsafe {
        AudioUnitGetParameter(
            unit,
            param_id,
            K_AUDIO_UNIT_SCOPE_GLOBAL,
            0,
            &mut value,
        );
    }
    value
}

pub fn set_parameter_value(unit: AudioUnit, param_id: u32, value: f32) {
    unsafe {
        AudioUnitSetParameter(
            unit,
            param_id,
            K_AUDIO_UNIT_SCOPE_GLOBAL,
            0,
            value,
            0,
        );
    }
}

fn get_parameter_ids(unit: AudioUnit) -> Option<Vec<u32>> {
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
        return None;
    }

    let count = data_size as usize / std::mem::size_of::<u32>();
    let mut ids = vec![0u32; count];
    let mut actual_size = data_size;
    let status = unsafe {
        AudioUnitGetProperty(
            unit,
            K_AUDIO_UNIT_PROPERTY_PARAMETER_LIST,
            K_AUDIO_UNIT_SCOPE_GLOBAL,
            0,
            ids.as_mut_ptr() as *mut std::os::raw::c_void,
            &mut actual_size,
        )
    };
    if status != NO_ERR {
        return None;
    }
    Some(ids)
}

fn query_parameter_info(unit: AudioUnit, param_id: u32) -> Option<AuParameter> {
    let mut info: AudioUnitParameterInfo = unsafe { std::mem::zeroed() };
    let mut size = std::mem::size_of::<AudioUnitParameterInfo>() as u32;

    let status = unsafe {
        AudioUnitGetProperty(
            unit,
            K_AUDIO_UNIT_PROPERTY_PARAMETER_INFO,
            K_AUDIO_UNIT_SCOPE_GLOBAL,
            param_id,
            &mut info as *mut AudioUnitParameterInfo as *mut std::os::raw::c_void,
            &mut size,
        )
    };
    if status != NO_ERR {
        return None;
    }

    let name = if info.flags & K_AUDIO_UNIT_PARAMETER_FLAG_HAS_CF_NAME_STRING != 0
        && !info.name_string.is_null()
    {
        let s = unsafe { cfstring_to_string(info.name_string) };
        unsafe {
            core_foundation_sys::base::CFRelease(
                info.name_string as *const std::os::raw::c_void,
            );
        }
        s
    } else {
        let end = info
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(info.name.len());
        String::from_utf8_lossy(&info.name[..end]).to_string()
    };

    Some(AuParameter {
        id: param_id,
        name,
        min: info.min_value,
        max: info.max_value,
        default: info.default_value,
        unit: AudioUnitParameterUnit::from_raw(info.unit),
    })
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;
    use crate::component::*;

    fn apple_delay_unit() -> AudioUnit {
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
        instance
    }

    #[test]
    fn test_get_parameter_list() {
        let unit = apple_delay_unit();
        let params = get_parameter_list(unit);
        assert!(!params.is_empty(), "AUDelay should have parameters");
        for p in &params {
            eprintln!(
                "  id={} name='{}' [{}, {}] default={} unit={}",
                p.id, p.name, p.min, p.max, p.default, p.unit
            );
        }
        unsafe {
            AudioUnitUninitialize(unit);
            AudioComponentInstanceDispose(unit);
        }
    }

    #[test]
    fn test_get_set_parameter_value() {
        let unit = apple_delay_unit();
        let params = get_parameter_list(unit);
        assert!(!params.is_empty());

        let p = &params[0];
        let mid = (p.min + p.max) / 2.0;
        set_parameter_value(unit, p.id, mid);
        let val = get_parameter_value(unit, p.id);
        assert!(
            (val - mid).abs() < 0.01,
            "Expected ~{}, got {}",
            mid,
            val
        );

        unsafe {
            AudioUnitUninitialize(unit);
            AudioComponentInstanceDispose(unit);
        }
    }
}
