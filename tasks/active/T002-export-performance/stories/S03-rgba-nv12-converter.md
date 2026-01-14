# S03 - Implement RGBAToNV12 GPU Converter

## Overview
- **Complexity:** Medium
- **Estimated Time:** 4-6 hours
- **Lines Added:** ~200-250
- **Files Changed:** 3 new files, 1 modified

## Objective
Create a GPU compute shader converter that transforms RGBA textures to NV12 format, following the existing `yuyv_nv12` pattern for NV12 output.

## Technical Design

### Module Structure
Create `crates/gpu-converters/src/rgba_nv12/`:
- `mod.rs` - Rust implementation
- `shader.wgsl` - WGSL compute shader

### NV12 Format Layout
```
NV12 Memory Layout:
┌────────────────────────┐
│                        │
│     Y Plane            │  width × height bytes
│     (luminance)        │  1 byte per pixel
│                        │
├────────────────────────┤
│   UV Plane             │  width × (height/2) bytes
│   (interleaved U,V)    │  2 bytes per 2×2 block
└────────────────────────┘

Total: 1.5 bytes per pixel (vs RGBA's 4 bytes)
```

### Bind Group Layout
```
@group(0) @binding(0) - RGBA input texture (texture_2d<f32>)
@group(0) @binding(1) - Y plane output buffer (storage, read_write)
@group(0) @binding(2) - UV plane output buffer (storage, read_write)
@group(0) @binding(3) - Dimensions uniform (vec2<u32>)
```

### BT.709 Color Matrix

From RGBA [0,1] to YCbCr [16-235, 16-240]:
```
Y  = 16 + 65.481 × R + 128.553 × G + 24.966 × B
Cb = 128 - 37.797 × R - 74.203 × G + 112.0 × B
Cr = 128 + 112.0 × R - 93.786 × G - 18.214 × B
```

In normalized WGSL (input [0,1], output [0,255]):
```wgsl
let y  = 16.0 + 65.481 * r + 128.553 * g + 24.966 * b;
let cb = 128.0 - 37.797 * r - 74.203 * g + 112.0 * b;
let cr = 128.0 + 112.0 * r - 93.786 * g - 18.214 * b;
```

### Shader Design

**Key Logic:**
1. Each invocation processes one RGBA pixel at `(x, y)`
2. Always writes Y value to y_plane at `y * width + x`
3. Only writes UV on even-coordinate pixels to handle 2×2 subsampling:
   - UV written when `x % 2 == 0 && y % 2 == 0`
   - Average 4 pixels in 2×2 block for U and V
   - Write to uv_plane at `(y/2) * width + x`

**Workgroup Size:** `@workgroup_size(8, 8)` (standard for 2D processing)

### API Design

```rust
pub struct RGBAToNV12 {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl RGBAToNV12 {
    pub async fn new() -> Result<Self, GpuConverterError>;

    pub fn convert(
        &self,
        rgba_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(Vec<u8>, Vec<u8>), ConvertError>;
}
```

**Return Format:** `(y_plane, uv_plane)` where:
- `y_plane`: `Vec<u8>` of length `width × height`
- `uv_plane`: `Vec<u8>` of length `width × (height / 2)`

## Implementation Steps

### Step 1: Create shader file
**File:** `crates/gpu-converters/src/rgba_nv12/shader.wgsl`

Reference patterns:
- `yuyv_nv12/shader.wgsl` for NV12 output buffer structure
- `nv12_rgba/shader.wgsl` for BT.709 matrix (inverse direction)
- `rgba_uyvy/shader.wgsl` for RGBA texture sampling

### Step 2: Create Rust module
**File:** `crates/gpu-converters/src/rgba_nv12/mod.rs`

Follow `yuyv_nv12/mod.rs` pattern:
- Async `new()` with GPU device/adapter setup
- `convert()` that creates textures, bind groups, dispatches, reads back
- Input validation for even dimensions

### Step 3: Update lib.rs exports
**File:** `crates/gpu-converters/src/lib.rs`

Add:
```rust
mod rgba_nv12;
pub use rgba_nv12::RGBAToNV12;
```

### Step 4: Add tests
**File:** `crates/gpu-converters/src/rgba_nv12/mod.rs` (tests module)

Test cases:
- Known RGBA values → expected Y/UV output
- Dimension validation (reject odd dimensions)
- Round-trip with `NV12ToRGBA`

