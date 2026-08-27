//! Narrow in-process compiler API for editable Commander Blood game scripts.
//!
//! The recovered compiler implementation remains shared with the reverse-
//! engineering tools. This crate deliberately exposes only the two compilation
//! entry points needed by the modern game's startup verifier.

// These shared modules retain the tools crate's lint/test ownership. Re-linting
// them through this narrow wrapper produces duplicate, context-dependent lints.
#![allow(dead_code, clippy::all)]

#[path = "../../../src/bas_cfg.rs"]
mod bas_cfg;
#[path = "../../../src/bloodscript.rs"]
mod bloodscript;
#[path = "../../../src/descript.rs"]
mod descript;
#[path = "../../../src/descript_source.rs"]
mod descript_source;
#[path = "../../../src/font.rs"]
mod font;
#[path = "../../../src/presentation_catalog.rs"]
mod presentation_catalog;
#[path = "../../../src/script.rs"]
mod script;
#[path = "../../../src/ship3d.rs"]
mod ship3d;
#[path = "../../../src/util.rs"]
mod util;
#[path = "../../../src/vm.rs"]
mod vm;
#[path = "../../../src/vm_cfg.rs"]
mod vm_cfg;
#[path = "../../../src/vm_profile.rs"]
mod vm_profile;
#[path = "../../../src/vm_source.rs"]
mod vm_source;

/// Compile readable DESCRIPT source into the original `DESCRIPT.DES` image.
pub use descript_source::compile as compile_descript;
/// The five compiled images produced from one BloodScript profile.
pub use vm_profile::ProfileImages;
/// Compile one unified BloodScript source into its five original VM images.
pub use vm_profile::compile as compile_profile;
