//! Text rendering support for the `astra-gui-wgpu` backend.
//!
//! This module is backend-specific by design. The core `astra-gui` crate only produces
//! `Shape::Text` / `TextShape` with layout info; the WGPU backend is responsible for shaping,
//! glyph caching, atlas management, and drawing textured quads.
//!
//! Current structure:
//! - `atlas`: CPU-side glyph atlas placement + cache
//! - `vertex`: GPU vertex format for glyph quads
//!
//! Note: Text shaping/rasterization is handled by the `astra-gui-text` crate,
//! which provides the backend-agnostic text engine using cosmic-text.
//!
//! This module is conditionally compiled behind the `text-cosmic` feature.

#[cfg(feature = "text-cosmic")]
pub mod atlas;

#[cfg(feature = "text-cosmic")]
pub mod vertex;
