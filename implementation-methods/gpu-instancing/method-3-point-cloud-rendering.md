# Method 3: Point Cloud / Billboard Rendering

**Date:** 2025-12-06
**Status:** IMPLEMENTING

## Approach

Use simple point or billboard rendering - each particle is just a point or quad, rendered in a single draw call.

## Strategy

1. Create vertices array with one vertex per particle (point cloud)
2. Or create quad billboards facing camera (4 vertices per particle)
3. Use simple vertex shader to render as GL_POINTS or camera-facing quads
4. Single mesh, single draw call

## Expected Benefits

- Much simpler than full icosphere meshes
- Still single draw call
- Minimal vertex data
- Very fast

## Potential Drawbacks

- Points or billboards less visually impressive than 3D spheres
- But likely acceptable for particle effects

## Implementation Plan

1. Create single mesh with particle positions as vertices
2. Use point primitive topology or quad billboards
3. Simple shader for rendering

## Expected Performance

Should easily handle 1M+ particles.

## Result

**SUCCESS** - Compiles and ready for testing

## Implementation Details

- Used billboard quads (4 vertices per particle)
- Single merged mesh with all particles
- Workaround for Bevy's private `Indices` type: used `duplicate_vertices()` and `compute_flat_normals()`
- Each particle = 1 quad = 4 vertices, 6 indices (2 triangles)
- For 100k particles: 400k vertices, 600k indices
- Single draw call

## Code Location

`crates/particles-render/src/lib.rs` - `setup_billboard_particles()` function

## Issues Encountered

- Had to work around Bevy's private `Indices` enum
- Used `duplicate_vertices()` + `compute_flat_normals()` instead of directly setting indices
- This creates some redundancy but allows the code to compile

## Next Steps

- Test performance with 100k particles
- If performance good, may want to optimize billboard orientation (currently static quads)
