# Story S06: VTPixelTransferSession Implementation

**Status:** COMPLETE - Implementation ready for testing
**Date:** 2026-01-15
**Executor:** Claude Code

## Overview

Implemented Apple's hardware-accelerated `VTPixelTransferSession` for RGBA→NV12 format conversion to replace the CPU-based `sws_scale` that was consuming 64.8% of export time.

## Implementation Summary

### Files Modified

1. **`crates/frame-converter/src/videotoolbox.rs`**
   - Added `K_CV_PIXEL_FORMAT_TYPE_32_RGBA` constant (0x52474241)
   - Added `new_rgba_to_nv12()` constructor for simplified initialization
   - Added `convert_raw_rgba_to_nv12()` method accepting raw byte buffers
   - Added `extract_nv12_planes()` helper for extracting Y and UV planes from CVPixelBuffer

2. **`crates/export/src/mp4.rs`**
   - Replaced `RGBAToNV12` GPU converter with `VideoToolboxConverter`
   - Added `CAP_VIDEOTOOLBOX_CONVERSION` environment variable (enabled by default)
   - Added platform-specific conditional compilation for macOS
   - Updated logging prefixes to [T002-S06]
   - Added stub implementation for non-macOS platforms

3. **`crates/export/Cargo.toml`**
   - Added `cap-frame-converter` dependency

## Key Technical Details

### VideoToolbox Integration

The implementation adds a new public API to `VideoToolboxConverter`:

```rust
pub fn convert_raw_rgba_to_nv12(
    &self,
    rgba_data: &[u8],
    width: u32,
    height: u32,
    stride: usize,
) -> Result<(Vec<u8>, Vec<u8>), ConvertError>
```

This method:
1. Creates a CVPixelBuffer from raw RGBA bytes using `CVPixelBufferCreateWithBytes`
2. Creates a destination NV12 CVPixelBuffer
3. Calls `VTPixelTransferSessionTransferImage` for hardware acceleration
4. Extracts Y and UV planes from the output buffer
5. Returns them as separate vectors matching the GPU converter interface

### Environment Variable

- **Variable:** `CAP_VIDEOTOOLBOX_CONVERSION`
- **Default:** `true` (enabled)
- **Disable with:** `CAP_VIDEOTOOLBOX_CONVERSION=0` or `CAP_VIDEOTOOLBOX_CONVERSION=false`
- **Platform:** macOS only (always disabled on other platforms)

### Fallback Behavior

If VideoToolbox initialization fails:
1. Logs warning with error details
2. Falls back to existing CPU-based `sws_scale` path
3. Export continues normally with reduced performance

### Memory Management

The implementation properly manages CVPixelBuffer memory:
- Uses `CVPixelBufferCreateWithBytes` with no-op release callback (data owned by Rust)
- Releases input and output buffers with `CVPixelBufferRelease`
- Locks/unlocks buffers during plane extraction
- Copies planes row-by-row to handle stride differences

## Changes from GPU Converter

| Aspect | GPU Converter (S03/S04) | VideoToolbox (S06) |
|--------|-------------------------|-------------------|
| Device | Separate wgpu device | System VideoToolbox |
| Queue | Separate command queue | Apple's internal queue |
| Initialization | Async (shader compilation) | Sync (session creation) |
| Conversion API | `convert(&[u8], u32, u32)` | `convert_raw_rgba_to_nv12(&[u8], u32, u32, usize)` |
| Error Type | Custom GPU error | `ConvertError` |
| Platform | Cross-platform (wgpu) | macOS only |
| Performance | 13x regression | Expected 2-3x speedup |

## Code Changes Detail

### 1. Added RGBA Pixel Format Constant

```rust
const K_CV_PIXEL_FORMAT_TYPE_32_RGBA: u32 = 0x52474241;
```

The renderer outputs RGBA format, so this constant was needed (previously only BGRA and ARGB were defined).

### 2. Added Simplified Constructor

```rust
pub fn new_rgba_to_nv12(width: u32, height: u32) -> Result<Self, ConvertError> {
    // Creates session for RGBA→NV12 conversion without full ConversionConfig
}
```

This avoids needing to construct a full `ConversionConfig` with FFmpeg pixel types.

### 3. Added Raw Byte Conversion Method

The core conversion method that:
- Accepts raw RGBA byte buffer with explicit stride
- Creates CVPixelBuffer with proper format (RGBA)
- Performs hardware-accelerated conversion to NV12
- Extracts planes and returns as separate vectors
- Tracks conversion count for logging
- Verifies hardware usage on first conversion

### 4. Added Plane Extraction Helper

```rust
fn extract_nv12_planes(
    &self,
    pixel_buffer: CVPixelBufferRef,
) -> Result<(Vec<u8>, Vec<u8>), ConvertError>
```

Safely extracts Y and UV planes from NV12 CVPixelBuffer:
- Locks buffer for reading
- Validates plane count (must be 2 for NV12)
- Copies each plane row-by-row (handles stride)
- Unlocks buffer
- Returns owned Vec data

