# Method 5: Compute Shader Quad Generation

**Date:** 2025-12-06
**Status:** IMPLEMENTING

## Approach

Use a compute shader to expand particle positions into camera-facing billboard quads on the GPU.

## Strategy

1. **Compute shader**: Read particle positions, camera direction
2. For each particle: generate 4 vertices forming a camera-facing quad
3. Write vertices to output buffer (with positions, UVs)
4. **Vertex shader**: Pass through the generated vertices
5. **Fragment shader**: Draw circles using UV coordinates (discard corners)

## Pipeline

```
Particle positions (storage buffer)
  ↓
Compute Shader (expands to quads)
  ↓
Vertex buffer (4 vertices × N particles)
  ↓
Vertex Shader (pass through)
  ↓
Fragment Shader (draw circles)
```

## Expected Benefits

- All GPU-side (no CPU overhead)
- Camera-facing billboards
- Circles with proper radius
- Single draw call for all particles

## Implementation Steps

1. Create compute shader for quad generation
2. Set up storage buffers (input: positions, output: vertices)
3. Dispatch compute shader before rendering
4. Render generated vertices as triangles
5. Fragment shader with circle masking

## Expected Performance

Excellent - all GPU-parallel processing. Should handle 1M+ particles.

## Result

**IN PROGRESS** - Buffer management complete, pipelines next

## Progress

✅ Phase 1: Buffer Management & Bind Groups (DONE)
- GpuParticleBuffer (input positions)
- GpuVertexBuffer (output quads)
- GpuCameraBuffer (view/projection)
- GpuParticleSizeBuffer (uniform)
- Bind group creation
- Resource extraction from main → render world

🚧 Phase 2: Compute Pipeline & Dispatch (IN PROGRESS)
- Pipeline creation
- Shader loading
- Compute dispatch logic

⏳ Phase 3: Render Pipeline (TODO)
- Render pipeline setup
- Draw call implementation

⏳ Phase 4: Render Graph Integration (TODO)

## Issues Encountered

- Bevy 0.17 API changes: `RenderSet` → `RenderSystems`, `compute_matrix()` → `to_matrix()`, `get_single()` → `single()`
- Need `#[repr(C)]` + `bytemuck::Pod` for GPU uniforms
- Working around private `Indices` type from earlier attempts

## Code Location

`crates/particles-render/src/compute_billboard.rs` (272 lines so far)
