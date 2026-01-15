# Task: [T002] - Export Performance Optimization

## 0. Task Summary
-  **Task Name:** Export Performance Optimization
-  **Priority:** 3
-  **Number of Stories:** 7 (S01-S02 ✅, S03-S04 ⚠️ Superseded, S05-S07 Planned)
-  **Current Status:** ACTIVE - STRATEGIC PIVOT
-  **Platform:** macOS only (Windows feasible as future task)
-  **Dependencies:** `crates/export/`, `crates/rendering/`, `crates/enc-ffmpeg/`, `crates/frame-converter/`
-  **Rules Required:** CLAUDE.md (no comments, Rust clippy rules)
-  **Executor Ref:** See Stories S01-S07
-  **Acceptance Criteria:**
    - Measurable export speed improvement (target: 35-55% faster for 4K)
    - No regression in output quality
    - No increase in memory usage beyond acceptable bounds (~250MB)
    - Graceful handling of low-memory conditions
    - All existing export tests pass

## 1. Goal / Objective
Improve video export speed on macOS by addressing identified bottlenecks in the export pipeline. ~~Primary approach: moving RGBA→NV12 format conversion to GPU before readback.~~ **REVISED**: Use Apple-native APIs (VideoToolbox) for format conversion or let encoder handle BGRA directly.

## 2. Overall Status

**⚠️ STRATEGIC PIVOT (2026-01-15)**: Custom WGSL GPU shader approach (S03/S04) abandoned after research confirmed it is architecturally flawed. New direction uses Apple-native APIs.

### Current Architecture (Baseline: 43 fps)
```
Decode (HW) → Render/Composite (GPU/RGBA) → GPU Readback (RGBA) → Format Convert (CPU sws_scale ❌) → Encode (HW)
```

### ~~Original Target Architecture~~ (ABANDONED)
```
Decode (HW) → Render/Composite (GPU/RGBA) → Format Convert (GPU WGSL ❌) → GPU Readback (NV12) → Encode (HW)
```
**Why abandoned**: Custom GPU converter created 13x performance regression due to separate device/queue, blocking operations, and double readback. See Section 14 for details.

### NEW Target Architecture (Apple-Native)
```
Option A: Decode (HW) → Render/Composite (GPU/RGBA) → GPU Readback (BGRA) → Encode (HW, accepts BGRA directly ✅)
Option B: Decode (HW) → Render/Composite (GPU/RGBA) → GPU Readback (RGBA) → VTPixelTransfer (HW ✅) → Encode (HW)
```

### Performance Targets
| Metric | Baseline | Target | Hardware Max |
|--------|----------|--------|--------------|
| Export FPS | 43 fps | 50-55 fps | ~60 fps |
| Improvement | - | +16-28% | +40% |

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
| S01 | Increase channel buffer sizes (with safety) | Low | ~2h | ✅ Done | [S01-buffer-sizes.md](./stories/S01-buffer-sizes.md) |
| S02 | Audit format conversion flow | Low | ~2h | ✅ Done | [S02-format-audit.md](./stories/S02-format-audit.md) |
| S03 | ~~Implement RGBAToNV12 GPU converter~~ | Medium | ~4-6h | ⚠️ Superseded | [S03-rgba-nv12-converter.md](./stories/S03-rgba-nv12-converter.md) |
| S04 | ~~Integrate GPU conversion into frame pipeline~~ | Medium-High | ~6-8h | ⚠️ Superseded | [S04-pipeline-integration.md](./stories/S04-pipeline-integration.md) |
| S05 | Establish baseline & verify current bottleneck | Low | ~1-2h | 🔄 Revised | Inline |
| S06 | Implement VTPixelTransferSession for RGBA→NV12 | Medium | ~2-3h | 🆕 **NEXT** | Inline |
| S07 | Benchmark and validate improvements | Low | ~1h | Planned | Inline |

### Story Status Legend
- ✅ Done - Completed and working
- ⚠️ Superseded - Completed but approach abandoned (code disabled, kept for reference)
- 🔄 Revised - Story redefined based on implementation review
- 🆕 **NEXT** - Priority story for implementation

