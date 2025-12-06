# Method 4: Point Primitives with Custom Shader

**Date:** 2025-12-06
**Status:** IMPLEMENTING

## Approach

Use GPU point primitives with a custom shader that draws circles at each point position.

## Strategy

1. Create mesh with only vertex positions (vec3 per particle)
2. Use `PrimitiveTopology::PointList`
3. Vertex shader sets `gl_PointSize` for rasterization square
4. Fragment shader uses `gl_PointCoord` to draw circle and discard edges
5. Points automatically face camera (built-in billboarding)

## Expected Benefits

- **Minimal data**: 3 floats per particle vs 12 floats for quads
- **Automatic billboarding**: Points always face camera
- **Simplest geometry**: No quad generation needed
- **Maximum performance**: Single draw call, minimal vertex data

## Implementation Plan

1. Create mesh with positions only, `PrimitiveTopology::PointList`
2. Custom shader:
   - Vertex: set point size based on distance to camera
   - Fragment: draw circle using distance from `gl_PointCoord` center
3. Custom material to bind the shader

## Expected Performance

Should handle 1M+ particles easily with minimal GPU memory.

## Result

**READY FOR TESTING** - Compiles successfully

## Implementation Details

- Single mesh with only vertex positions (3 floats per particle)
- `PrimitiveTopology::PointList` for maximum efficiency
- Custom Material with shader binding for color
- Shader uses Bevy's standard mesh functions for transforms
- For 100k particles: 300k floats (vs 1.2M for quads)

## Code Locations

- Render code: `crates/particles-render/src/lib.rs`
- Shader: `assets/shaders/particle_points.wgsl`

## Important Note

WebGPU/WGSL `PointList` renders 1-pixel points by default. There's no `gl_PointSize` equivalent in WebGPU. Points will be very small.

**If points are too small**, we'll need to switch to Method 5: Geometry shader billboards or compute-based quad generation.

## Issues Encountered

- Had to use `bevy::shader::ShaderRef` import (not `render_resource::ShaderRef`)
- WebGPU doesn't support variable point sizes like OpenGL

## Next Steps

- Test if 1-pixel points are visible
- If too small, implement billboard quads with proper camera-facing orientation
