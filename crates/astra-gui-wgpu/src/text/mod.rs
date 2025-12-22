//! Text rendering support for the `astra-gui-wgpu` backend.
//!
//! This module is backend-specific by design. The core `astra-gui` crate only produces
//! `Shape::Text` / `TextShape` with layout info; the WGPU backend is responsible for shaping,
//! glyph caching, atlas management, and drawing textured quads.
//!
//! Current structure:
//! - `atlas`: CPU-side glyph atlas placement + cache
//! - `cosmic`: shaping/rasterization via `cosmic-text`
//! - `vertex`: GPU vertex format for glyph quads
//!
//! This module is conditionally compiled behind the `text-cosmic` feature.
//!
//! NOTE: We intentionally do not `pub use` re-exports yet to avoid unused import warnings
//! until the renderer is fully wired up.

#[cfg(feature = "text-cosmic")]
pub mod atlas;

#[cfg(feature = "text-cosmic")]
pub mod cosmic;

#[cfg(feature = "text-cosmic")]
pub mod vertex;