### Execution Order (Revised per Implementation Review)

```
S05 (Baseline) → S06 (VTPixelTransfer) → S07 (Validate)
```

**Rationale:** Implementation review found that S05's original premise ("BGRA direct input skips conversion") is incorrect - the encoder already handles BGRA but converts internally. S06 (VTPixelTransferSession) is the actual optimization. See Section 15 for full analysis.

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

### S03 - ~~Implement RGBAToNV12 GPU Converter~~ ⚠️ SUPERSEDED
**Status: SUPERSEDED** - Code complete but approach abandoned. See Section 14.

**Original Rationale:** The `crates/gpu-converters/` already has infrastructure for GPU format conversion. We need to add RGBA→NV12 (the reverse of existing NV12→RGBA).

**What was implemented:**
- ✅ WGSL compute shader for RGBA→NV12 conversion (BT.709 color matrix)
- ✅ Byte packing fix for correct NV12 output
- ✅ Color output verified correct (no green tint after fix)

**Why superseded:**
- The shader itself works correctly
- But integration architecture is fundamentally flawed (see S04)
- Research confirmed Apple-native APIs are superior for macOS
- Code kept in `crates/gpu-converters/src/rgba_nv12/` for reference (disabled by default)

### S04 - ~~Integrate GPU Conversion into Frame Pipeline~~ ⚠️ SUPERSEDED
**Status: SUPERSEDED** - Implementation caused 13x performance regression. See Section 14.

**Original Rationale:** The key optimization is converting BEFORE GPU→CPU readback. This reduces bandwidth by ~60%.

**What was implemented:**
- ✅ Integration into `mp4.rs` render_task
- ✅ Feature flag `CAP_GPU_FORMAT_CONVERSION` (default: disabled)
- ✅ Color output verified correct after shader fix

**Why superseded - CRITICAL ARCHITECTURAL FLAWS:**

1. **Separate GPU Context**: Created new `wgpu::Device`/`Queue` instead of sharing with renderer
2. **Blocking Operations**: `device.poll(Wait)` inside async task serialized ALL frame processing
3. **Double Readback**: Read RGBA to CPU → Convert on GPU → Read NV12 to CPU (2x transfers!)
4. **Result**: 13x performance regression (39s → 529s)

**Research Conclusion:** The approach requires fundamental redesign that isn't worth the effort when Apple provides superior native APIs. See `research-questions-export-optimization.md` for full analysis.

**Code Status:** Disabled by default (`CAP_GPU_FORMAT_CONVERSION=false`). Code preserved in:
- `crates/gpu-converters/src/rgba_nv12/` - Shader and Rust wrapper
- `crates/export/src/mp4.rs` - Integration (gated by env var)

### S05 - Establish Baseline & Verify Current Bottleneck 🔄 Revised
**Complexity: Low (~1-2h)** | **Status: Revised** | **Priority: HIGH**

**Background:** Implementation review found that the original S05 premise ("BGRA direct input skips conversion") is incorrect. The encoder already accepts BGRA but converts internally via FFmpeg's `sws_scale`. This story is revised to establish a proper baseline before implementing VTPixelTransferSession.

**Rationale:** We need documented baseline measurements and verification that `sws_scale` is indeed the bottleneck before implementing S06. This ensures we're measuring the right thing.

-   **Acceptance Criteria:**
    -   [ ] Document baseline export performance (FPS, time, CPU/GPU usage)
    -   [ ] Verify `sws_scale` is the bottleneck via profiling/logging
    -   [ ] Confirm current encoder path (BGRA → sws_scale → NV12 → VideoToolbox)
    -   [ ] Establish test recordings for consistent benchmarking

-   **Tasks/Subtasks:**
    -   [ ] Run export on standardized test recording (1080p, 1min)
    -   [ ] Log/profile to confirm `sws_scale` usage (`h264.rs:241-289`)
    -   [ ] Record baseline metrics: export time, FPS, CPU%, memory peak
    -   [ ] Document test recording specs for S07 comparison
    -   [ ] Verify `with_external_conversion(true)` would skip internal conversion

