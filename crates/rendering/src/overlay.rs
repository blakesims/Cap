use cap_project::{
    OverlayItem, OverlayItemStyle, OverlaySegment, OverlayType, SceneMode, SceneSegment,
    TextKeyframes, TextScalarKeyframe, TextSegment, TextVectorKeyframe, XY,
};

const ANIMATION_DURATION: f64 = 0.3;
const SLIDE_OFFSET_X: f64 = 0.15;
const TITLE_FONT_SIZE: f32 = 64.0;
const BULLET_FONT_SIZE: f32 = 40.0;
const FIRST_ITEM_Y: f64 = 0.25;
const ITEM_Y_SPACING: f64 = 0.12;
const LEFT_ALIGN_X: f64 = 0.25;
const CENTER_X: f64 = 0.5;
const CENTER_Y: f64 = 0.5;
const TEXT_WIDTH: f64 = 0.35;
const TEXT_HEIGHT: f64 = 0.15;

pub struct OverlayWarning {
    pub overlay_index: usize,
    pub item_index: usize,
    pub delay: f64,
    pub segment_duration: f64,
}

pub fn generate_scene_segments(overlays: &[OverlaySegment]) -> Vec<SceneSegment> {
    overlays
        .iter()
        .map(|overlay| {
            let mode = match overlay.overlay_type {
                OverlayType::Split => SceneMode::SplitScreenRight,
                OverlayType::FullScreen => SceneMode::Default,
            };

            SceneSegment {
                start: overlay.start,
                end: overlay.end,
                mode,
            }
        })
        .collect()
}

pub fn generate_text_segments(overlays: &[OverlaySegment]) -> (Vec<TextSegment>, Vec<OverlayWarning>) {
    let mut text_segments = Vec::new();
    let mut warnings = Vec::new();

    for (overlay_index, overlay) in overlays.iter().enumerate() {
        let segment_duration = overlay.end - overlay.start;

        for (item_index, item) in overlay.items.iter().enumerate() {
            if item.delay >= segment_duration {
                warnings.push(OverlayWarning {
                    overlay_index,
                    item_index,
                    delay: item.delay,
                    segment_duration,
                });
            }

            let text_segment = create_text_segment(overlay, item, item_index);
            text_segments.push(text_segment);
        }
    }

    (text_segments, warnings)
}

fn create_text_segment(overlay: &OverlaySegment, item: &OverlayItem, item_index: usize) -> TextSegment {
    let (center, font_size) = get_position_and_size(&item.style, item_index);
    let absolute_start = overlay.start + item.delay;
    let content = format_content(&item.content, &item.style, item_index);
    let keyframes = create_animation_keyframes(&item.style, absolute_start, overlay.start);

    TextSegment {
        start: absolute_start,
        end: overlay.end,
        enabled: true,
        content,
        center,
        size: XY::new(TEXT_WIDTH, TEXT_HEIGHT),
        font_family: "sans-serif".to_string(),
        font_size,
        font_weight: 700.0,
        italic: false,
        color: "#ffffff".to_string(),
        fade_duration: 0.0,
        keyframes,
    }
}

fn get_position_and_size(style: &OverlayItemStyle, item_index: usize) -> (XY<f64>, f32) {
    match style {
        OverlayItemStyle::Title => (XY::new(CENTER_X, CENTER_Y), TITLE_FONT_SIZE),
        OverlayItemStyle::Bullet | OverlayItemStyle::Numbered => {
            let y = FIRST_ITEM_Y + (item_index as f64 * ITEM_Y_SPACING);
            (XY::new(LEFT_ALIGN_X, y), BULLET_FONT_SIZE)
        }
    }
}

fn format_content(content: &str, style: &OverlayItemStyle, item_index: usize) -> String {
    match style {
        OverlayItemStyle::Title => content.to_string(),
        OverlayItemStyle::Bullet => format!("• {}", content),
        OverlayItemStyle::Numbered => format!("{}. {}", item_index + 1, content),
    }
}

fn create_animation_keyframes(
    _style: &OverlayItemStyle,
    absolute_start: f64,
    overlay_start: f64,
) -> TextKeyframes {
    let relative_time = absolute_start - overlay_start;
    let animation_end = relative_time + ANIMATION_DURATION;

    TextKeyframes {
        position: vec![
            TextVectorKeyframe {
                time: 0.0,
                x: -SLIDE_OFFSET_X,
                y: 0.5,
            },
            TextVectorKeyframe {
                time: relative_time,
                x: -SLIDE_OFFSET_X,
                y: 0.5,
            },
            TextVectorKeyframe {
                time: animation_end,
                x: 0.0,
                y: 0.5,
            },
        ],
        opacity: vec![
            TextScalarKeyframe {
                time: 0.0,
                value: 0.0,
            },
            TextScalarKeyframe {
                time: relative_time,
                value: 0.0,
            },
            TextScalarKeyframe {
                time: animation_end,
                value: 1.0,
            },
        ],
    }
}

