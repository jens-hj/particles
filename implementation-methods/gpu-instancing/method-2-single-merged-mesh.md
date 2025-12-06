# Method 2: Single Merged Mesh with Dynamic Vertices

**Date:** 2025-12-06
**Status:** IMPLEMENTING

## Approach

Instead of instancing, create a single large mesh that contains all particle sphere geometries, with positions updated each frame.

## Strategy

1. At startup, create one large mesh by duplicating sphere geometry for each particle
2. Store vertex offsets for each particle's portion of the mesh
3. Each frame, update only the position vertices in the mesh
4. Single mesh = single draw call

## Expected Benefits

- Simpler than custom render pipeline
- Single draw call
- Can use standard Bevy materials
- No complex render phase setup needed

## Potential Drawbacks

- Large mesh data (but still GPU-resident)
- CPU overhead updating vertices each frame
- Less flexible than true instancing

## Implementation Plan

1. Create mesh builder that duplicates icosphere geometry N times
2. Add vertex positions as offsets for each particle
3. Update mesh vertices each frame with particle positions
4. Use single material for entire mesh

## Expected Performance

Should be better than per-entity rendering, but may have CPU overhead from vertex updates.

## Result

**FAILED** - Bevy API limitations

## Issues Encountered

- `bevy::render::mesh::Indices` enum is private in Bevy 0.17
- Cannot directly construct or pattern match on Indices variants
- Cannot easily extract and rebuild index buffers
- API designed for high-level mesh construction, not low-level manipulation
- Would need to work around with `duplicate_vertices()` which defeats the purpose

## Lessons Learned

- Bevy 0.17 doesn't expose low-level mesh index manipulation easily
- Need a different approach that doesn't require direct index buffer manipulation