-   **Key Files:**
    - `crates/enc-ffmpeg/src/video/h264.rs:241-289` - sws_scale conversion point
    - `crates/export/src/mp4.rs` - Export orchestration

-   **Expected Outcome:**
    - Documented baseline: ~43 fps export performance
    - Confirmed bottleneck: CPU-based sws_scale
    - Ready for S06 implementation with clear before/after comparison

---

### S06 - Implement VTPixelTransferSession for RGBA→NV12 🆕 **PRIMARY**
**Complexity: Medium (~2-3h)** | **Status: NEXT** | **Priority: HIGHEST**

**Background:** Implementation review confirmed this is the actual optimization. The encoder already accepts BGRA but uses CPU-based `sws_scale` internally. VTPixelTransferSession replaces `sws_scale` with Apple's hardware-accelerated converter.

**Rationale:** Apple's `VTPixelTransferSession` is a hardware-accelerated format converter that:
- Runs on GPU/Apple Neural Engine
- Supports zero-copy with proper CVPixelBuffer management
- Already has partial infrastructure in `crates/frame-converter/src/videotoolbox.rs`
- Is the proven, Apple-recommended approach

-   **Acceptance Criteria:**
    -   [ ] Add RGBA→NV12 support to existing `VTPixelTransferSession` wrapper
    -   [ ] Replace `sws_scale` path with `VTPixelTransferSessionTransferImage`
    -   [ ] Enable via `with_external_conversion(true)` flag
    -   [ ] Verify output quality matches baseline
    -   [ ] Achieve target: 50+ fps (up from 43 fps baseline)

-   **Tasks/Subtasks:**
    -   [ ] Review existing `crates/frame-converter/src/videotoolbox.rs` (already supports YUYV→NV12, UYVY→NV12)
    -   [ ] Add `rgba_to_nv12()` method using existing patterns
    -   [ ] Wrap RGBA frame data in `CVPixelBuffer` (source)
    -   [ ] Create NV12 `CVPixelBuffer` for destination
    -   [ ] Call `VTPixelTransferSessionTransferImage` for conversion
    -   [ ] Integrate into `h264.rs` encoder setup (alternative to sws_scale path)
    -   [ ] Add feature flag for easy A/B testing

-   **API Usage (from research):**
    ```c
    VTPixelTransferSessionRef session;
    VTPixelTransferSessionCreate(kCFAllocatorDefault, &session);

    // Per frame:
    CVPixelBufferRef destNV12;
    CVPixelBufferCreate(..., kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange, &destNV12);
    VTPixelTransferSessionTransferImage(session, sourceRGBA, destNV12, NULL);
    // destNV12 now contains converted frame - send to encoder

    // Cleanup:
    VTPixelTransferSessionInvalidate(session);
    CFRelease(session);
    ```

-   **Key Files:**
    - `crates/frame-converter/src/videotoolbox.rs` - Existing VT wrapper (extend)
    - `crates/enc-ffmpeg/src/video/h264.rs:241-289` - Replace sws_scale call
    - `crates/export/src/mp4.rs` - Integration point

-   **Why This Works (vs Custom WGSL):**
    - Reuses existing VideoToolbox session (no new GPU context)
    - No blocking operations needed
    - No double readback - conversion happens in encoder path
    - Apple-optimized for their hardware

---

### S07 - Benchmark and Validate Improvements
**Complexity: Low (~2h)** | **Status: Planned**

-   **Acceptance Criteria:**
    -   [ ] Baseline measurements documented (43 fps / 39s for 1667 frames)
    -   [ ] Post-change measurements documented
    -   [ ] Target: 50-55 fps achieved
    -   [ ] Quality comparison completed (no regression)
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
    - Export FPS (frames / time)
    - CPU usage (% during export)
    - GPU usage (% during export)
    - Memory usage (peak)
    - Output file size (quality proxy)
    - Visual quality (spot check)

-   **Success Criteria:**
    | Metric | Baseline | Target | Result |
    |--------|----------|--------|--------|
    | Export FPS | 43 fps | 50-55 fps | TBD |
    | Improvement | - | +16-28% | TBD |

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

