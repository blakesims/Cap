use cap_project::{TextScalarKeyframe, TextSegment, TextVectorKeyframe, XY};

const BASE_TEXT_HEIGHT: f64 = 0.2;
const MAX_FONT_SIZE_PX: f32 = 256.0;

#[derive(Debug, Clone)]
pub struct PreparedText {
    pub content: String,
    pub bounds: [f32; 4],
    pub color: [f32; 4],
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: f32,
    pub italic: bool,
    pub opacity: f32,
}

fn parse_color(hex: &str) -> [f32; 4] {
    let color = hex.trim_start_matches('#');
    if color.len() == 6
        && let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&color[0..2], 16),
            u8::from_str_radix(&color[2..4], 16),
            u8::from_str_radix(&color[4..6], 16),
        )
    {
        return [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0];
    }

    [1.0, 1.0, 1.0, 1.0]
}

fn interpolate_text_vector(base: XY<f64>, keys: &[TextVectorKeyframe], time: f64) -> XY<f64> {
    if keys.is_empty() {
        return base;
    }

    let mut sorted = keys.to_vec();
    sorted.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if time <= sorted[0].time {
        return XY::new(sorted[0].x, sorted[0].y);
    }

    for i in 0..sorted.len() - 1 {
        let prev = &sorted[i];
        let next = &sorted[i + 1];

        if time >= prev.time && time <= next.time {
            let span = (next.time - prev.time).max(1e-6);
            let t = ((time - prev.time) / span).clamp(0.0, 1.0);
            return XY::new(
                prev.x + (next.x - prev.x) * t,
                prev.y + (next.y - prev.y) * t,
            );
        }
    }

    let last = sorted.last().unwrap();
    XY::new(last.x, last.y)
}

fn interpolate_text_scalar(base: f64, keys: &[TextScalarKeyframe], time: f64) -> f64 {
    if keys.is_empty() {
        return base;
    }

    let mut sorted = keys.to_vec();
    sorted.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if time <= sorted[0].time {
        return sorted[0].value;
    }

    for i in 0..sorted.len() - 1 {
        let prev = &sorted[i];
        let next = &sorted[i + 1];

        if time >= prev.time && time <= next.time {
            let span = (next.time - prev.time).max(1e-6);
            let t = ((time - prev.time) / span).clamp(0.0, 1.0);
            return (prev.value + (next.value - prev.value) * t).clamp(0.0, 1.0);
        }
    }

    sorted.last().unwrap().value.clamp(0.0, 1.0)
}

pub fn prepare_texts(
    output_size: XY<u32>,
    frame_time: f64,
    segments: &[TextSegment],
    hidden_indices: &[usize],
) -> Vec<PreparedText> {
    let mut prepared = Vec::new();
    let height_scale = if output_size.y == 0 {
        1.0
    } else {
        output_size.y as f32 / 1080.0
    };

    for (i, segment) in segments.iter().enumerate() {
        if !segment.enabled || hidden_indices.contains(&i) {
            continue;
        }

        if frame_time < segment.start || frame_time > segment.end {
            continue;
        }

        let relative_time = (frame_time - segment.start).max(0.0);

        let center_raw =
            interpolate_text_vector(segment.center, &segment.keyframes.position, relative_time);
        let center = XY::new(center_raw.x.clamp(0.0, 1.0), center_raw.y.clamp(0.0, 1.0));
        let size = XY::new(
            segment.size.x.clamp(0.01, 2.0),
            segment.size.y.clamp(0.01, 2.0),
        );
        let size_scale = (size.y / BASE_TEXT_HEIGHT).clamp(0.25, 4.0) as f32;

        let width = (size.x * output_size.x as f64).max(1.0) as f32;
        let height = (size.y * output_size.y as f64).max(1.0) as f32;
        let half_w = width / 2.0;
        let half_h = height / 2.0;

        let left = (center.x as f32 * output_size.x as f32 - half_w).max(0.0);
        let top = (center.y as f32 * output_size.y as f32 - half_h).max(0.0);
        let right = (left + width).min(output_size.x as f32);
        let bottom = (top + height).min(output_size.y as f32);

        let keyframe_opacity =
            interpolate_text_scalar(1.0, &segment.keyframes.opacity, relative_time);

        let fade_duration = segment.fade_duration.max(0.0);
        let mut opacity = keyframe_opacity as f32;
        if fade_duration > 0.0 {
            let time_since_start = (frame_time - segment.start).max(0.0);
            let time_until_end = (segment.end - frame_time).max(0.0);

            let fade_in = (time_since_start / fade_duration).min(1.0);
            let fade_out = (time_until_end / fade_duration).min(1.0);

            opacity *= (fade_in * fade_out) as f32;
        }

        prepared.push(PreparedText {
            content: segment.content.clone(),
            bounds: [left, top, right, bottom],
            color: parse_color(&segment.color),
            font_family: segment.font_family.clone(),
            font_size: ((segment.font_size * size_scale).max(1.0) * height_scale)
                .min(MAX_FONT_SIZE_PX),
            font_weight: segment.font_weight,
            italic: segment.italic,
            opacity,
        });
    }

    prepared
}

