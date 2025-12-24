//! # astra-gui
//!
//! Graphics backend agnostic UI library.
//!
//! This crate provides the core UI primitives and logic with zero dependencies
//! on any specific graphics API. Rendering is handled by separate backend crates
//! like `astra-gui-wgpu`.

mod color;
mod content;
mod debug;
mod hit_test;
mod layout;
mod measure;
mod mesh;
mod node;
mod output;
mod primitives;
mod style;
mod tessellate;
pub mod transition;

pub use color::*;
pub use content::*;
pub use debug::*;
pub use hit_test::*;
pub use layout::*;
pub use measure::*;
pub use mesh::*;
pub use node::*;
pub use output::*;
pub use primitives::*;
pub use style::*;
pub use tessellate::*;
pub use transition::*;
