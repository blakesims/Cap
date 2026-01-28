use cap_project::{
    SceneMode, SceneSegment, TextKeyframes, TextScalarKeyframe, TextSegment, TextVectorKeyframe, XY,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{info, instrument, warn};

use crate::editor_window::WindowEditorInstance;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportXY {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportVectorKeyframe {
    pub time: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportScalarKeyframe {
    pub time: f64,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportTextKeyframes {
    #[serde(default)]
    pub position: Vec<ImportVectorKeyframe>,
    #[serde(default)]
    pub opacity: Vec<ImportScalarKeyframe>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportTextSegment {
    pub start: f64,
    pub end: f64,
    pub content: String,
    #[serde(default)]
    pub center: Option<ImportXY>,
    #[serde(default)]
    pub font_size: Option<f32>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_weight: Option<String>,
    #[serde(default)]
    pub font_color: Option<String>,
    #[serde(default)]
    pub fade_duration: Option<f64>,
    #[serde(default)]
    pub keyframes: Option<ImportTextKeyframes>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ImportSceneMode {
    Camera,
    Screen,
    SplitScreenLeft,
    SplitScreenRight,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportSceneChange {
    pub time: f64,
    pub mode: ImportSceneMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TimelineImport {
    pub version: String,
    #[serde(default)]
    pub text_segments: Vec<ImportTextSegment>,
    #[serde(default)]
    pub scene_changes: Vec<ImportSceneChange>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum ImportMergeMode {
    #[default]
    Replace,
    Append,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub text_segments_imported: u32,
    pub scene_segments_created: u32,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum TimelineImportError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Unsupported version: expected '1.0.0', got '{0}'")]
    UnsupportedVersion(String),

    #[error("Invalid time range in segment {index}: start ({start}) must be less than end ({end})")]
    InvalidTimeRange { index: usize, start: f64, end: f64 },

    #[error("Empty content in segment {index}")]
    EmptyContent { index: usize },

    #[error("Negative keyframe time in segment {index}: {time}")]
    NegativeKeyframeTime { index: usize, time: f64 },

    #[error("Scene changes must be sorted by time")]
    UnsortedSceneChanges,

    #[error("Duplicate scene change time: {time}")]
    DuplicateSceneTime { time: f64 },

    #[error("Failed to write configuration: {0}")]
    WriteError(String),
}

impl From<TimelineImportError> for String {
    fn from(err: TimelineImportError) -> Self {
        err.to_string()
    }
}

pub fn validate_import(import: &TimelineImport) -> Result<Vec<String>, TimelineImportError> {
    let mut warnings = Vec::new();

    if import.version != "1.0.0" {
        return Err(TimelineImportError::UnsupportedVersion(
            import.version.clone(),
        ));
    }

    for (index, segment) in import.text_segments.iter().enumerate() {
        if segment.end <= segment.start {
            return Err(TimelineImportError::InvalidTimeRange {
                index,
                start: segment.start,
                end: segment.end,
            });
        }

        if segment.content.trim().is_empty() {
            return Err(TimelineImportError::EmptyContent { index });
        }

        if let Some(ref keyframes) = segment.keyframes {
            for kf in &keyframes.position {
                if kf.time < 0.0 {
                    return Err(TimelineImportError::NegativeKeyframeTime {
                        index,
                        time: kf.time,
                    });
                }
            }
            for kf in &keyframes.opacity {
                if kf.time < 0.0 {
                    return Err(TimelineImportError::NegativeKeyframeTime {
                        index,
                        time: kf.time,
                    });
                }
            }
        }

        if let Some(ref center) = segment.center {
            if center.x < 0.0 || center.x > 1.0 {
                warnings.push(format!(
                    "Segment {index}: center.x ({}) will be clamped to [0.0, 1.0]",
                    center.x
                ));
            }
            if center.y < 0.0 || center.y > 1.0 {
                warnings.push(format!(
                    "Segment {index}: center.y ({}) will be clamped to [0.0, 1.0]",
                    center.y
                ));
            }
        }

        if let Some(ref keyframes) = segment.keyframes {
            for (kf_idx, kf) in keyframes.position.iter().enumerate() {
                if kf.x < 0.0 || kf.x > 1.0 {
                    warnings.push(format!(
                        "Segment {index}: position keyframe {kf_idx} x ({}) will be clamped to [0.0, 1.0]",
                        kf.x
                    ));
                }
                if kf.y < 0.0 || kf.y > 1.0 {
                    warnings.push(format!(
                        "Segment {index}: position keyframe {kf_idx} y ({}) will be clamped to [0.0, 1.0]",
                        kf.y
                    ));
                }
            }
            for (kf_idx, kf) in keyframes.opacity.iter().enumerate() {
                if kf.value < 0.0 || kf.value > 1.0 {
                    warnings.push(format!(
                        "Segment {index}: opacity keyframe {kf_idx} value ({}) will be clamped to [0.0, 1.0]",
                        kf.value
                    ));
                }
            }
        }
    }

    let mut prev_time: Option<f64> = None;
    for change in &import.scene_changes {
        if let Some(pt) = prev_time {
            if change.time < pt {
                return Err(TimelineImportError::UnsortedSceneChanges);
            }
            if (change.time - pt).abs() < f64::EPSILON {
                return Err(TimelineImportError::DuplicateSceneTime { time: change.time });
            }
        }
        prev_time = Some(change.time);
    }

    Ok(warnings)
}

fn clamp_position(val: f64) -> f64 {
    val.clamp(0.0, 1.0)
}

fn clamp_opacity(val: f64) -> f64 {
    val.clamp(0.0, 1.0)
}

pub fn transform_text_segment(seg: &ImportTextSegment) -> TextSegment {
    let center = seg
        .center
        .as_ref()
        .map(|c| XY::new(clamp_position(c.x), clamp_position(c.y)))
        .unwrap_or_else(|| XY::new(0.5, 0.5));

    let font_size = seg.font_size.unwrap_or(50.0);
    let font_family = seg
        .font_family
        .clone()
        .unwrap_or_else(|| "Inter".to_string());
    let font_weight = seg
        .font_weight
        .as_ref()
        .and_then(|w| w.parse::<f32>().ok())
        .unwrap_or(700.0);
    let color = seg
        .font_color
        .clone()
        .unwrap_or_else(|| "#FFFFFF".to_string());
    let fade_duration = seg.fade_duration.unwrap_or(0.0);

    let keyframes = seg
        .keyframes
        .as_ref()
        .map(|kf| TextKeyframes {
            position: kf
                .position
                .iter()
                .map(|p| TextVectorKeyframe {
                    time: p.time,
                    x: clamp_position(p.x),
                    y: clamp_position(p.y),
                })
                .collect(),
            opacity: kf
                .opacity
                .iter()
                .map(|o| TextScalarKeyframe {
                    time: o.time,
                    value: clamp_opacity(o.value),
                })
                .collect(),
        })
        .unwrap_or_default();

    TextSegment {
        start: seg.start,
        end: seg.end,
        enabled: true,
        content: seg.content.clone(),
        center,
        size: XY::new(0.35, 0.2),
        font_family,
        font_size,
        font_weight,
        italic: false,
        color,
        fade_duration,
        keyframes,
    }
}

pub fn transform_scene_mode(mode: &ImportSceneMode) -> SceneMode {
    match mode {
        ImportSceneMode::Camera => SceneMode::CameraOnly,
        ImportSceneMode::Screen => SceneMode::Default,
        ImportSceneMode::SplitScreenLeft => SceneMode::SplitScreenLeft,
        ImportSceneMode::SplitScreenRight => SceneMode::SplitScreenRight,
    }
}

pub fn transform_scene_changes(
    changes: &[ImportSceneChange],
    video_duration: f64,
) -> Vec<SceneSegment> {
    if changes.is_empty() {
        return Vec::new();
    }

    let mut segments = Vec::with_capacity(changes.len());

    for (i, change) in changes.iter().enumerate() {
        let start = change.time;
        let end = if i + 1 < changes.len() {
            changes[i + 1].time
        } else {
            video_duration
        };

        if end > start {
            segments.push(SceneSegment {
                start,
                end,
                mode: transform_scene_mode(&change.mode),
            });
        }
    }

    segments
}

#[tauri::command]
#[specta::specta]
#[instrument(skip(editor_instance))]
pub async fn import_timeline_json(
    editor_instance: WindowEditorInstance,
    path: PathBuf,
    mode: ImportMergeMode,
) -> Result<ImportResult, String> {
    info!(?path, ?mode, "Starting timeline import");

    let content = std::fs::read_to_string(&path).map_err(TimelineImportError::from)?;

    let import: TimelineImport =
        serde_json::from_str(&content).map_err(TimelineImportError::from)?;

    let warnings = validate_import(&import)?;

    for warning in &warnings {
        warn!(%warning, "Import validation warning");
    }

    let mut config = editor_instance.project_config.1.borrow().clone();

    let video_duration = config
        .timeline
        .as_ref()
        .map(|t| t.duration())
        .unwrap_or(0.0);

    let text_segments: Vec<TextSegment> = import
        .text_segments
        .iter()
        .map(transform_text_segment)
        .collect();

    let scene_segments = transform_scene_changes(&import.scene_changes, video_duration);

    let text_segments_imported = text_segments.len() as u32;
    let scene_segments_created = scene_segments.len() as u32;

    let timeline = config
        .timeline
        .get_or_insert_with(|| cap_project::TimelineConfiguration {
            segments: vec![],
            zoom_segments: vec![],
            scene_segments: vec![],
            mask_segments: vec![],
            text_segments: vec![],
        });

    match mode {
        ImportMergeMode::Replace => {
            timeline.text_segments = text_segments;
            if !scene_segments.is_empty() {
                timeline.scene_segments = scene_segments;
            }
        }
        ImportMergeMode::Append => {
            timeline.text_segments.extend(text_segments);
            if !scene_segments.is_empty() {
                timeline.scene_segments.extend(scene_segments);
            }
        }
    }

    config
        .write(&editor_instance.project_path)
        .map_err(|e| TimelineImportError::WriteError(e.to_string()))?;

    editor_instance.project_config.0.send(config).ok();

    info!(
        text_segments_imported,
        scene_segments_created,
        warnings_count = warnings.len(),
        "Timeline import complete"
    );

    Ok(ImportResult {
        text_segments_imported,
        scene_segments_created,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_mismatch_validation() {
        let import = TimelineImport {
            version: "2.0.0".to_string(),
            text_segments: vec![],
            scene_changes: vec![],
        };

        let result = validate_import(&import);
        assert!(matches!(
            result,
            Err(TimelineImportError::UnsupportedVersion(v)) if v == "2.0.0"
        ));
    }

    #[test]
    fn test_invalid_time_range_validation() {
        let import = TimelineImport {
            version: "1.0.0".to_string(),
            text_segments: vec![ImportTextSegment {
                start: 5.0,
                end: 3.0,
                content: "Test".to_string(),
                center: None,
                font_size: None,
                font_family: None,
                font_weight: None,
                font_color: None,
                fade_duration: None,
                keyframes: None,
            }],
            scene_changes: vec![],
        };

        let result = validate_import(&import);
        assert!(matches!(
            result,
            Err(TimelineImportError::InvalidTimeRange {
                index: 0,
                start: 5.0,
                end: 3.0
            })
        ));
    }

    #[test]
    fn test_empty_content_validation() {
        let import = TimelineImport {
            version: "1.0.0".to_string(),
            text_segments: vec![ImportTextSegment {
                start: 0.0,
                end: 5.0,
                content: "   ".to_string(),
                center: None,
                font_size: None,
                font_family: None,
                font_weight: None,
                font_color: None,
                fade_duration: None,
                keyframes: None,
            }],
            scene_changes: vec![],
        };

        let result = validate_import(&import);
        assert!(matches!(
            result,
            Err(TimelineImportError::EmptyContent { index: 0 })
        ));
    }

    #[test]
    fn test_unsorted_scene_changes_validation() {
        let import = TimelineImport {
            version: "1.0.0".to_string(),
            text_segments: vec![],
            scene_changes: vec![
                ImportSceneChange {
                    time: 5.0,
                    mode: ImportSceneMode::Screen,
                },
                ImportSceneChange {
                    time: 2.0,
                    mode: ImportSceneMode::Camera,
                },
            ],
        };

        let result = validate_import(&import);
        assert!(matches!(
            result,
            Err(TimelineImportError::UnsortedSceneChanges)
        ));
    }

    #[test]
    fn test_negative_keyframe_time_validation() {
        let import = TimelineImport {
            version: "1.0.0".to_string(),
            text_segments: vec![ImportTextSegment {
                start: 0.0,
                end: 5.0,
                content: "Test".to_string(),
                center: None,
                font_size: None,
                font_family: None,
                font_weight: None,
                font_color: None,
                fade_duration: None,
                keyframes: Some(ImportTextKeyframes {
                    position: vec![ImportVectorKeyframe {
                        time: -1.0,
                        x: 0.5,
                        y: 0.5,
                    }],
                    opacity: vec![],
                }),
            }],
            scene_changes: vec![],
        };

        let result = validate_import(&import);
        assert!(matches!(
            result,
            Err(TimelineImportError::NegativeKeyframeTime { index: 0, time }) if time == -1.0
        ));
    }

    #[test]
    fn test_text_segment_transformation_with_defaults() {
        let import_seg = ImportTextSegment {
            start: 1.0,
            end: 5.0,
            content: "Hello World".to_string(),
            center: None,
            font_size: None,
            font_family: None,
            font_weight: None,
            font_color: None,
            fade_duration: None,
            keyframes: None,
        };

        let result = transform_text_segment(&import_seg);

        assert_eq!(result.start, 1.0);
        assert_eq!(result.end, 5.0);
        assert_eq!(result.content, "Hello World");
        assert_eq!(result.center.x, 0.5);
        assert_eq!(result.center.y, 0.5);
        assert_eq!(result.font_size, 50.0);
        assert_eq!(result.font_family, "Inter");
        assert_eq!(result.font_weight, 700.0);
        assert_eq!(result.color, "#FFFFFF");
        assert_eq!(result.fade_duration, 0.0);
        assert!(result.enabled);
    }

    #[test]
    fn test_text_segment_transformation_with_values() {
        let import_seg = ImportTextSegment {
            start: 2.0,
            end: 10.0,
            content: "Custom Text".to_string(),
            center: Some(ImportXY { x: 0.3, y: 0.7 }),
            font_size: Some(60.0),
            font_family: Some("Arial".to_string()),
            font_weight: Some("800".to_string()),
            font_color: Some("#FF0000".to_string()),
            fade_duration: Some(0.5),
            keyframes: Some(ImportTextKeyframes {
                position: vec![
                    ImportVectorKeyframe {
                        time: 0.0,
                        x: 0.2,
                        y: 0.8,
                    },
                    ImportVectorKeyframe {
                        time: 1.0,
                        x: 0.5,
                        y: 0.5,
                    },
                ],
                opacity: vec![
                    ImportScalarKeyframe {
                        time: 0.0,
                        value: 0.0,
                    },
                    ImportScalarKeyframe {
                        time: 0.5,
                        value: 1.0,
                    },
                ],
            }),
        };

        let result = transform_text_segment(&import_seg);

        assert_eq!(result.start, 2.0);
        assert_eq!(result.end, 10.0);
        assert_eq!(result.content, "Custom Text");
        assert_eq!(result.center.x, 0.3);
        assert_eq!(result.center.y, 0.7);
        assert_eq!(result.font_size, 60.0);
        assert_eq!(result.font_family, "Arial");
        assert_eq!(result.font_weight, 800.0);
        assert_eq!(result.color, "#FF0000");
        assert_eq!(result.fade_duration, 0.5);
        assert_eq!(result.keyframes.position.len(), 2);
        assert_eq!(result.keyframes.opacity.len(), 2);
    }

    #[test]
    fn test_scene_mode_transformation() {
        assert!(matches!(
            transform_scene_mode(&ImportSceneMode::Camera),
            SceneMode::CameraOnly
        ));
        assert!(matches!(
            transform_scene_mode(&ImportSceneMode::Screen),
            SceneMode::Default
        ));
        assert!(matches!(
            transform_scene_mode(&ImportSceneMode::SplitScreenLeft),
            SceneMode::SplitScreenLeft
        ));
        assert!(matches!(
            transform_scene_mode(&ImportSceneMode::SplitScreenRight),
            SceneMode::SplitScreenRight
        ));
    }

    #[test]
    fn test_scene_changes_transformation() {
        let changes = vec![
            ImportSceneChange {
                time: 0.0,
                mode: ImportSceneMode::Screen,
            },
            ImportSceneChange {
                time: 5.0,
                mode: ImportSceneMode::SplitScreenRight,
            },
            ImportSceneChange {
                time: 15.0,
                mode: ImportSceneMode::Screen,
            },
        ];

        let segments = transform_scene_changes(&changes, 30.0);

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].start, 0.0);
        assert_eq!(segments[0].end, 5.0);
        assert!(matches!(segments[0].mode, SceneMode::Default));
        assert_eq!(segments[1].start, 5.0);
        assert_eq!(segments[1].end, 15.0);
        assert!(matches!(segments[1].mode, SceneMode::SplitScreenRight));
        assert_eq!(segments[2].start, 15.0);
        assert_eq!(segments[2].end, 30.0);
        assert!(matches!(segments[2].mode, SceneMode::Default));
    }

    #[test]
    fn test_position_value_clamping() {
        let import_seg = ImportTextSegment {
            start: 0.0,
            end: 5.0,
            content: "Test".to_string(),
            center: Some(ImportXY { x: 1.5, y: -0.5 }),
            font_size: None,
            font_family: None,
            font_weight: None,
            font_color: None,
            fade_duration: None,
            keyframes: Some(ImportTextKeyframes {
                position: vec![ImportVectorKeyframe {
                    time: 0.0,
                    x: 2.0,
                    y: -1.0,
                }],
                opacity: vec![ImportScalarKeyframe {
                    time: 0.0,
                    value: 1.5,
                }],
            }),
        };

        let result = transform_text_segment(&import_seg);

        assert_eq!(result.center.x, 1.0);
        assert_eq!(result.center.y, 0.0);
        assert_eq!(result.keyframes.position[0].x, 1.0);
        assert_eq!(result.keyframes.position[0].y, 0.0);
        assert_eq!(result.keyframes.opacity[0].value, 1.0);
    }

    #[test]
    fn test_validation_generates_warnings_for_out_of_range() {
        let import = TimelineImport {
            version: "1.0.0".to_string(),
            text_segments: vec![ImportTextSegment {
                start: 0.0,
                end: 5.0,
                content: "Test".to_string(),
                center: Some(ImportXY { x: 1.5, y: -0.2 }),
                font_size: None,
                font_family: None,
                font_weight: None,
                font_color: None,
                fade_duration: None,
                keyframes: None,
            }],
            scene_changes: vec![],
        };

        let result = validate_import(&import);
        assert!(result.is_ok());
        let warnings = result.unwrap();
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("center.x"));
        assert!(warnings[1].contains("center.y"));
    }

    #[test]
    fn test_empty_scene_changes() {
        let segments = transform_scene_changes(&[], 30.0);
        assert!(segments.is_empty());
    }

    #[test]
    fn test_duplicate_scene_time_validation() {
        let import = TimelineImport {
            version: "1.0.0".to_string(),
            text_segments: vec![],
            scene_changes: vec![
                ImportSceneChange {
                    time: 5.0,
                    mode: ImportSceneMode::Screen,
                },
                ImportSceneChange {
                    time: 5.0,
                    mode: ImportSceneMode::Camera,
                },
            ],
        };

        let result = validate_import(&import);
        assert!(matches!(
            result,
            Err(TimelineImportError::DuplicateSceneTime { time }) if (time - 5.0).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn test_valid_import_no_warnings() {
        let import = TimelineImport {
            version: "1.0.0".to_string(),
            text_segments: vec![ImportTextSegment {
                start: 0.0,
                end: 5.0,
                content: "Valid text".to_string(),
                center: Some(ImportXY { x: 0.5, y: 0.5 }),
                font_size: Some(48.0),
                font_family: None,
                font_weight: None,
                font_color: None,
                fade_duration: None,
                keyframes: None,
            }],
            scene_changes: vec![
                ImportSceneChange {
                    time: 0.0,
                    mode: ImportSceneMode::Screen,
                },
                ImportSceneChange {
                    time: 5.0,
                    mode: ImportSceneMode::Camera,
                },
            ],
        };

        let result = validate_import(&import);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
