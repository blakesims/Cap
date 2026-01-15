# S04 - Integrate GPU Conversion into Frame Pipeline

## Overview
- **Complexity:** Medium-High
- **Estimated Time:** ~6-8 hours
- **Dependencies:** S03 (RGBAToNV12 converter), S02 (format audit)
- **Files Changed:** 4-5 files across rendering, export, and media-info crates

## Objective
Integrate the RGBAToNV12 GPU converter into the frame pipeline so format conversion happens on the GPU before readback. This eliminates the CPU-based FFmpeg software scaler bottleneck and reduces PCIe bandwidth by ~62.5% (4 bytes/pixel RGBA → 1.5 bytes/pixel NV12).

## Background

### Current Flow (CPU Bottleneck)
```
GPU Render (RGBA texture)
    ↓
GPU Readback (RGBA buffer) → 4 bytes/pixel
    ↓
RenderedFrame { data: Vec<u8> }  [RGBA]
    ↓
FFmpeg software scaler (CPU) ❌ BOTTLENECK
    ↓
FFmpeg Frame (NV12)
    ↓
h264_videotoolbox encoder
```

### Target Flow (GPU Conversion)
```
GPU Render (RGBA texture)
    ↓
GPU Format Conversion (compute shader) ✅
    ↓
GPU Readback (NV12 buffer) → 1.5 bytes/pixel
    ↓
RenderedFrame { data: Vec<u8> }  [NV12]
    ↓
FFmpeg Frame (NV12 direct, no conversion)
    ↓
h264_videotoolbox encoder (with_external_conversion=true)
```

## Acceptance Criteria

- [ ] `RGBAToNV12` GPU converter integrated into frame pipeline
- [ ] GPU readback produces NV12 data instead of RGBA
- [ ] `RenderedFrame` correctly represents NV12 format (Y+UV planes concatenated)
- [ ] `VideoInfo::wrap_frame()` handles NV12 multi-plane frames
- [ ] H264 encoder receives pre-converted NV12 frames
- [ ] `with_external_conversion()` enabled to skip CPU scaler
- [ ] All existing export functionality preserved
- [ ] Graceful fallback if GPU converter fails
- [ ] No memory leaks or performance regressions
- [ ] Bandwidth reduction measurable (~60% less GPU→CPU data)

## Technical Design

### Key Design Decision: RenderedFrame NV12 Representation

**Chosen Approach:** Option A - Concatenated planes with metadata

```rust
pub struct RenderedFrame {
    pub data: Vec<u8>,           // Y plane + UV plane concatenated
    pub width: u32,
    pub height: u32,
    pub padded_bytes_per_row: u32,
    pub frame_number: u32,
    pub target_time_ns: u64,
    pub pixel_format: PixelFormat,  // NEW: RGBA or NV12
    pub y_plane_size: Option<usize>, // NEW: For NV12, size of Y plane
}
```

**Rationale:** Minimizes changes to downstream code. Single `data` field maintained. Helper methods can extract Y/UV slices.

### NV12 Memory Layout

```
Total size: width × height × 1.5 bytes

Y Plane (full resolution):
  - Size: width × height bytes
  - Layout: One Y value per pixel
  - Stored at: data[0..y_plane_size]

UV Plane (half resolution, interleaved):
  - Size: width × (height/2) bytes
  - Layout: U,V,U,V,... pairs for each 2×2 block
  - Stored at: data[y_plane_size..]
```

### Integration Architecture

```
frame_pipeline.rs                    mp4.rs
     │                                  │
RenderSession {                    VideoInfo {
  rgba_to_nv12: RGBAToNV12,         format: NV12,
  ...                               ...
}                                  }
     │                                  │
submit_readback()                  H264Encoder::builder()
  ├─ GPU render to RGBA texture      ├─ with_external_conversion()
  ├─ GPU convert RGBA→NV12           └─ build()
  └─ GPU readback NV12 buffers          │
     │                              wrap_frame()
RenderedFrame {                      ├─ Copy Y plane
  data: [Y|UV],                      └─ Copy UV plane
  pixel_format: NV12,
  y_plane_size: Some(w*h),
}
```

## Implementation Steps

### Phase 1: Infrastructure (Low Risk)

#### Step 1.1: Add PixelFormat enum

**File:** `crates/rendering/src/frame_pipeline.rs`

Add near RenderedFrame definition:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PixelFormat {
    #[default]
    Rgba,
    Nv12,
}
```

#### Step 1.2: Extend RenderedFrame struct

**File:** `crates/rendering/src/frame_pipeline.rs` (lines 333-341)

```rust
#[derive(Clone)]
pub struct RenderedFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub padded_bytes_per_row: u32,
    pub frame_number: u32,
    pub target_time_ns: u64,
    pub pixel_format: PixelFormat,     // NEW
    pub y_plane_size: Option<usize>,   // NEW: None for RGBA
}

