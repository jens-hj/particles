//! # astra-gui
//!
//! Graphics backend agnostic UI library.
//!
//! This crate provides the core UI primitives and logic with zero dependencies
//! on any specific graphics API. Rendering is handled by separate backend crates
//! like `astra-gui-wgpu`.

mod output;
mod primitives;

pub use output::*;
pub use primitives::*;
