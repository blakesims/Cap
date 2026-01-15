# S04 Investigation Report: GPU RGBA→NV12 Conversion Issues

## Status: ⚠️ INVESTIGATION COMPLETE - APPROACH ABANDONED

**Date**: 2026-01-15
**Conclusion**: Custom WGSL GPU shader approach is architecturally flawed. Pivot to Apple-native APIs.

---

## 1. Executive Summary

S04 implementation of GPU-based RGBA→NV12 conversion resulted in:
1. **13x performance regression** (39s → 529s for same export)
2. **Video corruption** (green color filter, artifacts)

**FINAL STATUS (2026-01-15)**:
- ✅ Color corruption fixed (shader byte packing corrected)
- ❌ Performance regression NOT fixable without fundamental redesign
- 🔄 **DECISION**: Abandon custom GPU approach, use Apple-native APIs instead

See `research-questions-export-optimization.md` for comprehensive research findings that informed this decision.

---

## 2. Current Implementation Overview

### 2.1 Files Involved

| File | Purpose |
|------|---------|
| `crates/gpu-converters/src/rgba_nv12/mod.rs` | GPU converter implementation |
| `crates/gpu-converters/src/rgba_nv12/shader.wgsl` | WGSL compute shader |
| `crates/export/src/mp4.rs` | Integration point (render_task) |
| `crates/media-info/src/lib.rs` | `wrap_nv12_frame()` method |
| `crates/rendering/src/frame_pipeline.rs` | RenderedFrame struct |

### 2.2 Current Data Flow (S04)

```
GPU Render (RGBA texture)
    ↓
GPU Readback (RGBA buffer) - Pipelined with 3 buffers
    ↓
render_task receives RenderedFrame (RGBA)
    ↓
[BLOCKING] RGBAToNV12::convert() - Creates own GPU context, runs compute, waits
    ↓
wrap_nv12_frame() - Copies Y+UV planes to FFmpeg frame
    ↓
H264 Encoder (with_external_conversion=true, expects NV12)
```

### 2.3 Original Data Flow (Working)

```
GPU Render (RGBA texture)
    ↓
GPU Readback (RGBA buffer) - Pipelined with 3 buffers
    ↓
render_task receives RenderedFrame (RGBA)
    ↓
wrap_frame() - Copies RGBA to FFmpeg frame
    ↓
H264 Encoder (internal sws_scale converts RGBA→NV12)
```

---

## 3. Issue 1: Performance Regression

### 3.1 Observed Behavior

| Metric | Without S04 | With S04 | Regression |
|--------|-------------|----------|------------|
| Export time (1667 frames) | 39.7s | 529.7s | 13.3x slower |
| Frame processing | Parallel (jumping frame numbers) | Sequential | Lost parallelism |

### 3.2 Suspected Root Cause

The `RGBAToNV12::convert()` method is **synchronous and blocking**:

```rust
// In crates/gpu-converters/src/rgba_nv12/mod.rs
pub fn convert(&self, rgba_data: &[u8], width: u32, height: u32) -> Result<...> {
    // ... setup GPU resources ...

    // BLOCKING: Waits for GPU to complete
    let _submission = self.queue.submit(std::iter::once(encoder.finish()));

    // BLOCKING: Waits for buffer mapping
    read_buffer_to_vec(&y_read_buffer, &self.device)?  // calls device.poll(Wait)
    read_buffer_to_vec(&uv_read_buffer, &self.device)? // calls device.poll(Wait)
}
```

This is called inside an **async task** (`render_task` in mp4.rs), which:
1. Blocks the tokio runtime thread
2. Serializes all frame processing
3. Defeats the pipelined GPU readback design

### 3.3 Questions to Investigate

- [ ] Q1.1: How does the original pipeline achieve parallelism? Study `frame_pipeline.rs` pipelining.
- [ ] Q1.2: Can `RGBAToNV12::convert()` be made async/non-blocking?
- [ ] Q1.3: What is the overhead of creating GPU resources per-frame vs reusing?
- [ ] Q1.4: Would `spawn_blocking` wrapper help, or just move the bottleneck?
- [ ] Q1.5: Is there GPU resource contention between rendering and conversion contexts?