#[cfg(test)]
mod tests {
    use super::*;
    use cap_project::TextKeyframes;

    #[test]
    fn test_interpolate_text_scalar_empty() {
        let result = interpolate_text_scalar(0.5, &[], 1.0);
        assert_eq!(result, 0.5);
    }

    #[test]
    fn test_interpolate_text_scalar_single() {
        let keys = vec![TextScalarKeyframe {
            time: 0.0,
            value: 0.8,
        }];
        let result = interpolate_text_scalar(0.5, &keys, 1.0);
        assert_eq!(result, 0.8);
    }

    #[test]
    fn test_interpolate_text_scalar_interpolation() {
        let keys = vec![
            TextScalarKeyframe {
                time: 0.0,
                value: 0.0,
            },
            TextScalarKeyframe {
                time: 1.0,
                value: 1.0,
            },
        ];
        let result = interpolate_text_scalar(0.5, &keys, 0.5);
        assert!((result - 0.5).abs() < 1e-6);

        let result_before = interpolate_text_scalar(0.5, &keys, -1.0);
        assert_eq!(result_before, 0.0);

        let result_after = interpolate_text_scalar(0.5, &keys, 2.0);
        assert_eq!(result_after, 1.0);
    }

    #[test]
    fn test_interpolate_text_vector_empty() {
        let base = XY::new(0.5, 0.5);
        let result = interpolate_text_vector(base, &[], 1.0);
        assert_eq!(result.x, 0.5);
        assert_eq!(result.y, 0.5);
    }

    #[test]
    fn test_interpolate_text_vector_interpolation() {
        let keys = vec![
            TextVectorKeyframe {
                time: 0.0,
                x: 0.0,
                y: 0.0,
            },
            TextVectorKeyframe {
                time: 1.0,
                x: 1.0,
                y: 1.0,
            },
        ];
        let base = XY::new(0.5, 0.5);
        let result = interpolate_text_vector(base, &keys, 0.5);
        assert!((result.x - 0.5).abs() < 1e-6);
        assert!((result.y - 0.5).abs() < 1e-6);

        let result_before = interpolate_text_vector(base, &keys, -1.0);
        assert_eq!(result_before.x, 0.0);
        assert_eq!(result_before.y, 0.0);

        let result_after = interpolate_text_vector(base, &keys, 2.0);
        assert_eq!(result_after.x, 1.0);
        assert_eq!(result_after.y, 1.0);
    }

    #[test]
    fn test_interpolate_text_vector_single() {
        let keys = vec![TextVectorKeyframe {
            time: 0.5,
            x: 0.3,
            y: 0.7,
        }];
        let base = XY::new(0.0, 0.0);
        let result = interpolate_text_vector(base, &keys, 1.0);
        assert_eq!(result.x, 0.3);
        assert_eq!(result.y, 0.7);
    }