## Reference Patterns

### From yuyv_nv12/shader.wgsl (NV12 output)
```wgsl
@group(0) @binding(1) var<storage, read_write> y_plane: array<u32>;
@group(0) @binding(2) var<storage, read_write> uv_plane: array<u32>;

fn main(...) {
    let y_idx = pos.y * dimensions.x + pos.x;
    y_plane[y_idx] = pack4x8unorm(vec4(y_value, 0.0, 0.0, 0.0));

    if (pos.y % 2 == 0 && pos.x % 2 == 0) {
        let uv_idx = (pos.y / 2) * dimensions.x + pos.x;
        uv_plane[uv_idx] = pack4x8unorm(vec4(u_value, v_value, 0.0, 0.0));
    }
}
```

### From rgba_uyvy/shader.wgsl (RGBA input)
```wgsl
@group(0) @binding(0) var input_texture: texture_2d<f32>;

fn main(...) {
    let rgba = textureLoad(input_texture, pos, 0);
    let r = rgba.r;
    let g = rgba.g;
    let b = rgba.b;
}
```

## Testing Strategy

### Unit Tests
1. **Solid color test**: Pure red/green/blue RGBA → verify Y/UV values
2. **Gradient test**: RGBA gradient → verify smooth Y/UV output
3. **Dimension validation**: Assert error on odd width/height

### Round-trip Test
```rust
#[test]
fn test_round_trip() {
    let rgba_to_nv12 = RGBAToNV12::new().await?;
    let nv12_to_rgba = NV12ToRGBA::new().await?;

    let original_rgba = /* test image */;
    let (y, uv) = rgba_to_nv12.convert(&original_rgba, w, h)?;
    let restored_rgba = nv12_to_rgba.convert(&y, &uv, w, h)?;

    // Verify visual similarity (allow small precision loss)
    assert_similar(&original_rgba, &restored_rgba, tolerance);
}
```

### Performance Test
```rust
#[test]
fn benchmark_gpu_vs_cpu() {
    let start = Instant::now();
    // GPU conversion
    let gpu_time = start.elapsed();

    let start = Instant::now();
    // CPU conversion (for comparison)
    let cpu_time = start.elapsed();

    println!("GPU: {:?}, CPU: {:?}", gpu_time, cpu_time);
}
```

## Acceptance Criteria

- [ ] New `RGBAToNV12` converter following existing pattern
- [ ] Compute shader for RGBA→NV12 conversion (BT.709 color matrix)
- [ ] Handles even dimension requirement (NV12 constraint)
- [ ] Performance comparable to or better than CPU conversion
- [ ] Fallback to CPU if GPU unavailable (deferred to S04)
- [ ] Unit tests pass
- [ ] Round-trip test shows acceptable quality

## Checklist

- [ ] Create `crates/gpu-converters/src/rgba_nv12/mod.rs`
- [ ] Create `crates/gpu-converters/src/rgba_nv12/shader.wgsl`
- [ ] Implement WGSL compute shader with BT.709 matrix
- [ ] Implement Y plane output (full resolution)
- [ ] Implement UV plane output (half resolution, interleaved)
- [ ] Add input validation (even dimensions required)
- [ ] Update `crates/gpu-converters/src/lib.rs` to export
- [ ] Add unit tests
- [ ] Verify with cargo check/clippy
- [ ] Document color matrix in code structure (not comments)

## Risk Mitigations

| Risk | Mitigation |
|------|------------|
| Color accuracy | Use same BT.709 matrix as existing nv12_to_rgba |
| Performance | Follow proven yuyv_nv12 pattern |
| Even dimension | Add validation in convert() method |
| Fallback | CPU conversion remains (not removed in S03) |

## Code Locations Reference

| File | Purpose |
|------|---------|
| `crates/gpu-converters/src/yuyv_nv12/mod.rs` | Primary pattern (NV12 buffer output) |
| `crates/gpu-converters/src/yuyv_nv12/shader.wgsl` | Y/UV plane writing pattern |
| `crates/gpu-converters/src/rgba_uyvy/shader.wgsl` | RGBA input sampling pattern |
| `crates/gpu-converters/src/lib.rs` | Module exports |
| `crates/rendering/src/shaders/nv12_to_rgba.wgsl` | BT.709 matrix reference |