### 3.4 Potential Solutions

**Solution A: spawn_blocking wrapper**
```rust
let (y, uv) = tokio::task::spawn_blocking(move || {
    conv.convert(&rgba_data, width, height)
}).await??;
```
- Pros: Simple change
- Cons: Still blocks a thread, may not fix parallelism

**Solution B: Async GPU conversion**
- Refactor `RGBAToNV12` to use async buffer mapping
- Use channels to pipeline conversion
- Pros: Proper async integration
- Cons: Significant refactor

**Solution C: Integrate into rendering pipeline (before readback)**
```
GPU Render (RGBA) → GPU Convert (RGBA→NV12) → GPU Readback (NV12)
```
- Pros: Maximum efficiency, shares GPU context
- Cons: Requires changes to frame_pipeline.rs, more complex

**Solution D: Use existing infrastructure**
- Study `crates/frame-converter/src/videotoolbox.rs` - Apple's VTPixelTransferSession
- May already have optimized RGBA→NV12 path
- Pros: Apple-optimized, maintained
- Cons: macOS only

### 3.5 Investigation Tasks

- [ ] T1.1: Read and document how `PipelinedGpuReadback` achieves parallelism
- [ ] T1.2: Profile where time is spent in `RGBAToNV12::convert()`
- [ ] T1.3: Check if `util::read_buffer_to_vec()` can be made async
- [ ] T1.4: Study VideoToolbox VTPixelTransferSession as alternative
- [ ] T1.5: Determine if shared GPU context is feasible

---

## 4. Issue 2: Video Corruption (Green Tint)

### 4.1 Observed Behavior

- Video has green color filter/tint
- Artifacts visible throughout
- Corruption consistent across all frames

### 4.2 Suspected Root Causes

#### Hypothesis A: Wrong Input Format (RGBA vs BGRA)

The shader assumes RGBA input:
```wgsl
let rgba = textureLoad(rgba_input, pos, 0);
// rgba.r, rgba.g, rgba.b used directly
```

But if renderer outputs BGRA, the channels are swapped.

**Investigation**: Check `wgpu::TextureFormat` used in rendering.

#### Hypothesis B: Stride/Padding Mismatch

The shader outputs tightly packed NV12:
- Y plane: width × height bytes (no padding)
- UV plane: width × (height/2) bytes (no padding)

But FFmpeg may expect aligned/padded rows.

**Investigation**: Check `frame.stride(0)` and `frame.stride(1)` in `wrap_nv12_frame()`.

#### Hypothesis C: Color Range Mismatch

The shader uses BT.709 limited range coefficients:
```wgsl
Y  = 16 + 65.481×R + 128.553×G + 24.966×B   // Range: 16-235
Cb = 128 - 37.797×R - 74.203×G + 112.0×B    // Range: 16-240
Cr = 128 + 112.0×R - 93.786×G - 18.214×B    // Range: 16-240
```

But h264_videotoolbox may expect full range (0-255) or different matrix.

**Investigation**: Check what color range the H264 encoder expects.

#### Hypothesis D: UV Plane Interleaving Bug

NV12 UV plane format: `U0 V0 U1 V1 U2 V2 ...`

Shader code:
```wgsl
let uv_base = uv_row * dims.x + pos.x;
uv_plane[uv_base] = u_value;
uv_plane[uv_base + 1u] = v_value;
```

Potential issues:
- Only processes even (x,y) positions - are all covered?
- Is `dims.x` correct for UV plane indexing?

#### Hypothesis E: wrap_nv12_frame() Implementation Bug

```rust
pub fn wrap_nv12_frame(&self, data: &[u8], y_plane_size: usize, timestamp: i64) -> frame::Video {
    // Y plane copy
    for row in 0..height {
        y_dst[dst_start..].copy_from_slice(&y_data[src_start..src_end]);
    }
    // UV plane copy
    for row in 0..uv_height {
        uv_dst[dst_start..].copy_from_slice(&uv_data[src_start..src_end]);
    }
}
```

Potential issues:
- Stride calculation may be wrong
- FFmpeg frame may have different layout expectations

### 4.3 Questions to Investigate

