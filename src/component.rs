//! Audio Component discovery and enumeration.

#[cfg(target_os = "macos")]
use crate::cf::CfString;
#[cfg(target_os = "macos")]
use crate::types::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuType {
    Effect,
    Instrument,
    Generator,
    MusicEffect,
    Mixer,
    Converter,
    Output,
    MidiProcessor,
    Unknown(u32),
}

impl AuType {
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

    #[cfg(target_os = "macos")]
    pub fn to_raw(self) -> u32 {
        match self {
            AuType::Effect => K_AUDIO_UNIT_TYPE_EFFECT,
            AuType::Instrument => K_AUDIO_UNIT_TYPE_MUSIC_DEVICE,
            AuType::Generator => K_AUDIO_UNIT_TYPE_GENERATOR,
            AuType::MusicEffect => K_AUDIO_UNIT_TYPE_MUSIC_EFFECT,
            AuType::Mixer => K_AUDIO_UNIT_TYPE_MIXER,
            AuType::Converter => K_AUDIO_UNIT_TYPE_FORMAT_CONVERTER,
            AuType::Output => K_AUDIO_UNIT_TYPE_OUTPUT,
            AuType::MidiProcessor => K_AUDIO_UNIT_TYPE_MIDI_PROCESSOR,
            AuType::Unknown(code) => code,
        }
    }

    pub fn receives_midi(&self) -> bool {
        matches!(
            self,
            AuType::Instrument | AuType::MusicEffect | AuType::MidiProcessor
        )
    }
}

impl std::fmt::Display for AuType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuType::Effect => write!(f, "Effect"),
            AuType::Instrument => write!(f, "Instrument"),
            AuType::Generator => write!(f, "Generator"),
            AuType::MusicEffect => write!(f, "MusicEffect"),
            AuType::Mixer => write!(f, "Mixer"),
            AuType::Converter => write!(f, "Converter"),
            AuType::Output => write!(f, "Output"),
            AuType::MidiProcessor => write!(f, "MidiProcessor"),
            #[cfg(target_os = "macos")]
            AuType::Unknown(code) => write!(f, "Unknown({})", fourcc_to_string(*code)),
            #[cfg(not(target_os = "macos"))]
            AuType::Unknown(code) => write!(f, "Unknown({code:#x})"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuComponentInfo {
    pub name: String,
    pub manufacturer: String,
    pub manufacturer_code: u32,
    pub sub_type: u32,
    pub component_type: AuType,
    #[cfg(target_os = "macos")]
    pub component: AudioComponent,
}

#[cfg(target_os = "macos")]
fn enumerate_with_desc(desc: AudioComponentDescription) -> Vec<AuComponentInfo> {
    let mut results = Vec::new();
    let mut component: AudioComponent = std::ptr::null_mut();
    loop {
        component = unsafe { AudioComponentFindNext(component, &desc) };
        if component.is_null() {
            break;
        }
        if let Some(info) = component_info(component) {
            results.push(info);
        }
    }
    results
}

#[cfg(target_os = "macos")]
pub fn enumerate_components() -> Vec<AuComponentInfo> {
    enumerate_with_desc(AudioComponentDescription::default())
}

#[cfg(target_os = "macos")]
pub fn enumerate_components_of_type(au_type: AuType) -> Vec<AuComponentInfo> {
    enumerate_with_desc(AudioComponentDescription {
        component_type: au_type.to_raw(),
        ..Default::default()
    })
}

#[cfg(target_os = "macos")]
pub fn find_component(desc: &AudioComponentDescription) -> Option<AudioComponent> {
    let component = unsafe { AudioComponentFindNext(std::ptr::null_mut(), desc) };
    (!component.is_null()).then_some(component)
}

#[cfg(target_os = "macos")]
fn component_info(component: AudioComponent) -> Option<AuComponentInfo> {
    let name = unsafe {
        let mut name_ref: core_foundation_sys::string::CFStringRef = std::ptr::null();
        let status = AudioComponentCopyName(component, &mut name_ref);
        if status == NO_ERR {
            CfString::from_copied(name_ref)
                .map(|s| s.to_string())
                .unwrap_or_else(|| String::from("<unknown>"))
        } else {
            String::from("<unknown>")
        }
    };

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
        assert!(AuType::MusicEffect.receives_midi());
        assert!(AuType::MidiProcessor.receives_midi());
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
            let back = AuType::from_raw(ty.to_raw());
            assert_eq!(
                std::mem::discriminant(ty),
                std::mem::discriminant(&back),
                "Roundtrip failed for {ty:?}"
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_enumerate_components() {
        let components = enumerate_components();
        assert!(
            !components.is_empty(),
            "Expected at least one Audio Unit on the system"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_enumerate_effects() {
        let effects = enumerate_components_of_type(AuType::Effect);
        assert!(!effects.is_empty(), "Expected at least one Effect AU");
        for c in &effects {
            assert_eq!(c.component_type, AuType::Effect);
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_find_apple_au_delay() {
        let desc = AudioComponentDescription {
            component_type: K_AUDIO_UNIT_TYPE_EFFECT,
            component_sub_type: u32::from_be_bytes(*b"dely"),
            component_manufacturer: u32::from_be_bytes(*b"appl"),
            component_flags: 0,
            component_flags_mask: 0,
        };
        assert!(find_component(&desc).is_some());
    }
}