impl RenderedFrame {
    pub fn y_plane(&self) -> &[u8] {
        match self.pixel_format {
            PixelFormat::Rgba => &self.data,
            PixelFormat::Nv12 => {
                let size = self.y_plane_size.unwrap_or(self.data.len());
                &self.data[..size]
            }
        }
    }

    pub fn uv_plane(&self) -> Option<&[u8]> {
        match self.pixel_format {
            PixelFormat::Rgba => None,
            PixelFormat::Nv12 => {
                let y_size = self.y_plane_size.unwrap_or(0);
                Some(&self.data[y_size..])
            }
        }
    }
}
```

#### Step 1.3: Update existing RenderedFrame construction

**File:** `crates/rendering/src/frame_pipeline.rs` (lines 75-82)

```rust
RenderedFrame {
    data: data_vec,
    padded_bytes_per_row,
    width,
    height,
    frame_number,
    target_time_ns,
    pixel_format: PixelFormat::Rgba,   // Existing path = RGBA
    y_plane_size: None,
}
```

### Phase 2: Media Info Layer (Medium Risk)

#### Step 2.1: Extend wrap_frame for NV12

**File:** `crates/media-info/src/lib.rs` (around lines 290-325)

The existing `wrap_frame` method needs to handle NV12's multi-plane format:

```rust
pub fn wrap_frame(&self, data: &[u8], pts: i64, stride: usize) -> ffmpeg::Frame {
    match self.pixel_format {
        ffmpeg::format::Pixel::NV12 => {
            self.wrap_nv12_frame(data, pts)
        }
        _ => {
            // Existing single-plane logic
        }
    }
}

fn wrap_nv12_frame(&self, data: &[u8], pts: i64) -> ffmpeg::Frame {
    let mut frame = ffmpeg::Frame::video(
        ffmpeg::format::Pixel::NV12,
        self.width,
        self.height,
    );
    frame.set_pts(Some(pts));

    let y_size = (self.width * self.height) as usize;
    let y_data = &data[..y_size];
    let uv_data = &data[y_size..];

    // Y plane
    let y_stride = frame.stride(0);
    let y_dst = frame.data_mut(0);
    for row in 0..self.height as usize {
        let src_start = row * self.width as usize;
        let dst_start = row * y_stride;
        y_dst[dst_start..dst_start + self.width as usize]
            .copy_from_slice(&y_data[src_start..src_start + self.width as usize]);
    }

    // UV plane (height/2 rows)
    let uv_stride = frame.stride(1);
    let uv_dst = frame.data_mut(1);
    let uv_height = self.height as usize / 2;
    for row in 0..uv_height {
        let src_start = row * self.width as usize;
        let dst_start = row * uv_stride;
        uv_dst[dst_start..dst_start + self.width as usize]
            .copy_from_slice(&uv_data[src_start..src_start + self.width as usize]);
    }

    frame
}
```

### Phase 3: Frame Pipeline Integration (High Risk)

#### Step 3.1: Add GPU converter to RenderSession

**File:** `crates/rendering/src/frame_pipeline.rs`

Add import at top:
```rust
use cap_gpu_converters::RGBAToNV12;
```

Modify `RenderSession` struct (around line 206):
```rust
pub struct RenderSession {
    pub textures: (wgpu::Texture, wgpu::Texture),
    texture_views: (wgpu::TextureView, wgpu::TextureView),
    pub current_is_left: bool,
    pub pipelined_readback: PipelinedGpuReadback,
    pub rgba_to_nv12: Option<RGBAToNV12>,   // NEW
}
```

#### Step 3.2: Initialize converter in RenderSession::new

**File:** `crates/rendering/src/frame_pipeline.rs`

In `RenderSession::new()` method, after creating pipelined_readback:

```rust
let rgba_to_nv12 = match RGBAToNV12::new().await {
    Ok(converter) => {
        tracing::info!("GPU RGBA→NV12 converter initialized");
        Some(converter)
    }
    Err(e) => {
        tracing::warn!(error = %e, "GPU converter unavailable, using fallback");
        None
    }
};
```

#### Step 3.3: Modify submit_readback for NV12 path

**File:** `crates/rendering/src/frame_pipeline.rs` (lines 135-195)

This is the core change. The function needs to:
1. Check if GPU converter is available
2. If yes: read RGBA texture, convert on GPU, readback NV12
3. If no: use existing RGBA path

```rust
pub fn submit_readback(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    uniforms: &ProjectUniforms,
    mut render_encoder: wgpu::CommandEncoder,
    rgba_to_nv12: Option<&RGBAToNV12>,   // NEW parameter
) -> Result<(), RenderingError>
```

The implementation needs to:
1. Copy RGBA texture to staging buffer (for GPU converter input)
2. Call `rgba_to_nv12.convert()` to get Y and UV planes
3. Store NV12 data in pending readback

**Alternative approach:** Modify the converter to work directly with textures rather than CPU buffers to avoid extra copy.

### Phase 4: Export Layer Integration (Medium Risk)

#### Step 4.1: Update VideoInfo format

**File:** `crates/export/src/mp4.rs` (lines 75-77)

```rust
let mut video_info = if gpu_conversion_enabled {
    VideoInfo::from_raw(RawVideoFormat::Nv12, output_size.0, output_size.1, fps)
} else {
    VideoInfo::from_raw(RawVideoFormat::Rgba, output_size.0, output_size.1, fps)
};
```

#### Step 4.2: Enable external conversion

**File:** `crates/export/src/mp4.rs` (lines 94-96)

```rust
let encoder_builder = H264Encoder::builder(video_info)
    .with_bpp(self.effective_bpp());

