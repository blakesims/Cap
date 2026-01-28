# Cap Timeline Import Contract

Version: 1.0.0
Status: DRAFT
Related Task: T005-S03, S04

## Overview

This document defines the JSON schema and behavioral contract for external systems (LLMs) generating Cap timeline configurations. The output should be a partial `ProjectConfiguration` that Cap can merge into an existing project.

## Schema Definition

### Root Object

```typescript
interface TimelineImport {
  version: "1.0.0";
  textSegments: TextSegment[];
  sceneChanges?: SceneChange[];
}
```

### TextSegment

Each text segment represents a single text overlay with optional keyframe animation.

```typescript
interface TextSegment {
  start: number;
  end: number;
  content: string;
  center?: { x: number; y: number };
  fontSize?: number;
  fontFamily?: string;
  fontWeight?: string;
  fontColor?: string;
  fadeDuration?: number;
  keyframes?: TextKeyframes;
}

interface TextKeyframes {
  position?: VectorKeyframe[];
  opacity?: ScalarKeyframe[];
}

interface VectorKeyframe {
  time: number;
  x: number;
  y: number;
}

interface ScalarKeyframe {
  time: number;
  value: number;
}
```

### SceneChange

Scene changes control the layout mode at specific times.

```typescript
interface SceneChange {
  time: number;
  mode: "Camera" | "Screen" | "SplitScreenLeft" | "SplitScreenRight";
}
```

## Field Specifications

### Time Values

| Field | Unit | Range | Notes |
|-------|------|-------|-------|
| `start` | seconds | 0 to video_duration | Segment start time |
| `end` | seconds | start to video_duration | Must be > start |
| `keyframes[].time` | seconds | 0 to (end - start) | **Relative to segment start** |
| `sceneChanges[].time` | seconds | 0 to video_duration | Absolute time |

### Position Values

| Field | Range | Notes |
|-------|-------|-------|
| `center.x` | 0.0 - 1.0 | 0 = left edge, 1 = right edge |
| `center.y` | 0.0 - 1.0 | 0 = top edge, 1 = bottom edge |
| `keyframes.position[].x` | 0.0 - 1.0 | Same as center.x |
| `keyframes.position[].y` | 0.0 - 1.0 | Same as center.y |

### Opacity Values

| Field | Range | Notes |
|-------|-------|-------|
| `keyframes.opacity[].value` | 0.0 - 1.0 | 0 = invisible, 1 = fully visible |

### Text Styling

| Field | Default | Valid Values |
|-------|---------|--------------|
| `fontSize` | 50 | 10 - 200 |
| `fontFamily` | "Inter" | System fonts |
| `fontWeight` | "700" | "400", "500", "600", "700", "800" |
| `fontColor` | "#FFFFFF" | Hex color (#RGB or #RRGGBB) |
| `fadeDuration` | 0.0 | 0.0 - 2.0 seconds |

## Common Patterns

### Staggered Bullet Points

Multiple text segments with sequential start times, each fading in:

```json
{
  "version": "1.0.0",
  "textSegments": [
    {
      "start": 1.0,
      "end": 10.0,
      "content": "First point appears",
      "center": { "x": 0.3, "y": 0.3 },
      "keyframes": {
        "opacity": [
          { "time": 0.0, "value": 0.0 },
          { "time": 0.5, "value": 1.0 }
        ]
      }
    },
    {
      "start": 2.5,
      "end": 10.0,
      "content": "Second point follows",
      "center": { "x": 0.3, "y": 0.45 },
      "keyframes": {
        "opacity": [
          { "time": 0.0, "value": 0.0 },
          { "time": 0.5, "value": 1.0 }
        ]
      }
    },
    {
      "start": 4.0,
      "end": 10.0,
      "content": "Third point last",
      "center": { "x": 0.3, "y": 0.6 },
      "keyframes": {
        "opacity": [
          { "time": 0.0, "value": 0.0 },
          { "time": 0.5, "value": 1.0 }
        ]
      }
    }
  ]
}
```

### Text with Position Animation

Text that slides into view:

```json
{
  "version": "1.0.0",
  "textSegments": [
    {
      "start": 0.0,
      "end": 5.0,
      "content": "Sliding Title",
      "center": { "x": 0.5, "y": 0.3 },
      "keyframes": {
        "position": [
          { "time": 0.0, "x": 0.0, "y": 0.3 },
          { "time": 0.5, "x": 0.5, "y": 0.3 }
        ],
        "opacity": [
          { "time": 0.0, "value": 0.0 },
          { "time": 0.5, "value": 1.0 }
        ]
      }
    }
  ]
}
```

### Split-Screen with Text

Layout change combined with text overlay:

```json
{
  "version": "1.0.0",
  "sceneChanges": [
    { "time": 0.0, "mode": "Screen" },
    { "time": 5.0, "mode": "SplitScreenRight" },
    { "time": 30.0, "mode": "Screen" }
  ],
  "textSegments": [
    {
      "start": 5.5,
      "end": 29.5,
      "content": "Key Takeaway",
      "center": { "x": 0.3, "y": 0.5 },
      "fontSize": 60,
      "fontWeight": "800",
      "keyframes": {
        "opacity": [
          { "time": 0.0, "value": 0.0 },
          { "time": 0.3, "value": 1.0 }
        ]
      }
    }
  ]
}
```

## Validation Rules

### Required Fields

- `version` - Must be "1.0.0"
- `textSegments[].start` - Required
- `textSegments[].end` - Required
- `textSegments[].content` - Required (non-empty string)

### Constraints

1. `end > start` for all segments
2. `keyframes[].time >= 0` (relative times cannot be negative)
3. All position values must be in range [0.0, 1.0]
4. All opacity values must be in range [0.0, 1.0]
5. `sceneChanges` must be sorted by time (ascending)
6. Text segments may overlap in time
7. Keyframes within a property should be sorted by time

### Error Handling

Cap will reject imports with:
- Missing required fields → Error with field name
- Invalid time ranges → Error with segment index
- Out-of-range values → Clamped with warning
- Unknown fields → Ignored (forward compatibility)

## Merge Behavior

When importing into an existing project:

| Mode | Behavior |
|------|----------|
| `replace` | Clear existing textSegments, replace with imported |
| `append` | Add imported segments to existing (default) |
| `merge` | Intelligent merge by time ranges (future) |

## Tips for LLM Generation

1. **Use split-screen for emphasis** - Switch to `SplitScreenRight` when presenting key points
2. **Stagger bullet timing** - 1-2 second gaps between bullet appearances feels natural
3. **Match text position to layout**:
   - In `SplitScreenRight`: camera is right (40%), text goes left (x: 0.2-0.4)
   - In `SplitScreenLeft`: camera is left (40%), text goes right (x: 0.6-0.8)
4. **Keep fade durations short** - 0.3-0.5 seconds for snappy feel
5. **Position vertically spaced bullets** - Use y increments of 0.12-0.15 per line
6. **End segments before transitions** - End text 0.5s before layout changes
