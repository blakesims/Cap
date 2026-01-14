# Task: [T002] - Export Performance Optimization

## 0. Task Summary
-  **Task Name:** Export Performance Optimization
-  **Priority:** 3
-  **Number of Stories:** 5
-  **Current Status:** ACTIVE
-  **Platform:** macOS only (Windows feasible as future task)
-  **Dependencies:** `crates/export/`, `crates/rendering/`, `crates/enc-ffmpeg/`, `crates/gpu-converters/`, `crates/frame-converter/`
-  **Rules Required:** CLAUDE.md (no comments, Rust clippy rules)
-  **Executor Ref:** See Stories S01-S05
-  **Acceptance Criteria:**
    - Measurable export speed improvement (target: 35-55% faster for 4K)
    - No regression in output quality
    - No increase in memory usage beyond acceptable bounds (~250MB)
    - Graceful handling of low-memory conditions
    - All existing export tests pass

## 1. Goal / Objective
Improve video export speed on macOS by addressing identified bottlenecks in the export pipeline, primarily by moving RGBA→NV12 format conversion to GPU before readback, and secondarily by optimizing buffer sizes with safety mechanisms.

## 2. Overall Status
Active development. Performance analysis complete. Code review complete. Bottlenecks identified and verified against codebase. Implementation in progress.

### Current Architecture
```
Decode (HW) → Render/Composite (GPU/RGBA) → GPU Readback (RGBA) → Format Convert (CPU ❌) → Encode (HW)
```

### Target Architecture
```
Decode (HW) → Render/Composite (GPU/RGBA) → Format Convert (GPU ✅) → GPU Readback (NV12) → Encode (HW)
```

### Identified Bottlenecks (Verified)
1. **Primary**: CPU-based RGBA→NV12 conversion via FFmpeg software scaler (`h264.rs:241-289`)
2. **Secondary**: Small channel buffers (8 frames) cause renderer to block on encoder backpressure
3. **Bandwidth**: Reading RGBA (4 bytes/pixel) vs NV12 (1.5 bytes/pixel) from GPU wastes PCIe bandwidth

### Existing Infrastructure (Discovered in Review)
- `crates/gpu-converters/` - Existing compute shaders for format conversion (NV12→RGBA, YUYV→NV12, etc.)
- `crates/frame-converter/src/videotoolbox.rs` - Apple's native `VTPixelTransferSession` for hardware format conversion
- `H264EncoderBuilder::with_external_conversion()` - Flag to skip internal software scaler

## 3. Stories Breakdown

| Story ID | Story Name / Objective | Complexity | Est. Hours | Status | Link |
| :--- | :--- | :--- | :--- | :--- | :--- |
| S01 | Increase channel buffer sizes (with safety) | Low | ~2h | Done | [S01-buffer-sizes.md](./stories/S01-buffer-sizes.md) |
| S02 | Audit format conversion flow | Low | ~2h | Done | [S02-format-audit.md](./stories/S02-format-audit.md) |
| S03 | Implement RGBAToNV12 GPU converter | Medium | ~4-6h | Planned | Inline |
| S04 | Integrate GPU conversion into frame pipeline | Medium-High | ~6-8h | Planned | Inline |
| S05 | Benchmark and validate improvements | Low | ~2h | Planned | Inline |

## 4. Story Details

### S01 - Increase Channel Buffer Sizes (With Safety)
**Complexity: Low (~2h)**

**Rationale:** Current 8-frame buffer creates artificial stalls. However, simply increasing without safety mechanisms risks memory exhaustion on low-RAM Macs.

-   **Acceptance Criteria:**
    -   [ ] MP4 export channel increased from 8 to 32 frames (high RAM) or 16 frames (low RAM)
    -   [ ] GIF export channel increased from 4 to 16 frames
    -   [ ] Adaptive sizing based on available RAM
    -   [ ] Timeout mechanism on channel sends to prevent deadlocks
    -   [ ] Memory usage validated on 8GB Mac

-   **Tasks/Subtasks:**
    -   [ ] Modify `crates/export/src/mp4.rs:62-63` - add adaptive channel sizing
    -   [ ] Modify `crates/export/src/gif.rs:44` - increase to 16
    -   [ ] Add memory detection: if RAM > 16GB use 32, else use 16
    -   [ ] Add send timeout (e.g., 5 seconds) with graceful error handling
    -   [ ] Test on various RAM configurations