- [ ] Q2.1: What texture format does the renderer output? (RGBA8Unorm, BGRA8Unorm?)
- [ ] Q2.2: What stride does FFmpeg expect for NV12 frames?
- [ ] Q2.3: What color range does h264_videotoolbox expect?
- [ ] Q2.4: Is the UV interleaving correct for all pixel positions?
- [ ] Q2.5: How does the working CPU path (sws_scale) configure NV12?

### 4.4 Investigation Tasks

- [ ] T2.1: Find renderer texture format in frame_pipeline.rs
- [ ] T2.2: Check FFmpeg NV12 frame stride requirements
- [ ] T2.3: Compare shader coefficients with FFmpeg sws_scale BT.709
- [ ] T2.4: Verify UV plane indexing covers all 2x2 blocks correctly
- [ ] T2.5: Add debug logging to compare input/output sizes
- [ ] T2.6: Study how `h264.rs` configures the encoder for NV12 input

---

## 5. Architectural Analysis

### 5.1 Current Architecture Problems

1. **Separate GPU contexts**: Converter creates own device/queue, separate from renderer
2. **Blocking in async**: Synchronous GPU wait inside async task
3. **Per-frame resource creation**: Creates textures/buffers every frame
4. **Double data movement**: RGBA read from GPU → Convert on GPU → Read NV12 from GPU

### 5.2 Ideal Architecture

```
Renderer GPU Context:
  Render Pass → RGBA Texture
       ↓
  Compute Pass → NV12 Buffers (Y + UV)
       ↓
  Pipelined Readback → NV12 data to CPU
       ↓
  Encoder receives NV12 directly
```

Benefits:
- Single GPU context (no resource duplication)
- GPU-to-GPU conversion (no intermediate CPU copy)
- Maintains pipelining
- Reduces PCIe bandwidth (NV12 is 1.5 bytes/pixel vs RGBA 4 bytes/pixel)

### 5.3 Questions to Investigate

- [ ] Q3.1: Can `RGBAToNV12` be modified to accept external device/queue?
- [ ] Q3.2: Can it work with wgpu::Texture input instead of raw bytes?
- [ ] Q3.3: How would this integrate with `PipelinedGpuReadback`?
- [ ] Q3.4: What changes are needed to `RenderSession`?

---

## 6. Reference Code Locations

### 6.1 Rendering Pipeline

| File | Lines | Purpose |
|------|-------|---------|
| `crates/rendering/src/frame_pipeline.rs` | 86-204 | PipelinedGpuReadback |
| `crates/rendering/src/frame_pipeline.rs` | 206-303 | RenderSession |
| `crates/rendering/src/frame_pipeline.rs` | 366-409 | finish_encoder() |
| `crates/rendering/src/lib.rs` | 502-512 | RenderVideoConstants (device, queue) |

### 6.2 GPU Converter

| File | Lines | Purpose |
|------|-------|---------|
| `crates/gpu-converters/src/rgba_nv12/mod.rs` | 12-102 | RGBAToNV12::new() |
| `crates/gpu-converters/src/rgba_nv12/mod.rs` | 104-249 | RGBAToNV12::convert() |
| `crates/gpu-converters/src/rgba_nv12/shader.wgsl` | all | Compute shader |
| `crates/gpu-converters/src/util.rs` | all | read_buffer_to_vec() |

### 6.3 Export Integration

| File | Lines | Purpose |
|------|-------|---------|
| `crates/export/src/mp4.rs` | 51-55 | gpu_conversion_enabled() |
| `crates/export/src/mp4.rs` | 71-85 | Converter initialization |
| `crates/export/src/mp4.rs` | 252-292 | Frame conversion in render_task |
| `crates/media-info/src/lib.rs` | 325-373 | wrap_nv12_frame() |

### 6.4 H264 Encoder

| File | Lines | Purpose |
|------|-------|---------|
| `crates/enc-ffmpeg/src/video/h264.rs` | 83-86 | with_external_conversion() |
| `crates/enc-ffmpeg/src/video/h264.rs` | 233-240 | External conversion handling |

### 6.5 Alternative Approaches

| File | Purpose |
|------|---------|
| `crates/frame-converter/src/videotoolbox.rs` | Apple VTPixelTransferSession |
| `crates/gpu-converters/src/nv12_rgba/` | Reverse conversion (for reference) |