pub fn validate_overlay_items(overlays: &[OverlaySegment]) -> Vec<OverlayWarning> {
    let mut warnings = Vec::new();

    for (overlay_index, overlay) in overlays.iter().enumerate() {
        let segment_duration = overlay.end - overlay.start;

        for (item_index, item) in overlay.items.iter().enumerate() {
            if item.delay >= segment_duration {
                warnings.push(OverlayWarning {
                    overlay_index,
                    item_index,
                    delay: item.delay,
                    segment_duration,
                });
            }
        }
    }

    warnings
}

pub fn merge_with_existing(
    existing_scene_segments: &[SceneSegment],
    existing_text_segments: &[TextSegment],
    overlay_scene_segments: Vec<SceneSegment>,
    overlay_text_segments: Vec<TextSegment>,
) -> (Vec<SceneSegment>, Vec<TextSegment>) {
    let mut scene_segments = existing_scene_segments.to_vec();
    scene_segments.extend(overlay_scene_segments);
    scene_segments.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));

    let mut text_segments = existing_text_segments.to_vec();
    text_segments.extend(overlay_text_segments);
    text_segments.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));

    (scene_segments, text_segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_overlay(overlay_type: OverlayType, items: Vec<OverlayItem>) -> OverlaySegment {
        OverlaySegment {
            start: 10.0,
            end: 45.0,
            overlay_type,
            items,
        }
    }

    fn make_item(delay: f64, content: &str, style: OverlayItemStyle) -> OverlayItem {
        OverlayItem {
            delay,
            content: content.to_string(),
            style,
        }
    }

    #[test]
    fn test_generate_scene_segments_split() {
        let overlays = vec![make_overlay(OverlayType::Split, vec![])];
        let scenes = generate_scene_segments(&overlays);

        assert_eq!(scenes.len(), 1);
        assert_eq!(scenes[0].start, 10.0);
        assert_eq!(scenes[0].end, 45.0);
        assert!(matches!(scenes[0].mode, SceneMode::SplitScreenRight));
    }

    #[test]
    fn test_generate_scene_segments_fullscreen() {
        let overlays = vec![make_overlay(OverlayType::FullScreen, vec![])];
        let scenes = generate_scene_segments(&overlays);

        assert_eq!(scenes.len(), 1);
        assert!(matches!(scenes[0].mode, SceneMode::Default));
    }

    #[test]
    fn test_generate_text_segments_title() {
        let overlays = vec![make_overlay(
            OverlayType::Split,
            vec![make_item(0.5, "Overview", OverlayItemStyle::Title)],
        )];
        let (texts, warnings) = generate_text_segments(&overlays);

        assert_eq!(texts.len(), 1);
        assert!(warnings.is_empty());

        let text = &texts[0];
        assert_eq!(text.start, 10.5);
        assert_eq!(text.end, 45.0);
        assert_eq!(text.content, "Overview");
        assert!((text.center.x - CENTER_X).abs() < 1e-6);
        assert!((text.center.y - CENTER_Y).abs() < 1e-6);
        assert_eq!(text.font_size, TITLE_FONT_SIZE);
    }

    #[test]
    fn test_generate_text_segments_bullet() {
        let overlays = vec![make_overlay(
            OverlayType::Split,
            vec![
                make_item(1.0, "First point", OverlayItemStyle::Bullet),
                make_item(2.0, "Second point", OverlayItemStyle::Bullet),
            ],
        )];
        let (texts, warnings) = generate_text_segments(&overlays);

        assert_eq!(texts.len(), 2);
        assert!(warnings.is_empty());

        assert_eq!(texts[0].content, "• First point");
        assert!((texts[0].center.x - LEFT_ALIGN_X).abs() < 1e-6);
        assert!((texts[0].center.y - FIRST_ITEM_Y).abs() < 1e-6);
        assert_eq!(texts[0].font_size, BULLET_FONT_SIZE);

        assert_eq!(texts[1].content, "• Second point");
        assert!((texts[1].center.y - (FIRST_ITEM_Y + ITEM_Y_SPACING)).abs() < 1e-6);
    }

    #[test]
    fn test_generate_text_segments_numbered() {
        let overlays = vec![make_overlay(
            OverlayType::Split,
            vec![
                make_item(1.0, "Step one", OverlayItemStyle::Numbered),
                make_item(2.0, "Step two", OverlayItemStyle::Numbered),
            ],
        )];
        let (texts, _) = generate_text_segments(&overlays);

        assert_eq!(texts[0].content, "1. Step one");
        assert_eq!(texts[1].content, "2. Step two");
    }

    #[test]
    fn test_generate_text_segments_animation_keyframes() {
        let overlays = vec![make_overlay(
            OverlayType::Split,
            vec![make_item(2.0, "Test", OverlayItemStyle::Title)],
        )];
        let (texts, _) = generate_text_segments(&overlays);
        let keyframes = &texts[0].keyframes;

        assert_eq!(keyframes.position.len(), 3);
        assert_eq!(keyframes.opacity.len(), 3);

        assert_eq!(keyframes.opacity[0].time, 0.0);
        assert_eq!(keyframes.opacity[0].value, 0.0);

        assert_eq!(keyframes.opacity[1].time, 2.0);
        assert_eq!(keyframes.opacity[1].value, 0.0);

        assert!((keyframes.opacity[2].time - 2.3).abs() < 1e-6);
        assert_eq!(keyframes.opacity[2].value, 1.0);

        assert!((keyframes.position[1].x - (-SLIDE_OFFSET_X)).abs() < 1e-6);
        assert_eq!(keyframes.position[2].x, 0.0);
    }

    #[test]
    fn test_warning_for_delay_exceeds_duration() {
        let overlays = vec![make_overlay(
            OverlayType::Split,
            vec![make_item(50.0, "Late item", OverlayItemStyle::Title)],
        )];
        let (texts, warnings) = generate_text_segments(&overlays);

        assert_eq!(texts.len(), 1);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].overlay_index, 0);
        assert_eq!(warnings[0].item_index, 0);
        assert_eq!(warnings[0].delay, 50.0);
        assert_eq!(warnings[0].segment_duration, 35.0);
    }

    #[test]
    fn test_multiple_overlays() {
        let overlays = vec![
            OverlaySegment {
                start: 5.0,
                end: 15.0,
                overlay_type: OverlayType::Split,
                items: vec![make_item(1.0, "First overlay", OverlayItemStyle::Title)],
            },
            OverlaySegment {
                start: 20.0,
                end: 30.0,
                overlay_type: OverlayType::FullScreen,
                items: vec![make_item(0.5, "Second overlay", OverlayItemStyle::Bullet)],
            },
        ];

        let scenes = generate_scene_segments(&overlays);
        let (texts, warnings) = generate_text_segments(&overlays);

        assert_eq!(scenes.len(), 2);
        assert_eq!(texts.len(), 2);
        assert!(warnings.is_empty());

        assert!(matches!(scenes[0].mode, SceneMode::SplitScreenRight));
        assert!(matches!(scenes[1].mode, SceneMode::Default));
    }

    #[test]
    fn test_merge_with_existing() {
        let existing_scenes = vec![SceneSegment {
            start: 0.0,
            end: 5.0,
            mode: SceneMode::CameraOnly,
        }];
        let existing_texts = vec![TextSegment {
            start: 0.0,
            end: 5.0,
            enabled: true,
            content: "Existing".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 48.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 0.15,
            keyframes: TextKeyframes::default(),
        }];

        let overlay_scenes = vec![SceneSegment {
            start: 10.0,
            end: 20.0,
            mode: SceneMode::SplitScreenRight,
        }];
        let overlay_texts = vec![TextSegment {
            start: 10.0,
            end: 20.0,
            enabled: true,
            content: "Overlay".to_string(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.35, 0.2),
            font_family: "sans-serif".to_string(),
            font_size: 64.0,
            font_weight: 700.0,
            italic: false,
            color: "#ffffff".to_string(),
            fade_duration: 0.0,
            keyframes: TextKeyframes::default(),
        }];

        let (merged_scenes, merged_texts) =
            merge_with_existing(&existing_scenes, &existing_texts, overlay_scenes, overlay_texts);

        assert_eq!(merged_scenes.len(), 2);
        assert_eq!(merged_texts.len(), 2);

        assert_eq!(merged_scenes[0].start, 0.0);
        assert_eq!(merged_scenes[1].start, 10.0);

        assert_eq!(merged_texts[0].content, "Existing");
        assert_eq!(merged_texts[1].content, "Overlay");
    }

    #[test]
    fn test_empty_overlays() {
        let scenes = generate_scene_segments(&[]);
        let (texts, warnings) = generate_text_segments(&[]);

        assert!(scenes.is_empty());
        assert!(texts.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_overlay_with_no_items() {
        let overlays = vec![make_overlay(OverlayType::Split, vec![])];
        let scenes = generate_scene_segments(&overlays);
        let (texts, warnings) = generate_text_segments(&overlays);

        assert_eq!(scenes.len(), 1);
        assert!(texts.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_overlay_items() {
        let overlays = vec![
            OverlaySegment {
                start: 0.0,
                end: 10.0,
                overlay_type: OverlayType::Split,
                items: vec![
                    make_item(5.0, "Valid", OverlayItemStyle::Title),
                    make_item(15.0, "Invalid - exceeds", OverlayItemStyle::Bullet),
                ],
            },
            OverlaySegment {
                start: 20.0,
                end: 25.0,
                overlay_type: OverlayType::FullScreen,
                items: vec![make_item(5.0, "Exactly at boundary", OverlayItemStyle::Title)],
            },
        ];

        let warnings = validate_overlay_items(&overlays);

        assert_eq!(warnings.len(), 2);
        assert_eq!(warnings[0].overlay_index, 0);
        assert_eq!(warnings[0].item_index, 1);
        assert_eq!(warnings[1].overlay_index, 1);
        assert_eq!(warnings[1].item_index, 0);
    }
}