-   **Risk Mitigations:**
    - Adaptive sizing prevents OOM on 8GB Macs
    - Timeout prevents infinite stalls if encoder hangs
    - Can reduce buffer size if memory pressure detected

-   **Verified Code Locations:**
    - `crates/export/src/mp4.rs:62` - `let (tx_image_data, rx_image_data) = channel(8);`
    - `crates/export/src/mp4.rs:63` - `let (frame_tx, frame_rx) = channel(8);`
    - `crates/export/src/gif.rs:44` - `let (tx_image_data, rx) = channel(4);`

### S02 - Audit Format Conversion Flow
**Complexity: Low (~2h)**

**Rationale:** Need to fully understand current flow before optimizing. Previous analysis may have incomplete picture.

-   **Acceptance Criteria:**
    -   [ ] Document complete format flow from decoder to encoder
    -   [ ] Identify all conversion points
    -   [ ] Determine if any conversions are truly redundant
    -   [ ] Understand `with_external_conversion()` usage

-   **Tasks/Subtasks:**
    -   [ ] Add format logging at key points (temporary, for analysis)
    -   [ ] Trace: decoder output format → renderer input → renderer output → encoder input
    -   [ ] Check when `with_external_conversion()` is used and its effect
    -   [ ] Document findings in this task

-   **Key Questions to Answer:**
    1. What format does AVAssetReader output? (Likely NV12 from hardware decoder)
    2. Does renderer always need RGBA? (Yes, for compositing)
    3. Can we detect passthrough cases (no effects) and skip RGBA entirely?
    4. What format does h264_videotoolbox prefer? (NV12)

-   **Key Files:**
    - `crates/rendering/src/decoder/avassetreader.rs` - Decoder output
    - `crates/rendering/src/lib.rs` - Render pipeline
    - `crates/rendering/src/frame_pipeline.rs` - GPU readback
    - `crates/enc-ffmpeg/src/video/h264.rs:233-240` - `with_external_conversion` handling

### S03 - Implement RGBAToNV12 GPU Converter
**Complexity: Medium (~4-6h)**

**Rationale:** The `crates/gpu-converters/` already has infrastructure for GPU format conversion. We need to add RGBA→NV12 (the reverse of existing NV12→RGBA).

-   **Acceptance Criteria:**
    -   [ ] New `RGBAToNV12` converter following existing pattern
    -   [ ] Compute shader for RGBA→NV12 conversion (BT.709 color matrix)
    -   [ ] Handles even dimension requirement (NV12 constraint)
    -   [ ] Performance comparable to or better than CPU conversion
    -   [ ] Fallback to CPU if GPU unavailable

-   **Tasks/Subtasks:**
    -   [ ] Study existing `nv12_rgba/` converter as template
    -   [ ] Create `rgba_nv12/` module in `crates/gpu-converters/`
    -   [ ] Implement WGSL compute shader for RGBA→NV12
    -   [ ] Handle Y plane (full res) and UV plane (half res, interleaved)
    -   [ ] Add unit tests
    -   [ ] Benchmark GPU vs CPU conversion time

-   **Alternative Approach - VideoToolbox:**
    -   [ ] Evaluate `VTPixelTransferSession` in `crates/frame-converter/src/videotoolbox.rs`
    -   [ ] May be simpler and Apple-optimized
    -   [ ] Trade-off: Less control vs. maintained by Apple
    -   [ ] **Decision needed**: Custom shader vs VideoToolbox

-   **Technical Notes:**
    ```
    NV12 Format:
    - Y plane: width × height bytes (1 byte per pixel)
    - UV plane: width × (height/2) bytes (interleaved U,V at half resolution)
    - Total: 1.5 bytes per pixel (vs 4 for RGBA)

    Conversion (BT.709):
    Y  =  0.2126 R + 0.7152 G + 0.0722 B
    Cb = -0.1146 R - 0.3854 G + 0.5000 B + 128
    Cr =  0.5000 R - 0.4542 G - 0.0458 B + 128
    ```

-   **Existing Code to Reference:**
    - `crates/gpu-converters/src/nv12_rgba/mod.rs` - Reverse conversion (NV12→RGBA)
    - `crates/gpu-converters/src/nv12_rgba/shader.wgsl` - Shader example
    - `crates/gpu-converters/src/uyvy_nv12/` - Another NV12 output example

