# Astra GUI - Graphics Backend Agnostic UI Library

## Overview

Creating a custom UI library called **astra-gui** that is graphics backend agnostic (like egui). The library will be extractable from this project and work with any graphics backend (WGPU, OpenGL, etc.).

## Architecture: Three-Layer Separation

Following egui's proven pattern:

```
astra-gui/              → Core UI logic (NO graphics dependencies)
    ↓
astra-gui-winit/        → Winit event handling integration
    ↓
astra-gui-wgpu/         → WGPU rendering backend (one of many possible backends)
```

**Key principle:** The core `astra-gui` crate has ZERO dependencies on any graphics API. It only defines shapes, handles layout, and manages UI state. Rendering backends are completely separate.

## Why This Works

- Core library is pure logic - no WGPU, no OpenGL, no Vulkan
- Any graphics backend can implement the renderer trait
- Just like egui: `egui` (core) + `egui-wgpu` or `egui-glow` (backends)
- Genuinely extractable and reusable

## Phase 1: Minimal Implementation

**Goal:** Render a single container with:
- Background color
- Rounded corners
- Configurable fill color/alpha
- Configurable stroke color/alpha/width

This gives us the foundational architecture without complexity.

## Crate Structure

### 1. `astra-gui` (Core - Graphics Agnostic)

**Location:** `crates/astra-gui/`

**Dependencies:**
- `glam` (math primitives - already in workspace)
- `bytemuck` (optional, for Pod trait derives)

**Responsibilities:**
- Define primitives (shapes, colors, strokes)
- UI state management (Context)
- Layout logic (future)
- Widget logic (future)
- Input/output data structures

**Key Files:**
```
crates/astra-gui/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs          # Re-exports
    ├── primitives.rs   # Shape, Color, Stroke
    ├── context.rs      # Context, Ui (future)
    ├── input.rs        # RawInput (future)
    └── output.rs       # FullOutput
```

### 2. `astra-gui-winit` (Platform Integration)

**Location:** `crates/astra-gui-winit/`

**Dependencies:**
- `astra-gui`
- `winit` (already in workspace)

**Responsibilities:**
- Convert winit events to astra-gui RawInput
- Platform-specific clipboard, cursor handling (future)

**Note:** Not needed for Phase 1! We'll add this in Phase 2 when we need interaction.

### 3. `astra-gui-wgpu` (WGPU Backend)

**Location:** `crates/astra-gui-wgpu/`

**Dependencies:**
- `astra-gui`
- `wgpu` (already in workspace)
- `bytemuck`

**Responsibilities:**
- Tessellate shapes to vertices/indices
- Manage WGPU pipelines, buffers, textures
- Render shapes to RenderPass

**Key Files:**
```
crates/astra-gui-wgpu/
├── Cargo.toml
└── src/
    ├── lib.rs          # Renderer
    ├── tessellator.rs  # Shape → Mesh conversion
    └── shaders/
        └── ui.wgsl     # Vertex + fragment shader
```

## Phase 1 Implementation Details

### Core Types (`astra-gui`)

```rust
// primitives.rs

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Stroke {
    pub width: f32,
    pub color: Color,
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Clone, Debug)]
pub struct RoundedRect {
    pub rect: Rect,
    pub rounding: f32,        // Corner radius
    pub fill: Color,
    pub stroke: Option<Stroke>,
}

pub enum Shape {
    RoundedRect(RoundedRect),
    // Future: Circle, Line, Mesh, etc.
}

pub struct ClippedShape {
    pub clip_rect: Rect,
    pub shape: Shape,
}
```

```rust
// output.rs

pub struct FullOutput {
    pub shapes: Vec<ClippedShape>,
}
```

### Tessellation (`astra-gui-wgpu`)

**Strategy:** CPU-based tessellation of rounded rectangles

```rust
// tessellator.rs

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],  // RGBA in linear space
}

pub struct Tessellator {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Tessellator {
    pub fn tessellate(&mut self, shapes: &[ClippedShape]) -> (&[Vertex], &[u32]) {
        self.vertices.clear();
        self.indices.clear();
        
        for clipped in shapes {
            if let Shape::RoundedRect(r) = &clipped.shape {
                self.tessellate_rounded_rect(r);
            }
        }
        
        (&self.vertices, &self.indices)
    }
    
    fn tessellate_rounded_rect(&mut self, rect: &RoundedRect) {
        // Generate vertices for rounded corners
        // Fill: Triangle fan or indexed triangles
        // Stroke: Quad strip around perimeter
        
        let base_idx = self.vertices.len() as u32;
        
        // Fill
        if rect.fill.a > 0.0 {
            self.add_rounded_rect_fill(rect, base_idx);
        }
        
        // Stroke
        if let Some(stroke) = &rect.stroke {
            if stroke.width > 0.0 && stroke.color.a > 0.0 {
                self.add_rounded_rect_stroke(rect, stroke);
            }
        }
    }
}
```

### Shader (`astra-gui-wgpu`)

```wgsl
// shaders/ui.wgsl

struct Uniforms {
    screen_size: vec2<f32>,
}

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Convert screen-space pixels to NDC [-1, 1]
    let ndc = (in.pos / uniforms.screen_size) * 2.0 - 1.0;
    out.position = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);
    out.color = in.color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
```