### 5. Updated mp4.rs Integration

Replaced GPU converter initialization:
```rust
#[cfg(target_os = "macos")]
let converter: Option<Arc<VideoToolboxConverter>> = if videotoolbox_conversion_enabled() {
    match VideoToolboxConverter::new_rgba_to_nv12(output_size.0, output_size.1) {
        Ok(converter) => {
            info!("[T002-S06] VideoToolbox RGBA→NV12 converter initialized - using hardware acceleration");
            Some(Arc::new(converter))
        }
        Err(e) => {
            warn!(error = %e, "[T002-S06] VideoToolbox initialization failed - falling back to CPU conversion");
            None
        }
    }
} else {
    debug!("[T002-S06] VideoToolbox conversion disabled via CAP_VIDEOTOOLBOX_CONVERSION");
    None
};
```

Updated conversion call:
```rust
match conv.convert_raw_rgba_to_nv12(
    &rgba_data,
    frame.width,
    frame.height,
    frame.width as usize * 4,
) {
    Ok((y_plane, uv_plane)) => {
        // Package NV12 data for encoder
    }
    Err(e) => {
        warn!("[T002-S06] VideoToolbox conversion failed for frame");
        return Err(format!("[T002-S06] VideoToolbox conversion failed: {e}"));
    }
}
```

## Testing Plan (S07)

### Baseline Comparison

Use the same test recording from S05:
- **Resolution:** 3840x2160 (4K)
- **Target FPS:** 60
- **Total Frames:** 7743
- **Baseline Export FPS:** 38.6 fps
- **Baseline sws_scale Time:** 130.2s (64.8% of export)

### Expected Results

| Metric | Baseline (S05) | Target (S06) | Improvement |
|--------|---------------|--------------|-------------|
| Export FPS | 38.6 fps | 50-55 fps | +30-42% |
| Total Time | 200.8s | ~141-155s | ~46-60s faster |
| Conversion Method | sws_scale (CPU) | VTPixelTransfer (HW) | Hardware accelerated |

### Test Procedure

1. Enable VideoToolbox conversion (default)
2. Export the baseline test recording
3. Monitor logs for `[T002-S06]` prefixed messages
4. Verify "hardware acceleration confirmed" log appears
5. Compare export FPS to baseline
6. Test fallback: disable with `CAP_VIDEOTOOLBOX_CONVERSION=0`
7. Verify graceful fallback to CPU path

### Success Criteria

- Export FPS ≥ 50 fps (30% improvement minimum)
- No visual quality regression
- Hardware acceleration confirmed via logs
- Graceful fallback on initialization failure
- Memory usage similar to baseline (~250MB)

## Potential Issues & Mitigations

### Issue 1: CVPixelBuffer Stride Mismatch

**Risk:** If CVPixelBuffer output stride doesn't match expected width, plane extraction could fail.

**Mitigation:** `extract_nv12_planes()` handles stride differences by copying row-by-row instead of bulk copy.

### Issue 2: VideoToolbox Session Creation Failure

**Risk:** VTPixelTransferSessionCreate might fail on some Macs.

**Mitigation:** Fallback to CPU path is built-in. Failure is logged but doesn't break export.

### Issue 3: RGBA Format Not Supported

**Risk:** Some Macs might not support RGBA input format.

**Mitigation:** If format is unsupported, initialization will fail and fall back to CPU path. Could add runtime format check in future.

### Issue 4: Performance Not as Expected

**Risk:** VTPixelTransferSession might not be as fast as anticipated.

**Mitigation:** Baseline established in S05 provides clear comparison point. If insufficient, can investigate further optimizations or alternative approaches.

## Next Steps

1. **S07 Benchmarking:**
   - Test with baseline recording
   - Measure export FPS improvement
   - Document results
   - Compare to theoretical maximum (109.6 fps without conversion)

2. **If Performance Insufficient:**
   - Profile VTPixelTransferSession overhead
   - Consider batch conversion of multiple frames
   - Investigate IOSurface-based zero-copy paths

3. **Future Enhancements:**
   - Support BGRA input directly (skip RGBA extraction)
   - Batch multiple frame conversions
   - Add Windows equivalent (Direct3D 11 pixel format conversion)

## Lessons Learned

1. Apple's native APIs are better integrated than custom implementations
2. Raw byte API needed for export pipeline (no ffmpeg::frame::Video)
3. Proper memory management critical with Core Foundation types
4. Fallback paths essential for robustness
5. Platform-specific code needs stub implementations for cross-compilation

## References

- **S05 Baseline Report:** `baseline-report.md`
- **S04 Investigation:** `s04-investigation-report.md` (GPU converter failure analysis)
- **VideoToolbox Existing Code:** `crates/frame-converter/src/videotoolbox.rs`
- **Export Pipeline:** `crates/export/src/mp4.rs`