let encoder_builder = if gpu_conversion_enabled {
    encoder_builder.with_external_conversion()
} else {
    encoder_builder
};

let encoder = encoder_builder.build(o)?;
```

#### Step 4.3: Update wrap_frame call

**File:** `crates/export/src/mp4.rs` (lines 220-224)

```rust
let video = if frame.pixel_format == PixelFormat::Nv12 {
    video_info.wrap_nv12_frame(&frame.data, frame_number as i64)
} else {
    video_info.wrap_frame(
        &frame.data,
        frame_number as i64,
        frame.padded_bytes_per_row as usize,
    )
};
```

### Phase 5: Validation & Fallback

#### Step 5.1: Feature flag for rollback

**File:** `crates/rendering/Cargo.toml` or runtime config

```rust
pub fn gpu_conversion_enabled() -> bool {
    std::env::var("CAP_GPU_FORMAT_CONVERSION")
        .map(|v| v != "0" && v.to_lowercase() != "false")
        .unwrap_or(true)  // Enabled by default
}
```

#### Step 5.2: Graceful fallback path

If GPU converter fails at any point:
1. Log warning with context
2. Fall back to RGBA readback + CPU conversion
3. Continue export without interruption

## Code Locations Reference

| File | Line(s) | Purpose | Change Type |
|------|---------|---------|-------------|
| `crates/rendering/src/frame_pipeline.rs` | 333-341 | RenderedFrame struct | Extend |
| `crates/rendering/src/frame_pipeline.rs` | 206 | RenderSession struct | Add field |
| `crates/rendering/src/frame_pipeline.rs` | 135-195 | submit_readback | Major modify |
| `crates/rendering/src/frame_pipeline.rs` | 75-82 | RenderedFrame construction | Update |
| `crates/media-info/src/lib.rs` | 290-325 | wrap_frame | Extend for NV12 |
| `crates/export/src/mp4.rs` | 75-77 | VideoInfo format | Conditional |
| `crates/export/src/mp4.rs` | 94-96 | H264Encoder builder | Add external_conversion |
| `crates/export/src/mp4.rs` | 220-224 | wrap_frame call | Conditional |
| `crates/gpu-converters/src/lib.rs` | - | Module export | Verify accessible |

## Risks and Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| GPU converter fails on some systems | Medium | Runtime fallback to CPU path |
| Stride/padding mismatch with FFmpeg | High | Careful stride calculation, testing |
| Breaking existing export | High | Feature flag, extensive testing |
| Memory layout bugs | Medium | Unit tests for Y/UV plane extraction |
| Performance regression | Low | Benchmark before/after |
| Odd dimension frames | Low | Validation already in S03 converter |

## Testing Strategy

### Unit Tests
- [ ] RenderedFrame Y/UV plane extraction
- [ ] VideoInfo wrap_nv12_frame output validation
- [ ] Pixel format enum serialization

### Integration Tests
- [ ] Export 1080p video with GPU conversion
- [ ] Export 4K video with GPU conversion
- [ ] Export with GPU converter disabled (fallback)
- [ ] Verify output plays correctly in various players

### Validation
- [ ] ffprobe shows correct pixel format
- [ ] Visual quality matches CPU path
- [ ] File size similar to CPU path (same codec)
- [ ] No corruption or artifacts

## Checklist

- [ ] Add PixelFormat enum to frame_pipeline.rs
- [ ] Extend RenderedFrame with pixel_format and y_plane_size fields
- [ ] Add y_plane() and uv_plane() helper methods
- [ ] Update existing RenderedFrame construction for RGBA
- [ ] Extend wrap_frame in media-info for NV12
- [ ] Add RGBAToNV12 to RenderSession
- [ ] Initialize converter in RenderSession::new
- [ ] Modify submit_readback for NV12 path
- [ ] Update mp4.rs VideoInfo format
- [ ] Enable with_external_conversion()
- [ ] Update wrap_frame call in mp4.rs
- [ ] Add feature flag/env var for GPU conversion
- [ ] Implement fallback path
- [ ] Test on various resolutions
- [ ] Verify Clippy compliance (no code comments!)
- [ ] Run existing export tests
- [ ] Measure bandwidth reduction