### S04 - Integrate GPU Conversion into Frame Pipeline
**Complexity: Medium-High (~6-8h)**

**Rationale:** The key optimization is converting BEFORE GPU→CPU readback. This reduces bandwidth by ~60% (NV12 is 1.5 bytes/pixel vs RGBA at 4 bytes/pixel).

-   **Acceptance Criteria:**
    -   [ ] `frame_pipeline.rs` supports NV12 output path
    -   [ ] Conversion happens on GPU before readback
    -   [ ] `PipelinedGpuReadback` handles NV12 buffers
    -   [ ] H264Encoder receives NV12 directly (skip software converter)
    -   [ ] Feature flag for easy rollback
    -   [ ] Bandwidth reduction measurable (~40% less GPU→CPU data)

-   **Tasks/Subtasks:**
    -   [ ] Add NV12 texture support to `frame_pipeline.rs`
    -   [ ] Integrate `RGBAToNV12` converter after render, before readback
    -   [ ] Update `PipelinedGpuReadback` buffer sizing for NV12 (smaller buffers)
    -   [ ] Use `H264EncoderBuilder::with_external_conversion(true)` to skip software path
    -   [ ] Add feature flag: `gpu-format-conversion` (default: enabled)
    -   [ ] Ensure fallback to CPU conversion if GPU path fails

-   **Architecture Change:**
    ```
    Current:
    GPU Render (RGBA) → Readback (RGBA, 4 bytes/px) → CPU Convert (RGBA→NV12) → Encode

    New:
    GPU Render (RGBA) → GPU Convert (RGBA→NV12) → Readback (NV12, 1.5 bytes/px) → Encode
    ```

-   **Key Files to Modify:**
    - `crates/rendering/src/frame_pipeline.rs` - Add NV12 path
    - `crates/export/src/mp4.rs` - Pass flag for external conversion
    - `crates/enc-ffmpeg/src/video/h264.rs` - Verify external conversion works

-   **Memory Impact:**
    ```
    Current (RGBA):  32 frames × 4K × 4 bytes = 32 × 33MB = ~1GB readback buffers
    New (NV12):      32 frames × 4K × 1.5 bytes = 32 × 12MB = ~400MB readback buffers
    Savings: ~60% less memory for readback buffers
    ```

### S05 - Benchmark and Validate Improvements
**Complexity: Low (~2h)**

-   **Acceptance Criteria:**
    -   [ ] Baseline measurements documented (before changes)
    -   [ ] Post-change measurements documented
    -   [ ] Various test cases covered
    -   [ ] Quality comparison completed
    -   [ ] Memory usage validated

-   **Test Matrix:**
    | Resolution | Duration | Source Format | Test |
    |------------|----------|---------------|------|
    | 1080p | 1 min | NV12 | Standard |
    | 4K | 1 min | NV12 | High res |
    | 1080p | 10 min | NV12 | Long duration |
    | 1080p | 1 min | Fragmented (HLS) | Edge case |

-   **Metrics to Capture:**
    - Export time (wall clock)
    - CPU usage (% during export)
    - GPU usage (% during export)
    - Memory usage (peak)
    - Output file size (quality proxy)
    - Visual quality (spot check)

-   **Tools:**
    - macOS: Instruments (Time Profiler, Metal System Trace)
    - Existing tests: `crates/export/tests/export_benchmark.rs`

## 5. Technical Considerations

### Hardware Acceleration Status

| Component | Current | After Optimization |
|-----------|---------|-------------------|
| Decoding | ✅ HW (VideoToolbox) | ✅ HW (no change) |
| Rendering | ✅ GPU (Metal/wgpu) | ✅ GPU (no change) |
| Format Convert | ❌ CPU (FFmpeg) | ✅ GPU (compute shader) |
| GPU Readback | RGBA (4 bytes/px) | NV12 (1.5 bytes/px) |
| Encoding | ✅ HW (VideoToolbox) | ✅ HW (no change) |

### Key Files

| File | Purpose | Changes |
|------|---------|---------|
| `crates/export/src/mp4.rs` | Export orchestration | Buffer sizes, external conversion flag |
| `crates/export/src/gif.rs` | GIF export | Buffer size |
| `crates/gpu-converters/src/` | GPU format converters | Add `rgba_nv12/` |
| `crates/rendering/src/frame_pipeline.rs` | GPU readback | NV12 output path |
| `crates/enc-ffmpeg/src/video/h264.rs` | H264 encoder | Verify external conversion |

