# Particles - Refactoring & Optimization TODO

## Overview
This document tracks refactoring and optimization tasks identified through codebase analysis and comparison with industry best practices.

**Analysis Date**: 2024-12-14
**Current Version**: 0.3.2

---

## High Priority - Performance Improvements

### 1. Replace Vec with VecDeque for Frame Timing
- **File**: `src/main.rs:613-615`
- **Issue**: `Vec::remove(0)` is O(n), shifting 100 elements every frame
- **Solution**: Use `VecDeque::push_back()` and `pop_front()` for O(1) operations
- **Impact**: Minor performance improvement in frame time tracking
- **Reference**: [VecDeque Documentation](https://doc.rust-lang.org/std/collections/struct.VecDeque.html)
- **Status**: ⬜ Pending

**Current Code**:
```rust
self.frame_times.push(frame_time);
if self.frame_times.len() > 100 {
    self.frame_times.remove(0);  // O(n)
}
```

**Target Code**:
```rust
self.frame_times.push_back(frame_time);
if self.frame_times.len() > 100 {
    self.frame_times.pop_front();  // O(1)
}
```

---

### 2. Implement Ring Buffer Pattern for GPU Readbacks
- **File**: `src/main.rs:185-286`
- **Issue**: Blocking `poll(Wait)` calls stall GPU pipeline; creates temporary 112KB buffers
- **Solution**: Pre-allocate 3-4 staging buffers in a ring, rotate between them
- **Impact**: High - reduces GPU-CPU synchronization stalls
- **Reference**: [wgpu-async Library](https://github.com/lucentflux/wgpu-async), [WebGPU Best Practices](https://toji.dev/webgpu-best-practices/buffer-uploads)
- **Status**: ⬜ Pending

**Current Pattern**:
```rust
let multi_nucleus_staging = self.device.create_buffer(...);  // Created every call
device.poll(wgpu::Maintain::Wait);  // Blocking
```

**Target Pattern**:
- Allocate 3 staging buffers at initialization
- Track current buffer index, rotate on each readback
- Use `map_async` with callbacks instead of blocking `poll(Wait)`

---

### 3. Add Dirty Flag Tracking for PhysicsParams
- **File**: `src/main.rs:628`
- **Issue**: Updates uniform buffer every frame even when params unchanged
- **Solution**: Track changes in `PhysicsParams`, only call `update_params()` when dirty
- **Impact**: Medium - reduces unnecessary GPU buffer writes
- **Reference**: [WebGPU Uniforms Guide](https://webgpufundamentals.org/webgpu/lessons/webgpu-uniforms.html)
- **Status**: ⬜ Pending

**Implementation**:
```rust
// In PhysicsParams or UiState
pub struct PhysicsParams {
    // ... existing fields ...
    dirty: bool,
}

// In render loop
if self.ui_state.physics_params.is_dirty() {
    self.simulation.update_params(&self.ui_state.physics_params);
    self.ui_state.physics_params.clear_dirty();
}
```

---

## Medium Priority - Code Quality

### 4. Remove Empty compute.rs File
- **File**: `crates/particle-simulation/src/compute.rs`
- **Issue**: 0-byte placeholder file from earlier refactoring
- **Solution**: Delete the file
- **Impact**: Low - code cleanup
- **Status**: ⬜ Pending

---

### 5. Extend Element Lookup Tables to All 118 Elements
- **File**: `src/gui.rs:555-605`
- **Issue**: Element name/symbol tables only cover elements 1-20, but physics engine supports all 118
- **Solution**: Complete the periodic table data
- **Impact**: Medium - UI matches engine capabilities
- **Reference**: README mentions "CPK coloring for all 118 elements"
- **Status**: ⬜ Pending

**Current Coverage**: Elements 1-20 (H through Ca)
**Target Coverage**: Elements 1-118 (H through Og)

**Implementation Options**:
1. Extend existing match statements
2. Use const arrays for O(1) lookups
3. Use `phf` crate for compile-time perfect hash maps

---

### 6. Optimize Nucleus Data Readback
- **File**: `src/main.rs:189`
- **Issue**: Hardcoded search limit of 1000 nuclei, linear search
- **Solution**:
  - Cache selected nucleus data with timestamp
  - Implement GPU-side index lookup (sort nuclei by anchor_hadron_index)
  - Dynamic search capacity based on actual nucleus count
- **Impact**: Medium - reduces readback overhead
- **Status**: ⬜ Pending

---

## Experimental - Profiling & Testing

### 7. Profile Workgroup Size Alternatives
- **File**: `crates/particle-simulation/src/shaders/*.wgsl`
- **Current**: `@workgroup_size(256)` - industry standard, good choice
- **Experiment**: Test with 512 and 1024 for potential GPU occupancy improvements
- **Impact**: Potential 5-10% performance gain (GPU-dependent)
- **Reference**: [WebGPU Compute Shaders](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html)
- **Status**: ⬜ Pending

**Note**: Current 256 is optimal for most GPUs. This is purely experimental.

---

## Additional Considerations

### Unused Buffer Cleanup
- **File**: `src/main.rs:115-116, 448`
- **Item**: `_selected_nucleus_staging_buffer` is pre-allocated but unused (prefixed with `_`)
- **Decision**: Either remove or repurpose for ring-buffering (Task #2)

### Parameter Update Frequency
- Related to Task #3
- Consider batching multiple parameter changes into single GPU write

### Element Data Structure
- For Task #5, consider creating a shared `periodic_table` crate if data grows large
- Could include: symbols, names, CPK colors, atomic masses, electron configs

---

## Verification Checklist

After each change:
- [ ] Run `cargo check` - ensure no compilation warnings
- [ ] Run `cargo run` - visual inspection of behavior
- [ ] Profile performance impact (if applicable)
- [ ] Create commit with conventional commit message

---

## Implementation Order

**Recommended sequence**:
1. Task #4 (Remove empty file) - Quick win, no testing needed
2. Task #1 (VecDeque) - Simple, low-risk performance improvement
3. Task #3 (Dirty flag) - Medium complexity, clear benefit
4. Task #5 (Element tables) - Independent, improves UI completeness
5. Task #6 (Nucleus optimization) - Builds on understanding from Task #2
6. Task #2 (Ring buffers) - Most complex, highest impact
7. Task #7 (Workgroup profiling) - Experimental, requires benchmarking

---

## Notes from Codebase Analysis

### Strengths ✓
- Zero compilation warnings
- Excellent crate decomposition (physics/simulation/renderer)
- Proper GPU resource management
- Correct physics integration (Velocity Verlet)
- Well-structured shader code

### Architecture Quality
- Main crate is appropriately minimal
- Clean separation of concerns
- All dependencies up-to-date (wgpu 27.0, egui 0.33, etc.)
- No unsafe code in hot paths

### Performance Characteristics
- GPU-accelerated N-body simulation
- 8000 particles with real-time interaction
- Multiple LOD systems (shell_fade, bond_fade, etc.)
- Efficient force computation with range cutoffs

---

**Last Updated**: 2024-12-14