### std::sync::mpsc::send_timeout() Does NOT Exist (CRITICAL)
- **Issue**: Plan assumed `std::sync::mpsc::SyncSender::send_timeout()` exists - it doesn't in stable Rust
- **Error**: `error[E0658]: use of unstable library feature 'std_internals'`
- **Fix**: Use blocking `send()` instead - the receive-side timeout already provides stall protection
- **Impact**: Always verify std API existence in stable Rust. Alternative crates like `crossbeam-channel` have `send_timeout()`, but std doesn't
- **Lesson**: The increased buffer sizes (S01's main value) absorb temporary encoder delays; receive-side timeout catches true stalls

## 11. S01-S03 Testing Results (2026-01-15)

Manual testing on 16GB macOS system confirmed:
- Export completes successfully (MP4)
- Buffer config correctly detected 16GB RAM → 32/16 frame buffers
- Log output: `total_ram_gb=16.0 rendered_buffer=32 encoder_buffer=16`
- No crashes or errors related to S01-S03 changes
- Existing decoder warnings (BufferAsyncError) are pre-existing issues unrelated to this work

## 12. S04 Implementation Notes (2026-01-15)

### Integration Approach

Implemented "simple" integration path that provides the main benefit (eliminating CPU sws_scale):

```
Current Flow (with S04):
GPU Render (RGBA) → GPU Readback (RGBA) → GPU Convert (RGBA→NV12) → Encode (NV12)

Full Integration (future optimization):
GPU Render (RGBA) → GPU Convert (RGBA→NV12) → GPU Readback (NV12) → Encode (NV12)
```

### Files Modified

1. **crates/rendering/src/frame_pipeline.rs**
   - Extended `RenderedFrame` with `pixel_format` and `y_plane_size` fields
   - Added `y_plane()` and `uv_plane()` helper methods
   - Reused existing `PixelFormat` enum from decoder module

2. **crates/media-info/src/lib.rs**
   - Added `wrap_nv12_frame()` method for multi-plane NV12 format

3. **crates/export/Cargo.toml**
   - Added `cap-gpu-converters` dependency

4. **crates/export/src/mp4.rs**
   - Initialize `RGBAToNV12` converter at export start
   - Conditional NV12 format based on converter availability
   - Enable `with_external_conversion()` when using NV12
   - Convert RGBA→NV12 in render_task
   - Environment variable `CAP_GPU_FORMAT_CONVERSION` for disable

### Benefits of Current Approach

- Eliminates CPU sws_scale (main bottleneck)
- Simple to implement and test
- Graceful fallback to RGBA path if GPU fails
- No changes to core rendering loop
- Feature flag for easy rollback

### Future Optimization (Not Implemented)

Full integration would move GPU conversion BEFORE readback:
- Requires refactoring `RGBAToNV12` to use shared device/queue
- Would reduce PCIe bandwidth by ~62.5%
- More complex, deferred to follow-up task

### Testing Required

- Build on macOS and verify no compilation errors
- Export video and check for NV12 conversion log messages
- Verify output quality matches pre-S04 output
- Measure performance improvement in S05

## 13. S04 Issues and Fixes (2026-01-15)

### Critical Issues Discovered

Testing revealed two critical issues with S04:

1. **13x Performance Regression** (39s → 529s for same export)
   - Root cause: Blocking GPU operations (`device.poll(Wait)`) inside async task
   - Creates separate GPU context instead of sharing with renderer
   - Serializes all frame processing, defeating pipelining

2. **Video Corruption** (Green color filter, artifacts)
   - Root cause: WGSL shader using `array<u32>` but treating indices as byte offsets
   - Each `y_plane[idx] = value` wrote 4 bytes instead of 1 byte
   - Memory layout completely wrong, causing color corruption

### Immediate Fix Applied

Disabled GPU conversion by default until fixes are verified:
```rust
fn gpu_conversion_enabled() -> bool {
    std::env::var("CAP_GPU_FORMAT_CONVERSION")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)  // Changed from true to false
}
```

### Shader Fix (2026-01-15)

Fixed byte packing in `shader.wgsl`:

**Before (BROKEN)**:
```wgsl
@group(0) @binding(1) var<storage, read_write> y_plane: array<u32>;
let y_idx = pos.y * dims.x + pos.x;
y_plane[y_idx] = y_value;  // Writes 4 bytes per pixel!
```

**After (FIXED)**:
```wgsl
@group(0) @binding(1) var<storage, read_write> y_plane: array<atomic<u32>>;
let y_linear = pos.y * dims.x + pos.x;
let y_word_idx = y_linear / 4u;
let y_byte_pos = y_linear % 4u;
let y_shifted = y_value << (y_byte_pos * 8u);
atomicOr(&y_plane[y_word_idx], y_shifted);  // Packs 4 bytes into each u32
```

Also updated Rust code to:
- Zero-initialize buffers (required for atomicOr)
- Pad buffer sizes to u32 boundaries
- Truncate output to exact NV12 sizes

### Remaining Performance Issue

The blocking architecture issue remains unsolved. Options:

1. **Short-term**: Use `spawn_blocking` wrapper (simple, may not fully fix)
2. **Medium-term**: Use VideoToolbox `VTPixelTransferSession` (macOS native, Apple-optimized)
3. **Long-term**: Integrate conversion into rendering pipeline BEFORE readback (ideal architecture)

### Testing After Fix - VERIFIED (2026-01-15)

Color fix confirmed working on macOS:
- ✅ Video output is correct (no green tint or artifacts)
- ⚠️ Performance still degraded: 527.9s for 1667 frames (vs 39s without GPU conversion)
- ~~**Next step**: Fix blocking architecture to restore performance~~
- **UPDATED**: Approach abandoned - see Section 14

## 14. Strategic Pivot: Apple-Native APIs (2026-01-15)

### Decision Summary

**The custom WGSL GPU shader approach (S03/S04) is ABANDONED.**

After comprehensive research (see `research-questions-export-optimization.md`), we determined:

1. The custom GPU converter is **architecturally flawed** - not fixable with minor changes
2. Apple provides **superior native alternatives** we weren't using
3. The VideoToolbox encoder can **accept BGRA directly** - may not need conversion at all

### Research Findings

| Question | Answer |
|----------|--------|
| Is custom WGSL approach viable? | **NO** - requires fundamental redesign not worth the effort |
| Can encoder accept BGRA directly? | **YES** - VideoToolbox H.264 accepts BGRA/RGBA input |
| Is VTPixelTransferSession available? | **YES** - hardware-accelerated, already in codebase |
| What's the theoretical max performance? | ~60 fps (currently 43 fps, ~40% headroom) |

### Why Custom GPU Approach Failed

1. **Separate GPU Context**: Created new `wgpu::Device`/`Queue` instead of sharing with renderer
2. **Blocking Operations**: `device.poll(Wait)` inside async task serialized ALL frame processing
3. **Double Readback**: RGBA GPU→CPU then NV12 GPU→CPU (defeated entire purpose)
4. **Result**: 13x worse performance (39s → 529s)

To fix this properly would require:
- Merging GPU contexts (share device/queue with renderer)
- Converting BEFORE readback (not after)
- Async buffer mapping (no blocking)
- This is essentially a complete rewrite with high risk

### New Approach: Apple-Native APIs

**Option 1 (HIGHEST PRIORITY): BGRA Direct Input**
```
GPU Render (RGBA) → GPU Readback (BGRA) → Encoder (accepts BGRA directly)
```
- If VideoToolbox accepts BGRA, we skip conversion entirely
- Simplest possible solution
- Test in S05

**Option 2 (FALLBACK): VTPixelTransferSession**
```
GPU Render (RGBA) → GPU Readback (RGBA) → VTPixelTransfer (HW) → Encoder (NV12)
```
- Apple's hardware-accelerated format converter
- Already exists in `crates/frame-converter/src/videotoolbox.rs`
- Fast, zero-copy capable with proper CVPixelBuffer management
- Implement in S06 if S05 insufficient

### Code Disposition

| Component | Location | Status |
|-----------|----------|--------|
| WGSL Shader | `crates/gpu-converters/src/rgba_nv12/shader.wgsl` | Preserved (disabled) |
| Rust Wrapper | `crates/gpu-converters/src/rgba_nv12/mod.rs` | Preserved (disabled) |
| Integration | `crates/export/src/mp4.rs` | Gated by `CAP_GPU_FORMAT_CONVERSION=false` |

The code is kept for:
- Reference for future Windows implementation (where VTPixelTransfer doesn't exist)
- Educational value (working BT.709 RGBA→NV12 shader)
- Potential future use if wgpu gains better CVPixelBuffer/IOSurface support

### Updated Task Plan (Revised 2026-01-15)

**Execution Order:** S05 → S06 → S07

1. **S05** (Baseline): Establish baseline metrics & verify sws_scale is the bottleneck
2. **S06** (PRIMARY): Implement VTPixelTransferSession to replace sws_scale
3. **S07** (Validate): Benchmark and validate 50-55 fps target achieved

See Section 15 for implementation review that led to this revised order.

### Key Learnings

1. **Platform-native APIs first**: Always evaluate OS-provided solutions before custom implementations
2. **Architecture before optimization**: A fast algorithm with bad integration is slower than a slow algorithm with good integration
3. **Research before coding**: The research document revealed answers in hours that would have taken days to discover through trial and error
4. **Fail fast, pivot decisively**: Recognizing the 13x regression and pivoting saved significant wasted effort

### References

- `research-questions-export-optimization.md` - Comprehensive research with citations
- `s04-investigation-report.md` - Technical investigation of failures
- Apple WWDC: VideoToolbox and VTPixelTransferSession documentation
- FFmpeg VideoToolbox overlay filter examples

## 15. Implementation Review Findings (2026-01-15)

Prior to implementation, a subagent reviewed the S05-S07 plan for gaps, inconsistencies, and incorrect assumptions. The full review is in `stories/S05-S07-implementation-review.md`.

### Critical Finding: S05 Premise Was Incorrect

**Original S05 Assumption:** "VideoToolbox encoder can accept BGRA directly, skipping conversion entirely"

**Actual Behavior:** The encoder already accepts BGRA, but FFmpeg's `h264_videotoolbox` performs internal conversion via `sws_scale` before passing to VideoToolbox. The "BGRA direct input" optimization was already happening - just inefficiently.

```
What we thought:      BGRA → [skip conversion] → Encoder
What actually happens: BGRA → sws_scale (CPU) → NV12 → Encoder
```

### Revised Understanding

| Component | Original Understanding | Corrected Understanding |
|-----------|----------------------|-------------------------|
| S05 (BGRA Direct) | "Skip conversion entirely" | Conversion already happens via sws_scale internally |
| S06 (VTPixelTransfer) | "Fallback if S05 fails" | **The actual optimization** - replaces sws_scale |
| Bottleneck | "Format conversion" | Specifically `sws_scale` CPU conversion in encoder path |

### Why This Matters

1. **S05 was solving an already-solved problem** - encoder already handles BGRA
2. **S06 is the real optimization** - replaces CPU sws_scale with HW VTPixelTransfer
3. **No code changes needed for "BGRA direct"** - it's already the default

### Revised Execution Order

```
Previous:  S05 (test BGRA) → S06 (fallback) → S07 (benchmark)
Revised:   S05 (baseline)  → S06 (primary)  → S07 (validate)
```

**Rationale:**
- S05 now establishes baseline and confirms sws_scale is the bottleneck
- S06 is the primary implementation story (not a fallback)
- S07 validates the improvement against documented baseline

### Other Review Findings

1. **Baseline poorly documented**: The "43 fps" figure was referenced but test conditions weren't specified
2. **Success criteria needs verification**: Need to confirm hardware specs, test recording, measurement methodology
3. **VTPixelTransferSession exists**: `crates/frame-converter/src/videotoolbox.rs` already has YUYV→NV12 support; adding RGBA→NV12 follows same pattern

### Action Taken

- S05 redefined as baseline establishment story
- S06 promoted from "fallback" to "primary optimization"
- Story details updated with corrected technical understanding
- Full review document preserved for reference
