# S02 - Format Conversion Flow Audit

## Overview
- **Complexity:** Low (research only)
- **Estimated Time:** ~2 hours
- **Type:** Research/Documentation
- **Dependencies:** None
- **Outputs:** Verified format flow, confirmed optimization strategy for S03/S04

## Objective
Document the complete format conversion flow from decoder to encoder, identify all conversion points, and verify the optimization strategy for S03/S04.

## Complete Format Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           CURRENT EXPORT PIPELINE                            │
└─────────────────────────────────────────────────────────────────────────────┘

[1. HARDWARE DECODE]
    AVAssetReader
    ├── Output: NV12 (most common from VideoToolbox)
    │           BGRA, RGBA, or YUV420P (fallback)
    └── Location: avassetreader.rs:78-199

            │
            │ DecodedFrame (PixelFormat: NV12/RGBA/YUV420P)
            ▼

[2. DISPLAY LAYER - YUV→RGBA GPU CONVERSION]
    DisplayLayer.prepare()
    ├── If NV12:    yuv_converter.convert_nv12() → GPU compute shader
    ├── If YUV420P: yuv_converter.convert_yuv420p() → GPU compute shader
    ├── If RGBA:    Direct queue.write_texture() → No conversion
    └── Location: display.rs:109-392

            │
            │ RGBA Texture (GPU)
            ▼

[3. GPU RENDERING/COMPOSITING]
    All rendering layers composite onto Rgba8Unorm texture
    ├── Background, cursor, watermark, effects, etc.
    └── Location: lib.rs render_video_to_channel()

            │
            │ RGBA Texture (GPU) - frame_pipeline.rs:225
            ▼

[4. GPU READBACK] ⚠️ BANDWIDTH BOTTLENECK
    PipelinedGpuReadback.submit_readback()
    ├── copy_texture_to_buffer() - RGBA (4 bytes/pixel)
    ├── 4K60: 3840×2160×4×60 = ~1.98 GB/s bandwidth
    └── Location: frame_pipeline.rs:155-171

            │
            │ RGBA Vec<u8> (CPU)
            ▼

[5. ENCODER INPUT WRAPPING]
    VideoInfo::wrap_frame()
    ├── Wraps RGBA bytes into ffmpeg::frame::Video
    └── Location: mp4.rs:219-226

            │
            │ frame::Video (Pixel::RGBA)
            ▼

[6. H264 ENCODER] ⚠️ CPU BOTTLENECK
    H264Encoder.queue_frame()
    ├── encoder_supports_input_format(RGBA)? → Usually NO
    ├── If NO: Creates FFmpeg sws_scale converter
    │   └── RGBA → NV12 conversion on CPU (slow!)
    ├── Location: h264.rs:196-289, converter at h264.rs:248-258
    └── Output: NV12 frames to h264_videotoolbox

            │
            │ NV12 frame::Video
            ▼

[7. HARDWARE ENCODE]
    h264_videotoolbox
    └── Native NV12 input, hardware encoding