---

## 7. Investigation Findings

### 7.1 Performance Findings

**Root Cause**: Blocking GPU operations in async context

The `RGBAToNV12::convert()` method uses synchronous blocking:
- `device.poll(wgpu::PollType::Wait)` blocks until ALL GPU work completes
- Called inside async `render_task`, which serializes all frame processing
- Creates separate GPU context instead of sharing with renderer
- Defeats the pipelined readback design (3-buffer rotation)

**Evidence**: Log shows sequential frame numbers (1, 2, 3...) instead of jumping (indicating parallelism lost)

### 7.2 Color Corruption Findings

**Root Cause**: WGSL shader byte packing bug

The shader declared `array<u32>` but treated indices as byte offsets:

```wgsl
// BEFORE (BROKEN):
@group(0) @binding(1) var<storage, read_write> y_plane: array<u32>;
let y_idx = pos.y * dims.x + pos.x;
y_plane[y_idx] = y_value;  // Writes 4 bytes where 1 byte needed!
```

This caused:
- Each Y pixel wrote 32 bits instead of 8 bits
- Memory layout completely corrupted
- Green tint due to wrong byte positions

**FIX APPLIED**:
```wgsl
// AFTER (FIXED):
@group(0) @binding(1) var<storage, read_write> y_plane: array<atomic<u32>>;
let y_word_idx = y_linear / 4u;
let y_byte_pos = y_linear % 4u;
let y_shifted = y_value << (y_byte_pos * 8u);
atomicOr(&y_plane[y_word_idx], y_shifted);  // Packs 4 bytes per u32
```

Also fixed in Rust:
- Zero-initialize buffers (required for atomicOr)
- Pad buffer sizes to u32 boundaries
- Truncate output to exact NV12 sizes

### 7.3 Architectural Findings

**Current architecture is fundamentally flawed for performance**:

1. Creates separate GPU context (device/queue) from renderer
2. Performs GPU conversion AFTER CPU readback of RGBA
3. Then reads NV12 back to CPU (double readback!)
4. Blocking wait serializes all frames

**Ideal architecture**:
```
GPU Render (RGBA) → GPU Convert (same context) → GPU Readback (NV12 only)
```

Benefits of ideal:
- Single GPU context (no resource duplication)
- No intermediate CPU copy
- Maintains pipelining
- Reduces PCIe bandwidth by 62.5%

---

## 8. Recommended Solution

### 8.1 Short-term Fix (VERIFIED ✅)

**Color corruption fix** - Completed & Verified 2026-01-15:
- Fixed byte packing in shader.wgsl using atomic operations
- Updated Rust code to zero-initialize buffers and handle padding
- GPU conversion remains disabled by default (`CAP_GPU_FORMAT_CONVERSION=0`)
- **VERIFIED**: Colors are correct, no green tint or artifacts
- **ISSUE**: Performance still degraded (527.9s vs 39s baseline)

### 8.2 Medium-term Solution

**Consider VideoToolbox `VTPixelTransferSession`** for macOS:
- Apple's hardware-accelerated format converter
- Already exists in `crates/frame-converter/src/videotoolbox.rs`
- Would integrate better with VideoToolbox encoder
- No blocking issues (designed for frame-by-frame processing)

### 8.3 Long-term Architecture

**Integrate NV12 conversion into rendering pipeline BEFORE GPU readback**:

```
Current (broken):
  GPU Render → GPU Readback (RGBA) → GPU Convert → GPU Readback (NV12) → Encode

Target:
  GPU Render → GPU Convert (same context) → GPU Readback (NV12) → Encode
```

Required changes:
1. Refactor `RGBAToNV12` to accept external device/queue
2. Add texture input support (not just byte array)
3. Integrate into `finish_encoder()` in frame_pipeline.rs
4. Update `PipelinedGpuReadback` for NV12 buffer sizing

### 8.4 Implementation Plan

**Phase 1** (VERIFIED ✅): Fix color corruption
- [x] Fix shader byte packing
- [x] Update Rust buffer handling
- [x] Verify on macOS with `CAP_GPU_FORMAT_CONVERSION=1` - Colors correct, performance degraded

