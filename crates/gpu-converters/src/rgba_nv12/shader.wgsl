@group(0) @binding(0) var rgba_input: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> y_plane: array<u32>;
@group(0) @binding(2) var<storage, read_write> uv_plane: array<u32>;
@group(0) @binding(3) var<uniform> dimensions: vec2<u32>;

fn rgb_to_y(r: f32, g: f32, b: f32) -> u32 {
    let y_f = 16.0 + 65.481 * r + 128.553 * g + 24.966 * b;
    return u32(clamp(y_f, 0.0, 255.0));
}

fn rgb_to_u(r: f32, g: f32, b: f32) -> u32 {
    let u_f = 128.0 - 37.797 * r - 74.203 * g + 112.0 * b;
    return u32(clamp(u_f, 0.0, 255.0));
}

fn rgb_to_v(r: f32, g: f32, b: f32) -> u32 {
    let v_f = 128.0 + 112.0 * r - 93.786 * g - 18.214 * b;
    return u32(clamp(v_f, 0.0, 255.0));
}

@compute
@workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pos = global_id.xy;
    let dims = dimensions;

    if (pos.x >= dims.x || pos.y >= dims.y) {
        return;
    }

    let rgba = textureLoad(rgba_input, pos, 0);
    let r = rgba.r;
    let g = rgba.g;
    let b = rgba.b;

    let y_value = rgb_to_y(r, g, b);
    let y_idx = pos.y * dims.x + pos.x;
    y_plane[y_idx] = y_value;

    if (pos.x % 2u == 0u && pos.y % 2u == 0u) {
        let rgba00 = textureLoad(rgba_input, pos, 0);
        let rgba10 = textureLoad(rgba_input, pos + vec2<u32>(1u, 0u), 0);
        let rgba01 = textureLoad(rgba_input, pos + vec2<u32>(0u, 1u), 0);
        let rgba11 = textureLoad(rgba_input, pos + vec2<u32>(1u, 1u), 0);

        let avg_r = (rgba00.r + rgba10.r + rgba01.r + rgba11.r) * 0.25;
        let avg_g = (rgba00.g + rgba10.g + rgba01.g + rgba11.g) * 0.25;
        let avg_b = (rgba00.b + rgba10.b + rgba01.b + rgba11.b) * 0.25;

        let u_value = rgb_to_u(avg_r, avg_g, avg_b);
        let v_value = rgb_to_v(avg_r, avg_g, avg_b);

        let uv_row = pos.y / 2u;
        let uv_base = uv_row * dims.x + pos.x;
        uv_plane[uv_base] = u_value;
        uv_plane[uv_base + 1u] = v_value;
    }
}