```

## Conversion Points Summary

| Stage | Input Format | Output Format | Location | Hardware |
|-------|--------------|---------------|----------|----------|
| Decode | H264 bitstream | NV12 | avassetreader.rs | GPU (VT) |
| Display prep | NV12 | RGBA | display.rs | GPU (compute) |
| Render | RGBA | RGBA | lib.rs | GPU (render) |
| Readback | RGBA | RGBA | frame_pipeline.rs | PCIe |
| **Encode prep** | **RGBA** | **NV12** | **h264.rs** | **CPU** |
| Encode | NV12 | H264 | h264_videotoolbox | GPU (VT) |

## Key Findings

### 1. Decoder Output Format
AVAssetReader uses VideoToolbox which outputs NV12 natively (avassetreader.rs:120-147). This confirms NV12 is the optimal encoder input format.

### 2. Dual GPU Conversions
Currently the pipeline does NV12→RGBA (at decode for compositing) and conceptually back to NV12 (at encode). This represents an opportunity to optimize the output path.

### 3. CPU Bottleneck Location
The CPU bottleneck is confirmed at h264.rs:248-258 where `sws_scale` creates an FFmpeg software scaler for RGBA→NV12 conversion.

### 4. `with_external_conversion()` API Ready
h264.rs:83-86 and h264.rs:233-240 show a flag that skips creating the internal FFmpeg software scaler. When `true`, expects frames already in NV12.

### 5. `queue_preconverted_frame()` Exists
h264.rs:466-490 has a method that accepts pre-converted frames, ready for direct NV12 input.

### 6. No RGBA→NV12 GPU Converter
gpu-converters/ has NV12→RGBA but not the reverse. Must implement in S03.

## `with_external_conversion()` Analysis

**Current behavior (external_conversion = false):**
```rust
let converter = if needs_pixel_conversion || needs_scaling {
    ffmpeg::software::scaling::Context::get(...)
}
```

**With external_conversion = true:**
```rust
let converter = if external_conversion {
    None
} else if needs_pixel_conversion || needs_scaling {
    ...
}
```

**Usage requirements:**
- Must call `queue_preconverted_frame()` instead of `queue_frame()`
- Frame must already be in encoder's expected format (NV12)
- Frame dimensions must match encoder's output dimensions

## Existing GPU Converters

| Converter | Direction | Location |
|-----------|-----------|----------|
| nv12_rgba | NV12 → RGBA | gpu-converters/src/nv12_rgba/ |
| uyvy_nv12 | UYVY → NV12 | gpu-converters/src/uyvy_nv12/ |
| yuyv_nv12 | YUYV → NV12 | gpu-converters/src/yuyv_nv12/ |
| rgba_uyvy | RGBA → UYVY | gpu-converters/src/rgba_uyvy/ |
| **rgba_nv12** | **RGBA → NV12** | **MISSING - implement in S03** |

## Optimized Pipeline (Target)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          OPTIMIZED PIPELINE (TARGET)                         │
└─────────────────────────────────────────────────────────────────────────────┘

[Steps 1-3: Same as current]

            │
            │ RGBA Texture (GPU)
            ▼

[4. GPU RGBA→NV12 CONVERSION] ✅ NEW (S03)
    RgbaToNv12Converter (compute shader)
    ├── Converts RGBA texture to NV12 planes on GPU
    ├── Y plane: Full resolution
    ├── UV plane: Half resolution (interleaved)
    └── Location: NEW - gpu-converters/src/rgba_nv12/

            │
            │ NV12 Texture (GPU)
            ▼

[5. GPU READBACK] ✅ REDUCED BANDWIDTH (S04)
    PipelinedGpuReadback (NV12 path)
    ├── Read Y plane + UV plane (1.5 bytes/pixel)
    ├── 4K60: 3840×2160×1.5×60 = ~0.74 GB/s (62.5% reduction)
    └── Location: frame_pipeline.rs (modified)

            │
            │ NV12 data (CPU)
            ▼

[6. H264 ENCODER] ✅ DIRECT PASSTHROUGH (S04)
    H264Encoder.queue_preconverted_frame()
    ├── with_external_conversion(true) - skip internal converter
    ├── Direct NV12 → h264_videotoolbox
    └── Location: h264.rs:466-490
```

## Assumptions Verified

| Original Assumption | Status | Actual Finding |
|---------------------|--------|----------------|
| AVAssetReader outputs NV12 | VERIFIED | Yes, from VideoToolbox (avassetreader.rs:120-147) |
| Renderer needs RGBA | VERIFIED | Yes, all compositing is Rgba8Unorm (frame_pipeline.rs:225) |
| h264_videotoolbox prefers NV12 | VERIFIED | Yes, default output_format (h264.rs:202) |
| CPU conversion is bottleneck | VERIFIED | Yes, sws_scale at h264.rs:248-258 |

## Questions Answered

1. **What format does AVAssetReader output?**
   → NV12 from VideoToolbox hardware decoder (most common)

2. **Does renderer always need RGBA?**
   → Yes, all compositing uses Rgba8Unorm textures for GPU rendering

3. **Can we detect passthrough cases (no effects) and skip RGBA?**
   → Not easily; the renderer always produces RGBA. Would require significant architecture changes.

4. **What format does h264_videotoolbox prefer?**
   → NV12 (confirmed as default output_format in h264.rs:202)

## Recommendations for S03/S04

### S03 - RGBA→NV12 Shader
Follow `nv12_rgba/` pattern but reverse the math (BT.709):
```
Y  = 16 + 65.481 R + 128.553 G + 24.966 B
Cb = 128 - 37.797 R - 74.203 G + 112.0 B
Cr = 128 + 112.0 R - 93.786 G - 18.214 B
```

### S04 - Integration Points
1. `frame_pipeline.rs:submit_readback()` - Add NV12 texture and conversion step
2. `mp4.rs` - Pass NV12 VideoInfo to encoder, use `with_external_conversion(true)`
3. `h264.rs` - Verify `queue_preconverted_frame()` works with VideoToolbox

### Feature Flag
Add `gpu_nv12_conversion` flag in export settings for rollback capability

## Critical Files for S03/S04

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `crates/gpu-converters/src/rgba_nv12/` | New converter | Create (S03) |
| `crates/rendering/src/frame_pipeline.rs` | GPU readback | Add NV12 path (S04) |
| `crates/enc-ffmpeg/src/video/h264.rs` | H264 encoder | Use external_conversion (S04) |
| `crates/export/src/mp4.rs` | Export orchestration | Configure NV12 path (S04) |
| `crates/rendering/src/yuv_converter.rs` | Reference | Pattern for GPU YUV conversion |

## Acceptance Criteria

- [x] Document complete format flow from decoder to encoder
- [x] Identify all conversion points
- [x] Determine if any conversions are truly redundant (Answer: No easy passthrough available)
- [x] Understand `with_external_conversion()` usage
- [x] Verify optimization strategy for S03/S04