**Phase 2** (IN PROGRESS): Address performance
- Option A: Try `spawn_blocking` wrapper (quick test)
- Option B: Evaluate VideoToolbox path (medium effort)
- Option C: Full pipeline integration (high effort, best result)

**Phase 3** (Future): Full optimization
- Integrate conversion before readback
- Share GPU context with renderer
- Achieve target 35-55% performance improvement

---

## 9. Appendix

### 9.1 Test Commands

```bash
# Enable GPU conversion for testing
CAP_GPU_FORMAT_CONVERSION=1 pnpm dev:desktop

# Disable GPU conversion (default)
CAP_GPU_FORMAT_CONVERSION=0 pnpm dev:desktop

# Check output video format
ffprobe -v error -select_streams v:0 -show_entries stream=pix_fmt,color_range,color_space output.mp4
```

### 9.2 Relevant Log Messages

```
# GPU converter initialized
"GPU RGBA→NV12 converter initialized - using GPU format conversion"

# Frame conversion active
"GPU RGBA→NV12 conversion active width=X height=Y y_size=Z uv_size=W"

# Fallback to CPU
"GPU converter initialization failed - falling back to CPU conversion"
```

### 9.3 NV12 Format Reference

```
NV12 Memory Layout:
┌─────────────────────────────┐
│ Y Plane (width × height)    │  Full resolution, 1 byte/pixel
│ Y00 Y01 Y02 Y03 ...         │
│ Y10 Y11 Y12 Y13 ...         │
│ ...                         │
├─────────────────────────────┤
│ UV Plane (width × height/2) │  Half vertical resolution
│ U00 V00 U01 V01 ...         │  Interleaved U,V pairs
│ U10 V10 U11 V11 ...         │
│ ...                         │
└─────────────────────────────┘

Total size: width × height × 1.5 bytes
```

### 9.4 BT.709 Color Matrix Reference

```
Full range (0-255):
Y  = 0.2126×R + 0.7152×G + 0.0722×B
Cb = -0.1146×R - 0.3854×G + 0.5×B + 128
Cr = 0.5×R - 0.4542×G - 0.0458×B + 128

Limited range (16-235 for Y, 16-240 for UV):
Y  = 16 + 65.481×R + 128.553×G + 24.966×B
Cb = 128 - 37.797×R - 74.203×G + 112×B
Cr = 128 + 112×R - 93.786×G - 18.214×B
```

---

## 10. Final Conclusion (2026-01-15)

### Decision: ABANDON Custom GPU Approach

After thorough investigation and external research, the custom WGSL GPU shader approach is **abandoned** in favor of Apple-native APIs.

### Rationale

1. **Architectural Flaw is Fundamental**: The 13x performance regression stems from:
   - Separate GPU context (device/queue)
   - Blocking operations in async context
   - Double GPU→CPU readback

   Fixing this requires a complete rewrite, not incremental improvements.

2. **Apple Provides Superior Alternatives**:
   - VideoToolbox encoder can accept BGRA directly (no conversion needed)
   - `VTPixelTransferSession` provides hardware-accelerated conversion
   - Both are maintained by Apple and optimized for the platform

3. **Cost/Benefit Analysis**:
   - Custom GPU fix: High effort, high risk, medium gain
   - Apple-native APIs: Low-medium effort, low risk, high gain

### Code Disposition

The custom GPU converter code is **preserved but disabled**:
- `CAP_GPU_FORMAT_CONVERSION` defaults to `false`
- Code remains in `crates/gpu-converters/src/rgba_nv12/` for reference
- May be useful for Windows implementation (no VTPixelTransfer available)

### Next Steps

1. **S05**: Test BGRA direct input to VideoToolbox encoder
2. **S06**: Implement `VTPixelTransferSession` if S05 insufficient
3. **S07**: Benchmark and validate 50-55 fps target

### Lessons Learned

1. Always evaluate platform-native APIs before custom implementations
2. Architecture matters more than algorithm speed
3. Research before extensive coding saves time
4. Fail fast, pivot decisively when approach is fundamentally flawed

---

**Investigation Complete. See `main.md` Section 14 for the strategic pivot details.**