### Memory Budget

| Scenario | Current | Proposed |
|----------|---------|----------|
| Frame buffers (32 frames, 4K, RGBA) | 32 × 33MB = 1GB | N/A |
| Frame buffers (32 frames, 4K, NV12) | N/A | 32 × 12MB = 400MB |
| GPU readback (3 buffers) | 3 × 33MB = 100MB | 3 × 12MB = 36MB |
| Total export pipeline | ~1.1GB | ~450MB |

### Feature Flag Strategy

```rust
// In export config or environment
pub struct ExportConfig {
    pub gpu_format_conversion: bool,  // default: true
    pub buffer_size: Option<usize>,   // default: adaptive
}

// Fallback behavior
if gpu_format_conversion && gpu_converter_available() {
    use_gpu_path()
} else {
    use_cpu_path()  // existing behavior
}
```

## 6. Risks and Mitigations

| Risk | Severity | Mitigation |
| :--- | :--- | :--- |
| Memory exhaustion on 8GB Macs | Medium-High | Adaptive buffer sizing, NV12 reduces memory |
| Deadlock if encoder stalls | Medium | Timeout on channel sends |
| GPU converter bugs | Medium | Feature flag, fallback to CPU, extensive testing |
| Quality regression | High | Visual comparison, file size validation |
| Odd dimension handling | Low | NV12 requires even dims, add validation |

## 7. Open Questions

| Question | Options | Recommendation |
|----------|---------|----------------|
| VideoToolbox vs custom shader? | VT (simpler) vs WGSL (control) | Start with WGSL to match existing infrastructure |
| Buffer size configurable? | Hardcoded vs user config | Adaptive default, advanced setting for override |
| Windows support? | Include vs separate task | Separate task (T003), focus on macOS first |

## 8. Success Metrics

| Metric | Baseline (Est.) | Target | Stretch |
|--------|-----------------|--------|---------|
| 1080p 1min export | ~15 seconds | ~10 seconds (33% faster) | ~8 seconds |
| 4K 1min export | ~60 seconds | ~35 seconds (40% faster) | ~30 seconds |
| Memory usage | ~1.1GB | <500MB | <400MB |
| GPU→CPU bandwidth | ~130 MB/s (4K60 RGBA) | ~50 MB/s (4K60 NV12) | - |

## 9. Review Findings (2026-01-14)

Code review identified several corrections to original plan:

1. **GPU converters already exist** - `crates/gpu-converters/` has infrastructure, just missing RGBA→NV12
2. **VideoToolbox option overlooked** - `VTPixelTransferSession` available but WGSL preferred for consistency
3. **Conversion location critical** - Must happen before GPU readback, not after
4. **Buffer increase alone insufficient** - Only 5-10% improvement without GPU conversion
5. **Memory savings with NV12** - Switching to NV12 readback saves ~60% memory

Original estimates revised upward: 35-55% improvement (from 20-40%) due to combined bandwidth + CPU savings.

## 10. Learnings (S01)

### API Verification Critical
- **Issue**: Plan assumed `tokio::sync::mpsc::Sender::send_timeout()` exists - it doesn't
- **Fix**: Must use `tokio::time::timeout()` wrapper pattern (already used elsewhere in codebase)
- **Impact**: Always verify API existence before planning implementation

### Error Type Precision
- **Issue**: Confused `RecvTimeoutError` vs `SendTimeoutError` for std sync channels
- **Fix**: For send operations, use `std::sync::mpsc::SendTimeoutError`
- **Impact**: Error types are operation-specific, not channel-specific

### Architecture Dictates Timeout Strategy
- **Issue**: MP4 and GIF exports have different architectures
  - MP4: renderer -> render_task -> encoder (sync channel in middle)
  - GIF: renderer -> encoder (no intermediate task)
- **Impact**: Timeout protection must match architecture. GIF timeout requires cap_rendering changes (deferred to S04)

### Dependency Version Drift
- **Issue**: Workspace defines `sysinfo = "0.32"` but recording crate uses `"0.35"`
- **Fix**: Use explicit version matching recording crate
- **Impact**: Check actual dependency versions, not just workspace definitions