    #[test]
    fn test_interpolate_text_scalar_out_of_order() {
        let keys = vec![
            TextScalarKeyframe {
                time: 1.0,
                value: 1.0,
            },
            TextScalarKeyframe {
                time: 0.0,
                value: 0.0,
            },
        ];
        let result = interpolate_text_scalar(0.5, &keys, 0.5);
        assert!((result - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_interpolate_text_vector_out_of_order() {
        let keys = vec![
            TextVectorKeyframe {
                time: 1.0,
                x: 1.0,
                y: 1.0,
            },
            TextVectorKeyframe {
                time: 0.0,
                x: 0.0,
                y: 0.0,
            },
        ];
        let base = XY::new(0.5, 0.5);
        let result = interpolate_text_vector(base, &keys, 0.5);
        assert!((result.x - 0.5).abs() < 1e-6);
        assert!((result.y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_interpolate_text_scalar_multiple_segments() {
        let keys = vec![
            TextScalarKeyframe {
                time: 0.0,
                value: 0.0,
            },
            TextScalarKeyframe {
                time: 1.0,
                value: 1.0,
            },
            TextScalarKeyframe {
                time: 2.0,
                value: 0.5,
            },
        ];

        let at_zero = interpolate_text_scalar(0.0, &keys, 0.0);
        assert_eq!(at_zero, 0.0);

        let at_half = interpolate_text_scalar(0.0, &keys, 0.5);
        assert!((at_half - 0.5).abs() < 1e-6);

        let at_one = interpolate_text_scalar(0.0, &keys, 1.0);
        assert_eq!(at_one, 1.0);

        let at_1_5 = interpolate_text_scalar(0.0, &keys, 1.5);
        assert!((at_1_5 - 0.75).abs() < 1e-6);

        let at_two = interpolate_text_scalar(0.0, &keys, 2.0);
        assert_eq!(at_two, 0.5);
    }

    #[test]
    fn test_interpolate_text_scalar_clamping() {
        let keys = vec![
            TextScalarKeyframe {
                time: 0.0,
                value: 0.0,
            },
            TextScalarKeyframe {
                time: 1.0,
                value: 2.0,
            },
        ];

        let at_half = interpolate_text_scalar(0.0, &keys, 0.5);
        assert_eq!(at_half, 1.0);

        let at_one = interpolate_text_scalar(0.0, &keys, 1.0);
        assert_eq!(at_one, 1.0);
    }

    #[test]
    fn test_interpolate_text_scalar_very_close_keyframes() {
        let keys = vec![
            TextScalarKeyframe {
                time: 1.0,
                value: 0.0,
            },
            TextScalarKeyframe {
                time: 1.0000001,
                value: 1.0,
            },
        ];
        let result = interpolate_text_scalar(0.5, &keys, 1.00000005);
        assert!(result >= 0.0 && result <= 1.0);
    }

    #[test]
    fn test_parse_color_valid_hex() {
        let color = parse_color("#ff0000");
        assert!((color[0] - 1.0).abs() < 1e-6);
        assert!((color[1] - 0.0).abs() < 1e-6);
        assert!((color[2] - 0.0).abs() < 1e-6);
        assert_eq!(color[3], 1.0);
    }

    #[test]
    fn test_parse_color_no_hash() {
        let color = parse_color("00ff00");
        assert!((color[0] - 0.0).abs() < 1e-6);
        assert!((color[1] - 1.0).abs() < 1e-6);
        assert!((color[2] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_parse_color_invalid_returns_white() {
        let color = parse_color("invalid");
        assert_eq!(color, [1.0, 1.0, 1.0, 1.0]);

        let color_short = parse_color("#fff");
        assert_eq!(color_short, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_prepare_texts_empty_segments() {
        let result = prepare_texts(XY::new(1920, 1080), 0.0, &[], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_prepare_texts_disabled_segment() {
        let segment = TextSegment {
            start: 0.0,
            end: 10.0,
            enabled: false,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 0.0,
            keyframes: TextKeyframes::default(),
        };
        let result = prepare_texts(XY::new(1920, 1080), 5.0, &[segment], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_prepare_texts_outside_time_range() {
        let segment = TextSegment {
            start: 5.0,
            end: 10.0,
            enabled: true,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 0.0,
            keyframes: TextKeyframes::default(),
        };

        let before = prepare_texts(XY::new(1920, 1080), 2.0, &[segment.clone()], &[]);
        assert!(before.is_empty());

        let after = prepare_texts(XY::new(1920, 1080), 15.0, &[segment], &[]);
        assert!(after.is_empty());
    }

    #[test]
    fn test_prepare_texts_hidden_index() {
        let segment = TextSegment {
            start: 0.0,
            end: 10.0,
            enabled: true,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 0.0,
            keyframes: TextKeyframes::default(),
        };
        let result = prepare_texts(XY::new(1920, 1080), 5.0, &[segment], &[0]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_prepare_texts_basic_rendering() {
        let segment = TextSegment {
            start: 0.0,
            end: 10.0,
            enabled: true,
            content: "Hello World".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "Arial".to_string(),
            font_size: 48.0,
            font_weight: 400.0,
            italic: true,
            color: "#ff0000".to_string(),
            fade_duration: 0.0,
            keyframes: TextKeyframes::default(),
        };
        let result = prepare_texts(XY::new(1920, 1080), 5.0, &[segment], &[]);

        assert_eq!(result.len(), 1);
        let text = &result[0];
        assert_eq!(text.content, "Hello World");
        assert_eq!(text.font_family, "Arial");
        assert_eq!(text.font_weight, 400.0);
        assert!(text.italic);
        assert!((text.color[0] - 1.0).abs() < 1e-6);
        assert!((text.opacity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_prepare_texts_fade_in_out() {
        let segment = TextSegment {
            start: 0.0,
            end: 10.0,
            enabled: true,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 1.0,
            keyframes: TextKeyframes::default(),
        };

        let at_start = prepare_texts(XY::new(1920, 1080), 0.0, &[segment.clone()], &[]);
        assert_eq!(at_start.len(), 1);
        assert!((at_start[0].opacity - 0.0).abs() < 1e-6);

        let halfway_fade_in = prepare_texts(XY::new(1920, 1080), 0.5, &[segment.clone()], &[]);
        assert_eq!(halfway_fade_in.len(), 1);
        assert!((halfway_fade_in[0].opacity - 0.5).abs() < 1e-6);

        let middle = prepare_texts(XY::new(1920, 1080), 5.0, &[segment.clone()], &[]);
        assert_eq!(middle.len(), 1);
        assert!((middle[0].opacity - 1.0).abs() < 1e-6);

        let halfway_fade_out = prepare_texts(XY::new(1920, 1080), 9.5, &[segment.clone()], &[]);
        assert_eq!(halfway_fade_out.len(), 1);
        assert!((halfway_fade_out[0].opacity - 0.5).abs() < 1e-6);

        let at_end = prepare_texts(XY::new(1920, 1080), 10.0, &[segment], &[]);
        assert_eq!(at_end.len(), 1);
        assert!((at_end[0].opacity - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_prepare_texts_keyframe_opacity() {
        let segment = TextSegment {
            start: 0.0,
            end: 10.0,
            enabled: true,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 0.0,
            keyframes: TextKeyframes {
                position: vec![],
                opacity: vec![
                    TextScalarKeyframe {
                        time: 0.0,
                        value: 0.0,
                    },
                    TextScalarKeyframe {
                        time: 2.0,
                        value: 1.0,
                    },
                    TextScalarKeyframe {
                        time: 8.0,
                        value: 1.0,
                    },
                    TextScalarKeyframe {
                        time: 10.0,
                        value: 0.0,
                    },
                ],
            },
        };

        let at_start = prepare_texts(XY::new(1920, 1080), 0.0, &[segment.clone()], &[]);
        assert_eq!(at_start.len(), 1);
        assert!((at_start[0].opacity - 0.0).abs() < 1e-6);

        let at_one = prepare_texts(XY::new(1920, 1080), 1.0, &[segment.clone()], &[]);
        assert_eq!(at_one.len(), 1);
        assert!((at_one[0].opacity - 0.5).abs() < 1e-6);

        let at_five = prepare_texts(XY::new(1920, 1080), 5.0, &[segment.clone()], &[]);
        assert_eq!(at_five.len(), 1);
        assert!((at_five[0].opacity - 1.0).abs() < 1e-6);

        let at_nine = prepare_texts(XY::new(1920, 1080), 9.0, &[segment], &[]);
        assert_eq!(at_nine.len(), 1);
        assert!((at_nine[0].opacity - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_prepare_texts_keyframe_position() {
        let segment = TextSegment {
            start: 0.0,
            end: 10.0,
            enabled: true,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.2, 0.1),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 0.0,
            keyframes: TextKeyframes {
                position: vec![
                    TextVectorKeyframe {
                        time: 0.0,
                        x: 0.0,
                        y: 0.5,
                    },
                    TextVectorKeyframe {
                        time: 10.0,
                        x: 1.0,
                        y: 0.5,
                    },
                ],
                opacity: vec![],
            },
        };

        let at_start = prepare_texts(XY::new(1920, 1080), 0.0, &[segment.clone()], &[]);
        assert_eq!(at_start.len(), 1);
        assert!(at_start[0].bounds[0] < 10.0);

        let at_middle = prepare_texts(XY::new(1920, 1080), 5.0, &[segment.clone()], &[]);
        assert_eq!(at_middle.len(), 1);
        let mid_left = at_middle[0].bounds[0];
        assert!(mid_left > 700.0 && mid_left < 900.0);

        let at_end = prepare_texts(XY::new(1920, 1080), 10.0, &[segment], &[]);
        assert_eq!(at_end.len(), 1);
        assert!(at_end[0].bounds[0] > 1500.0);
    }

    #[test]
    fn test_prepare_texts_keyframe_opacity_with_fade() {
        let segment = TextSegment {
            start: 0.0,
            end: 10.0,
            enabled: true,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 1.0,
            keyframes: TextKeyframes {
                position: vec![],
                opacity: vec![TextScalarKeyframe {
                    time: 0.0,
                    value: 0.5,
                }],
            },
        };

        let at_half_fade = prepare_texts(XY::new(1920, 1080), 0.5, &[segment], &[]);
        assert_eq!(at_half_fade.len(), 1);
        assert!((at_half_fade[0].opacity - 0.25).abs() < 1e-6);
    }

    #[test]
    fn test_prepare_texts_zero_output_size() {
        let segment = TextSegment {
            start: 0.0,
            end: 10.0,
            enabled: true,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 0.0,
            keyframes: TextKeyframes::default(),
        };
        let result = prepare_texts(XY::new(0, 0), 5.0, &[segment], &[]);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_prepare_texts_multiple_segments() {
        let segments = vec![
            TextSegment {
                start: 0.0,
                end: 5.0,
                enabled: true,
                content: "First".to_string(),
                center: XY::new(0.5, 0.3),
                size: XY::new(0.35, 0.2),
                font_family: "sans-serif".to_string(),
                font_size: 48.0,
                font_weight: 700.0,
                italic: false,
                color: "#ffffff".to_string(),
                fade_duration: 0.0,
                keyframes: TextKeyframes::default(),
            },
            TextSegment {
                start: 3.0,
                end: 8.0,
                enabled: true,
                content: "Second".to_string(),
                center: XY::new(0.5, 0.5),
                size: XY::new(0.35, 0.2),
                font_family: "sans-serif".to_string(),
                font_size: 48.0,
                font_weight: 700.0,
                italic: false,
                color: "#ffffff".to_string(),
                fade_duration: 0.0,
                keyframes: TextKeyframes::default(),
            },
            TextSegment {
                start: 6.0,
                end: 10.0,
                enabled: true,
                content: "Third".to_string(),
                center: XY::new(0.5, 0.7),
                size: XY::new(0.35, 0.2),
                font_family: "sans-serif".to_string(),
                font_size: 48.0,
                font_weight: 700.0,
                italic: false,
                color: "#ffffff".to_string(),
                fade_duration: 0.0,
                keyframes: TextKeyframes::default(),
            },
        ];

        let at_one = prepare_texts(XY::new(1920, 1080), 1.0, &segments, &[]);
        assert_eq!(at_one.len(), 1);
        assert_eq!(at_one[0].content, "First");

        let at_four = prepare_texts(XY::new(1920, 1080), 4.0, &segments, &[]);
        assert_eq!(at_four.len(), 2);

        let at_seven = prepare_texts(XY::new(1920, 1080), 7.0, &segments, &[]);
        assert_eq!(at_seven.len(), 2);

        let at_nine = prepare_texts(XY::new(1920, 1080), 9.0, &segments, &[]);
        assert_eq!(at_nine.len(), 1);
        assert_eq!(at_nine[0].content, "Third");
    }

    #[test]
    fn test_prepare_texts_font_size_scaling() {
        let segment = TextSegment {
            start: 0.0,
            end: 10.0,
            enabled: true,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 0.0,
            keyframes: TextKeyframes::default(),
        };

        let at_1080p = prepare_texts(XY::new(1920, 1080), 5.0, &[segment.clone()], &[]);
        let at_720p = prepare_texts(XY::new(1280, 720), 5.0, &[segment.clone()], &[]);
        let at_4k = prepare_texts(XY::new(3840, 2160), 5.0, &[segment], &[]);

        assert!(at_720p[0].font_size < at_1080p[0].font_size);
        assert!(at_1080p[0].font_size < at_4k[0].font_size);
    }

    #[test]
    fn test_text_visible_at_exact_start_time() {
        let segment = TextSegment {
            start: 5.0,
            end: 10.0,
            enabled: true,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 0.0,
            keyframes: TextKeyframes::default(),
        };
        let result = prepare_texts(XY::new(1920, 1080), 5.0, &[segment], &[]);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_text_visible_at_exact_end_time() {
        let segment = TextSegment {
            start: 5.0,
            end: 10.0,
            enabled: true,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 0.0,
            keyframes: TextKeyframes::default(),
        };
        let result = prepare_texts(XY::new(1920, 1080), 10.0, &[segment], &[]);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_identical_keyframe_times() {
        let keys = vec![
            TextScalarKeyframe {
                time: 1.0,
                value: 0.3,
            },
            TextScalarKeyframe {
                time: 1.0,
                value: 0.8,
            },
        ];
        let result = interpolate_text_scalar(0.5, &keys, 1.0);
        assert!(result >= 0.0 && result <= 1.0);
    }

    #[test]
    fn test_overlapping_fade_regions() {
        let segment = TextSegment {
            start: 0.0,
            end: 2.0,
            enabled: true,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 1.5,
            keyframes: TextKeyframes::default(),
        };
        let result = prepare_texts(XY::new(1920, 1080), 1.0, &[segment], &[]);
        assert_eq!(result.len(), 1);
        assert!(result[0].opacity >= 0.0 && result[0].opacity <= 1.0);
    }

    #[test]
    fn test_position_horizontal_only() {
        let keys = vec![
            TextVectorKeyframe {
                time: 0.0,
                x: 0.0,
                y: 0.5,
            },
            TextVectorKeyframe {
                time: 10.0,
                x: 1.0,
                y: 0.5,
            },
        ];
        let base = XY::new(0.5, 0.5);
        let result = interpolate_text_vector(base, &keys, 5.0);
        assert!((result.x - 0.5).abs() < 1e-6);
        assert_eq!(result.y, 0.5);
    }

    #[test]
    fn test_interpolate_scalar_decreasing() {
        let keys = vec![
            TextScalarKeyframe {
                time: 0.0,
                value: 1.0,
            },
            TextScalarKeyframe {
                time: 1.0,
                value: 0.0,
            },
        ];
        let result = interpolate_text_scalar(0.5, &keys, 0.5);
        assert!((result - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_interpolate_scalar_quarter_points() {
        let keys = vec![
            TextScalarKeyframe {
                time: 0.0,
                value: 0.0,
            },
            TextScalarKeyframe {
                time: 1.0,
                value: 1.0,
            },
        ];
        let at_quarter = interpolate_text_scalar(0.0, &keys, 0.25);
        assert!((at_quarter - 0.25).abs() < 1e-6);

        let at_three_quarters = interpolate_text_scalar(0.0, &keys, 0.75);
        assert!((at_three_quarters - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_position_vertical_only() {
        let keys = vec![
            TextVectorKeyframe {
                time: 0.0,
                x: 0.5,
                y: 0.0,
            },
            TextVectorKeyframe {
                time: 10.0,
                x: 0.5,
                y: 1.0,
            },
        ];
        let base = XY::new(0.5, 0.5);
        let result = interpolate_text_vector(base, &keys, 5.0);
        assert_eq!(result.x, 0.5);
        assert!((result.y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_zigzag_position_path() {
        let keys = vec![
            TextVectorKeyframe {
                time: 0.0,
                x: 0.0,
                y: 0.0,
            },
            TextVectorKeyframe {
                time: 1.0,
                x: 1.0,
                y: 1.0,
            },
            TextVectorKeyframe {
                time: 2.0,
                x: 0.0,
                y: 1.0,
            },
        ];
        let base = XY::new(0.5, 0.5);

        let at_half = interpolate_text_vector(base, &keys, 0.5);
        assert!((at_half.x - 0.5).abs() < 1e-6);
        assert!((at_half.y - 0.5).abs() < 1e-6);

        let at_1_5 = interpolate_text_vector(base, &keys, 1.5);
        assert!((at_1_5.x - 0.5).abs() < 1e-6);
        assert_eq!(at_1_5.y, 1.0);
    }

    #[test]
    fn test_non_uniform_keyframe_spacing() {
        let keys = vec![
            TextScalarKeyframe {
                time: 0.0,
                value: 0.0,
            },
            TextScalarKeyframe {
                time: 0.1,
                value: 0.5,
            },
            TextScalarKeyframe {
                time: 0.9,
                value: 0.5,
            },
            TextScalarKeyframe {
                time: 1.0,
                value: 1.0,
            },
        ];

        let at_0_05 = interpolate_text_scalar(0.0, &keys, 0.05);
        assert!((at_0_05 - 0.25).abs() < 1e-6);

        let at_0_5 = interpolate_text_scalar(0.0, &keys, 0.5);
        assert_eq!(at_0_5, 0.5);

        let at_0_95 = interpolate_text_scalar(0.0, &keys, 0.95);
        assert!((at_0_95 - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_parse_color_black() {
        let color = parse_color("#000000");
        assert!((color[0] - 0.0).abs() < 1e-6);
        assert!((color[1] - 0.0).abs() < 1e-6);
        assert!((color[2] - 0.0).abs() < 1e-6);
        assert_eq!(color[3], 1.0);
    }

    #[test]
    fn test_parse_color_white_uppercase() {
        let color = parse_color("#FFFFFF");
        assert!((color[0] - 1.0).abs() < 1e-6);
        assert!((color[1] - 1.0).abs() < 1e-6);
        assert!((color[2] - 1.0).abs() < 1e-6);
        assert_eq!(color[3], 1.0);
    }

    #[test]
    fn test_parse_color_empty_string() {
        let color = parse_color("");
        assert_eq!(color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_multiple_hidden_indices() {
        let segments = vec![
            TextSegment {
                start: 0.0,
                end: 10.0,
                enabled: true,
                content: "First".to_string(),
                center: XY::new(0.5, 0.3),
                size: XY::new(0.35, 0.2),
                font_family: "sans-serif".to_string(),
                font_size: 48.0,
                font_weight: 700.0,
                italic: false,
                color: "#ffffff".to_string(),
                fade_duration: 0.0,
                keyframes: TextKeyframes::default(),
            },
            TextSegment {
                start: 0.0,
                end: 10.0,
                enabled: true,
                content: "Second".to_string(),
                center: XY::new(0.5, 0.5),
                size: XY::new(0.35, 0.2),
                font_family: "sans-serif".to_string(),
                font_size: 48.0,
                font_weight: 700.0,
                italic: false,
                color: "#ffffff".to_string(),
                fade_duration: 0.0,
                keyframes: TextKeyframes::default(),
            },
            TextSegment {
                start: 0.0,
                end: 10.0,
                enabled: true,
                content: "Third".to_string(),
                center: XY::new(0.5, 0.7),
                size: XY::new(0.35, 0.2),
                font_family: "sans-serif".to_string(),
                font_size: 48.0,
                font_weight: 700.0,
                italic: false,
                color: "#ffffff".to_string(),
                fade_duration: 0.0,
                keyframes: TextKeyframes::default(),
            },
        ];

        let result = prepare_texts(XY::new(1920, 1080), 5.0, &segments, &[0, 2]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "Second");
    }

    #[test]
    fn test_keyframe_opacity_during_fade_out() {
        let segment = TextSegment {
            start: 0.0,
            end: 10.0,
            enabled: true,
            content: "Test".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 2.0,
            keyframes: TextKeyframes {
                position: vec![],
                opacity: vec![TextScalarKeyframe {
                    time: 0.0,
                    value: 0.5,
                }],
            },
        };

        let result = prepare_texts(XY::new(1920, 1080), 9.0, &[segment], &[]);
        assert_eq!(result.len(), 1);
        let expected_fade_out = 1.0 / 2.0;
        let expected_opacity = 0.5 * expected_fade_out;
        assert!((result[0].opacity - expected_opacity as f32).abs() < 1e-6);
    }
}
