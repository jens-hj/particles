//! # astra-gui
//!
//! Graphics backend agnostic UI library.
//!
//! This crate provides the core UI primitives and logic with zero dependencies
//! on any specific graphics API. Rendering is handled by separate backend crates
//! like `astra-gui-wgpu`.

mod color;
mod layout;
mod mesh;
mod node;
mod output;
mod primitives;
mod tessellate;

pub use color::*;
pub use layout::*;
pub use mesh::*;
pub use node::*;
pub use output::*;
pub use primitives::*;
pub use tessellate::*;
