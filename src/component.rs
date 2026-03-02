//! Audio Component discovery and enumeration.
//!
//! Wraps `AudioComponentFindNext` to enumerate all AUv2 plugins on the system
//! and provides typed component info.

#[cfg(target_os = "macos")]
use crate::types::*;

/// The type/category of an Audio Unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuType {
    /// Audio effect (e.g., reverb, delay, EQ). Type code: `aufx`.
    Effect,
    /// Software instrument / synthesizer. Type code: `aumu`.
    Instrument,
    /// Audio generator (e.g., noise, tone). Type code: `augn`.
    Generator,
    /// Music device (alias for Instrument). Type code: `aumu`.
    MusicDevice,
    /// Music effect (instrument + effect hybrid). Type code: `aumf`.
    MusicEffect,
    /// Mixer. Type code: `aumx`.
    Mixer,
    /// Format converter. Type code: `aufc`.
    Converter,
    /// Output unit. Type code: `auou`.
    Output,
    /// MIDI processor. Type code: `aumi`.
    MidiProcessor,
    /// Unknown type.
    Unknown(u32),
}

impl AuType {
    /// Convert from the raw four-char-code component type.
    #[cfg(target_os = "macos")]
    pub fn from_raw(component_type: u32) -> Self {
        match component_type {
            K_AUDIO_UNIT_TYPE_EFFECT => AuType::Effect,
            K_AUDIO_UNIT_TYPE_MUSIC_DEVICE => AuType::Instrument,
            K_AUDIO_UNIT_TYPE_GENERATOR => AuType::Generator,
            K_AUDIO_UNIT_TYPE_MUSIC_EFFECT => AuType::MusicEffect,
            K_AUDIO_UNIT_TYPE_MIXER => AuType::Mixer,
            K_AUDIO_UNIT_TYPE_FORMAT_CONVERTER => AuType::Converter,
            K_AUDIO_UNIT_TYPE_OUTPUT => AuType::Output,
            K_AUDIO_UNIT_TYPE_MIDI_PROCESSOR => AuType::MidiProcessor,
            other => AuType::Unknown(other),
        }
    }

    /// Convert to the raw four-char-code component type.
    #[cfg(target_os = "macos")]
    pub fn to_raw(self) -> u32 {
        match self {
            AuType::Effect => K_AUDIO_UNIT_TYPE_EFFECT,
            AuType::Instrument | AuType::MusicDevice => K_AUDIO_UNIT_TYPE_MUSIC_DEVICE,
            AuType::Generator => K_AUDIO_UNIT_TYPE_GENERATOR,
            AuType::MusicEffect => K_AUDIO_UNIT_TYPE_MUSIC_EFFECT,
            AuType::Mixer => K_AUDIO_UNIT_TYPE_MIXER,
            AuType::Converter => K_AUDIO_UNIT_TYPE_FORMAT_CONVERTER,
            AuType::Output => K_AUDIO_UNIT_TYPE_OUTPUT,
            AuType::MidiProcessor => K_AUDIO_UNIT_TYPE_MIDI_PROCESSOR,
            AuType::Unknown(code) => code,
        }
    }

    /// Returns true if this AU type accepts MIDI input.
    pub fn receives_midi(&self) -> bool {
        matches!(
            self,
            AuType::Instrument
                | AuType::MusicDevice
                | AuType::MusicEffect
                | AuType::MidiProcessor
        )
    }
}

impl std::fmt::Display for AuType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuType::Effect => write!(f, "Effect"),
            AuType::Instrument => write!(f, "Instrument"),
            AuType::Generator => write!(f, "Generator"),
            AuType::MusicDevice => write!(f, "MusicDevice"),
            AuType::MusicEffect => write!(f, "MusicEffect"),
            AuType::Mixer => write!(f, "Mixer"),
            AuType::Converter => write!(f, "Converter"),
            AuType::Output => write!(f, "Output"),
            AuType::MidiProcessor => write!(f, "MidiProcessor"),
            AuType::Unknown(code) => write!(f, "Unknown({})", fourcc_to_string(*code)),
        }
    }
}

/// Information about a discovered Audio Unit component.
#[derive(Debug, Clone)]
pub struct AuComponentInfo {
    /// Display name of the plugin.
    pub name: String,
    /// Manufacturer four-char code as a string.
    pub manufacturer: String,
    /// Manufacturer four-char code (raw).
    pub manufacturer_code: u32,
    /// Sub-type four-char code (raw).
    pub sub_type: u32,
    /// AU type category.
    pub component_type: AuType,
    /// Opaque handle to the component (valid for the lifetime of the process).
    #[cfg(target_os = "macos")]
    pub component: AudioComponent,
}

#[cfg(target_os = "macos")]
fn enumerate_with_desc(desc: AudioComponentDescription) -> Vec<AuComponentInfo> {
    let mut results = Vec::new();
    let mut component: AudioComponent = std::ptr::null_mut();
    loop {
        component = unsafe { AudioComponentFindNext(component, &desc) };
        if component.is_null() { break; }
        if let Some(info) = component_info(component) { results.push(info); }
    }
    results
}

