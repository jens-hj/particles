# Method 1: Custom Material with Bevy's Material Trait

**Date:** 2025-12-06
**Status:** IMPLEMENTING

## Approach

Create a custom `ParticleMaterial` implementing Bevy's `Material` trait with custom vertex buffer layout for instance data.

## Strategy

1. Create `ParticleMaterial` struct with `AsBindGroup` derive
2. Implement `Material` trait to specify custom shader
3. Set up custom vertex buffer layout with instance attributes
4. Store particle positions in dedicated instance buffer
5. Render single icosphere mesh with instancing enabled

## Expected Benefits

- Integrates with Bevy's rendering infrastructure
- Less boilerplate than full custom render pipeline
- Material system handles pipeline creation and management
- Can leverage existing Bevy materials ecosystem

## Implementation Plan

1. Modify `particles-render` crate to use custom material
2. Configure vertex buffer layout: `@location(0)` = mesh vertex, `@location(1)` = instance position
3. Use `particle_instanced.wgsl` shader
4. Single draw call for all particles

## Expected Performance

Should handle 1M+ particles at 60+ FPS with single draw call.

## Result

**PAUSED** - Complexity too high, trying simpler approach first

## Issues Encountered

- Bevy 0.17's Material trait doesn't provide clean access to custom instance buffer vertex layouts
- Implementing full custom render pipeline requires:
  - Custom render phases
  - Manual pipeline creation
  - Extraction systems
  - Complex boilerplate (~300+ lines)
- This approach is possible but very complex for this use case

## Notes

- Existing shader `assets/shaders/particle_instanced.wgsl` already prepared for this approach
- Will need to ensure instance buffer is properly uploaded to GPU
