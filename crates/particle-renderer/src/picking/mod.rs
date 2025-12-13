//! GPU picking support (offscreen ID pass + readback).
//!
//! This module owns the offscreen target + readback buffer and re-exports the
//! picking renderer types.
//!
//! ID encoding convention (initial):
//! - 0 == "nothing"
//! - Otherwise, the lower 31 bits are an application-defined index,
//!   and the top bit can be used to distinguish entity classes later.

pub mod renderer;

pub use renderer::PickingRenderer;

pub mod overlay;
pub use overlay::PickingOverlay;

use wgpu::util::DeviceExt;

/// Result of a pick, as returned by the GPU readback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickResult {
    /// The raw ID as written by the picking shader.
    pub id: u32,
}

impl PickResult {
    /// True if the pick hit something (non-zero ID).
    pub fn is_hit(&self) -> bool {
        self.id != 0
    }
}

/// Offscreen resources used for GPU picking.
pub struct GpuPicker {
    /// Picking render target view used for ID rendering.
    pub id_texture_view: wgpu::TextureView,
    id_texture: wgpu::Texture,

    /// Buffer used to copy the ID pixel into CPU-visible memory.
    staging: wgpu::Buffer,

    /// Dimensions of the pick target. Kept flexible for future (e.g. NxN region).
    width: u32,
    height: u32,

    /// Texture format used for writing IDs.
    ///
    /// For WebGPU/WGPU portability, we default to `Rgba8Unorm` with bitpacking in shader.
    /// If we later choose to use `R32Uint`/`Rgba8Uint` we should adjust the pipeline
    /// and copy paths accordingly.
    format: wgpu::TextureFormat,
}

impl GpuPicker {
    /// Create a new picker target of the given size.
    ///
    /// `format` should match whatever the picking pipeline writes.
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);

        let id_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Picking ID Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let id_texture_view = id_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // One pixel readback:
        // - For RGBA8 formats, that's 4 bytes.
        // - For R32Uint, that's 4 bytes.
        // Keep it at 4 bytes for now.
        //
        // NOTE: `copy_texture_to_buffer` requires `bytes_per_row` to be aligned to
        // `wgpu::COPY_BYTES_PER_ROW_ALIGNMENT` (256). We satisfy this by always copying into a
        // 256-byte staging buffer and reading only the first 4 bytes.
        let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Picking Readback Buffer"),
            contents: &[0u8; wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize],
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        });

        Self {
            id_texture_view,
            id_texture,
            staging,
            width,
            height,
            format,
        }
    }

    /// Resize the picking render target.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);

        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;

        self.id_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Picking ID Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        self.id_texture_view = self
            .id_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
    }

    /// Copy a single pixel at `(x, y)` from the ID texture into the staging buffer.
    ///
    /// The render pass that writes to `id_texture` must be submitted before this copy
    /// if you expect deterministic results.
    pub fn encode_read_pixel(&self, encoder: &mut wgpu::CommandEncoder, x: u32, y: u32) {
        let x = x.min(self.width.saturating_sub(1));
        let y = y.min(self.height.saturating_sub(1));

        // `bytes_per_row` must be 256-byte aligned, even for 1x1 copies.
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.id_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Map the staging buffer and decode the result into a `PickResult`.
    ///
    /// This is synchronous in the sense that the caller is expected to `poll` the device
    /// appropriately after submitting the encoder that performed the copy.
    ///
    /// The decoding depends on how the shader writes the ID:
    /// - If the picking shader writes `R32Uint`, this reads that as the ID directly.
    /// - If it writes `Rgba8Unorm`, this assumes the shader bit-packed the ID into RGBA8.
    pub fn read_mapped(&self) -> PickResult {
        // NOTE: Caller must ensure:
        // - the buffer is mapped for read
        // - GPU work that writes into it has completed
        // This skeleton intentionally keeps ownership of the async mapping outside the module.

        let slice = self.staging.slice(..);
        let data = slice.get_mapped_range();

        let id = match self.format {
            wgpu::TextureFormat::R32Uint => {
                // 4 bytes: u32
                u32::from_le_bytes(data[0..4].try_into().unwrap())
            }
            _ => {
                // Default: treat first 4 bytes as packed RGBA8 and reconstruct u32.
                // Convention: little-endian packing (r=LSB).
                let r = data[0] as u32;
                let g = data[1] as u32;
                let b = data[2] as u32;
                let a = data[3] as u32;
                r | (g << 8) | (b << 16) | (a << 24)
            }
        };

        PickResult { id }
    }

    /// Access the staging buffer for mapping control (caller-driven).
    pub fn staging_buffer(&self) -> &wgpu::Buffer {
        &self.staging
    }

    /// Debug: expose the underlying picking ID texture.
    ///
    /// This is useful for visualizing the picking layer by sampling from this texture in a
    /// debug render pass. Prefer using `id_texture_view` for most cases; this is only needed
    /// if you need to copy/inspect the raw texture.
    pub fn id_texture(&self) -> &wgpu::Texture {
        &self.id_texture
    }

    /// Debug: expose the picking texture format.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Debug: expose current picking texture dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