/// Enumerate all Audio Unit components on the system.
///
/// This finds every AUv2 plugin registered with the system by iterating
/// through `AudioComponentFindNext` with a wildcard description.
#[cfg(target_os = "macos")]
pub fn enumerate_components() -> Vec<AuComponentInfo> {
    enumerate_with_desc(AudioComponentDescription::default())
}

/// Enumerate Audio Unit components of a specific type.
#[cfg(target_os = "macos")]
pub fn enumerate_components_of_type(au_type: AuType) -> Vec<AuComponentInfo> {
    enumerate_with_desc(AudioComponentDescription {
        component_type: au_type.to_raw(),
        ..Default::default()
    })
}

/// Find a specific Audio Unit component by its full description.
#[cfg(target_os = "macos")]
pub fn find_component(desc: &AudioComponentDescription) -> Option<AudioComponent> {
    let component = unsafe { AudioComponentFindNext(std::ptr::null_mut(), desc) };
    if component.is_null() {
        None
    } else {
        Some(component)
    }
}

/// Extract info from a single AudioComponent handle.
#[cfg(target_os = "macos")]
fn component_info(component: AudioComponent) -> Option<AuComponentInfo> {
    // Get the name
    let mut name_ref: core_foundation_sys::string::CFStringRef = std::ptr::null();
    let status = unsafe { AudioComponentCopyName(component, &mut name_ref) };
    let name = if status == NO_ERR && !name_ref.is_null() {
        let s = unsafe { cfstring_to_string(name_ref) };
        // AudioComponentCopyName uses the Copy rule, so we must release
        unsafe {
            core_foundation_sys::base::CFRelease(name_ref as *const std::os::raw::c_void);
        }
        s
    } else {
        String::from("<unknown>")
    };

    // Get the description to extract type/subtype/manufacturer
    let mut comp_desc = AudioComponentDescription::default();
    let status = unsafe { AudioComponentGetDescription(component, &mut comp_desc) };
    if status != NO_ERR {
        return None;
    }

    Some(AuComponentInfo {
        name,
        manufacturer: fourcc_to_string(comp_desc.component_manufacturer),
        manufacturer_code: comp_desc.component_manufacturer,
        sub_type: comp_desc.component_sub_type,
        component_type: AuType::from_raw(comp_desc.component_type),
        component,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_au_type_display() {
        assert_eq!(AuType::Effect.to_string(), "Effect");
        assert_eq!(AuType::Instrument.to_string(), "Instrument");
        assert_eq!(AuType::Generator.to_string(), "Generator");
    }

    #[test]
    fn test_au_type_receives_midi() {
        assert!(AuType::Instrument.receives_midi());
        assert!(AuType::MusicDevice.receives_midi());
        assert!(AuType::MusicEffect.receives_midi());
        assert!(!AuType::Effect.receives_midi());
        assert!(!AuType::Generator.receives_midi());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_au_type_roundtrip() {
        let types = [
            AuType::Effect,
            AuType::Instrument,
            AuType::Generator,
            AuType::MusicEffect,
            AuType::Mixer,
        ];
        for ty in &types {
            let raw = ty.to_raw();
            let back = AuType::from_raw(raw);
            assert_eq!(
                std::mem::discriminant(ty),
                std::mem::discriminant(&back),
                "Roundtrip failed for {:?}",
                ty
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_enumerate_components() {
        let components = enumerate_components();
        // macOS ships with built-in AUs, so we should find some
        assert!(
            !components.is_empty(),
            "Expected at least one Audio Unit on the system"
        );

        // Print first few for debugging
        for (i, c) in components.iter().take(5).enumerate() {
            eprintln!(
                "  [{}] {} (type={}, manufacturer={})",
                i, c.name, c.component_type, c.manufacturer
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_enumerate_effects() {
        let effects = enumerate_components_of_type(AuType::Effect);
        // macOS ships with Apple effects (AUBandpass, AUDelay, etc.)
        assert!(
            !effects.is_empty(),
            "Expected at least one Effect AU on macOS"
        );
        for c in &effects {
            assert_eq!(
                c.component_type,
                AuType::Effect,
                "All results should be Effect type"
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_find_apple_au_delay() {
        // Apple's built-in AUDelay should always be present
        let desc = AudioComponentDescription {
            component_type: K_AUDIO_UNIT_TYPE_EFFECT,
            component_sub_type: u32::from_be_bytes(*b"dely"),
            component_manufacturer: u32::from_be_bytes(*b"appl"),
            component_flags: 0,
            component_flags_mask: 0,
        };

        let component = find_component(&desc);
        assert!(
            component.is_some(),
            "Apple's AUDelay should be present on macOS"
        );
    }
}