### Renderer (`astra-gui-wgpu`)

```rust
// lib.rs

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    tessellator: Tessellator,
    vertex_capacity: usize,
    index_capacity: usize,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        // Create pipeline, buffers, etc.
    }
    
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        screen_width: f32,
        screen_height: f32,
        output: &FullOutput,
    ) {
        // 1. Tessellate shapes
        let (vertices, indices) = self.tessellator.tessellate(&output.shapes);
        
        // 2. Upload vertex/index data
        if vertices.len() > self.vertex_capacity {
            // Recreate larger buffer
        }
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(indices));
        
        // 3. Update uniforms
        let uniforms = [screen_width, screen_height];
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));
        
        // 4. Render
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,  // Preserve scene rendering
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
    }
}
```

## Integration with Particles Project

### Workspace Setup

Update `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/particle-physics",
    "crates/particle-simulation",
    "crates/particle-renderer",
    "crates/astra-gui",        # NEW
    "crates/astra-gui-wgpu",   # NEW
]

[workspace.dependencies]
# ... existing ...
astra-gui = { path = "crates/astra-gui" }
astra-gui-wgpu = { path = "crates/astra-gui-wgpu" }
```

### Main Binary Integration

Keep egui for now, add astra-gui alongside for testing:

```rust
// src/main.rs

use astra_gui::{Shape, RoundedRect, Rect, Color, Stroke, FullOutput};
use astra_gui_wgpu::Renderer as AstraRenderer;

// In initialization:
let mut astra_renderer = AstraRenderer::new(&device, surface_format);

// In render loop (after scene, after egui):
let astra_output = FullOutput {
    shapes: vec![
        ClippedShape {
            clip_rect: Rect { min: [0.0, 0.0], max: [1920.0, 1080.0] },
            shape: Shape::RoundedRect(RoundedRect {
                rect: Rect { min: [100.0, 100.0], max: [300.0, 250.0] },
                rounding: 10.0,
                fill: Color { r: 0.2, g: 0.4, b: 0.8, a: 0.9 },
                stroke: Some(Stroke {
                    width: 2.0,
                    color: Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
                }),
            }),
        },
    ],
};

astra_renderer.render(
    &device,
    &queue,
    &mut encoder,
    &view,
    window.inner_size().width as f32,
    window.inner_size().height as f32,
    &astra_output,
);
```

## Text Rendering (Future)

**What egui uses:** Custom text rendering system:
- `epaint` crate has its own font rasterization
- Uses `ab_glyph` for font loading and shaping
- Maintains its own texture atlas for glyphs
- CPU-based tessellation of text into textured quads

**For astra-gui:** We'll implement text in a later phase. Options:
1. Use `ab_glyph` like egui (proven, simple)
2. Use `glyphon` (more modern, WGPU-optimized)
3. Custom solution

But for Phase 1, we don't need text at all!

## Implementation Steps

### Step 1: Create Crate Structure
```bash
mkdir -p crates/astra-gui/src
mkdir -p crates/astra-gui-wgpu/src/shaders
```

### Step 2: Set Up Cargo.toml Files
- Define dependencies for each crate
- Add to workspace

### Step 3: Implement Core Types
- `astra-gui/src/primitives.rs` - Color, Stroke, Rect, RoundedRect, Shape
- `astra-gui/src/output.rs` - FullOutput, ClippedShape

### Step 4: Implement WGPU Backend
- `astra-gui-wgpu/src/tessellator.rs` - Rounded rect tessellation
- `astra-gui-wgpu/src/shaders/ui.wgsl` - Shader
- `astra-gui-wgpu/src/lib.rs` - Renderer

### Step 5: Integrate and Test
- Add to main.rs
- Render test container
- Verify rendering works alongside egui

### Step 6: Validate
- Check `cargo check`
- Run `cargo run`
- Verify no warnings
- Commit with conventional commit message

## Success Criteria for Phase 1

- [ ] Three crates compile without errors or warnings
- [ ] Can render a rounded rectangle container with:
  - Custom background color with alpha
  - Rounded corners (configurable radius)
  - Optional stroke with color, width, and alpha
- [ ] Renders correctly at different window sizes
- [ ] Works alongside existing egui UI
- [ ] Core `astra-gui` has zero graphics dependencies
- [ ] Clean architecture ready for future backends (OpenGL, etc.)

## Future Phases (Not Part of Phase 1)

- **Phase 2:** Input handling (astra-gui-winit) + interaction
- **Phase 3:** Layout system (stacking, spacing, padding)
- **Phase 4:** Basic widgets (Label, Button)
- **Phase 5:** Text rendering integration
- **Phase 6:** Advanced widgets (Slider, Checkbox)
- **Phase 7:** Optimization (batching, GPU tessellation)

## Why "astra-gui"?

- "Astra" means "star" in Latin - fits with particle/physics theme
- Short, memorable, available name
- Backend-agnostic architecture like stars visible from any vantage point

---

This plan focuses on building the minimal viable foundation with proper architecture, making it trivial to add features incrementally while maintaining extractability.
