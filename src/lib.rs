//! Audio Unit (AUv2) plugin hosting for macOS.
//!
//! This crate provides low-level bindings to Apple's AudioToolbox framework
//! for hosting AUv2 (Audio Unit version 2) plugins. It follows the same
//! pattern as `vst3-host` and `clap-host` in the Tutti ecosystem.
//!
//! # Platform
//!
//! This crate is macOS-only. All public types and functions are gated behind
//! `#[cfg(target_os = "macos")]`. On other platforms, the crate compiles
//! but exposes no functionality.
//!
//! # Usage
//!
//! ```rust,no_run
//! # #[cfg(target_os = "macos")]
//! # {
//! use au_host::component::{enumerate_components, AuType};
//! use au_host::instance::AuInstance;
//!
//! // Discover all effect AUs
//! let effects = au_host::component::enumerate_components_of_type(AuType::Effect);
//! for info in &effects {
//!     println!("{} by {}", info.name, info.manufacturer);
//! }
//!
//! // Instantiate the first one
//! if let Some(info) = effects.first() {
//!     let mut au = unsafe { AuInstance::new(info.component, 44100.0, 512) }.unwrap();
//!     au.initialize().unwrap();
//!
//!     // Process audio...
//!     let input = vec![vec![0.0f32; 512]; 2];
//!     let mut output = vec![vec![0.0f32; 512]; 2];
//!     let in_refs: Vec<&[f32]> = input.iter().map(|v| v.as_slice()).collect();
//!     let mut out_refs: Vec<&mut [f32]> = output.iter_mut().map(|v| v.as_mut_slice()).collect();
//!     au.process(&in_refs, &mut out_refs, 512).unwrap();
//! }
//! # }
//! ```

#[cfg(target_os = "macos")]
pub mod types;

#[cfg(target_os = "macos")]
pub mod component;

#[cfg(target_os = "macos")]
pub mod instance;

#[cfg(target_os = "macos")]
pub mod parameters;

#[cfg(target_os = "macos")]
pub mod editor;

// Re-export key types at crate root for convenience
#[cfg(target_os = "macos")]
pub use component::{AuComponentInfo, AuType};

#[cfg(target_os = "macos")]
pub use instance::{AuError, AuInstance, AuParameterInfo};

#[cfg(target_os = "macos")]
pub use parameters::AuParameter;

#[cfg(target_os = "macos")]
pub use editor::AuEditor;
