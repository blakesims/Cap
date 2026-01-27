# Task: T005 - Split-Screen with Animated Text Overlays

## 0. Task Summary
- **Task Name:** Split-Screen with Animated Text Overlays
- **Priority:** 2
- **Number of Stories:** 5
- **Current Status:** PLANNING
- **Dependencies:** crates/rendering/, crates/project/, apps/desktop/src/routes/editor/
- **Rules Required:** CLAUDE.md
- **Acceptance Criteria:**
  - [ ] New split-screen layout modes available (SplitScreenLeft, SplitScreenRight)
  - [ ] Animated transitions between all layout modes (300ms, matching existing)
  - [ ] Text segments support keyframe animation (position, opacity)
  - [ ] Bullet points can appear with staggered timing
  - [ ] JSON import allows LLM-generated timelines to be loaded
  - [ ] Text easily editable after import

## 1. Goal / Objective

Enable a "codified editing style" where recordings can use split-screen layouts with animated text bullet points. An external LLM workflow (`kb edit`) generates JSON describing when to switch layouts and when each text point appears; Cap renders this with smooth animations and allows manual tweaks.

## 2. Overall Status

PLANNING - Feasibility confirmed. Architecture supports extension. Research complete. Plan reviewed and updated with identified gaps.

## 3. Stories Breakdown

| Story ID | Story Name / Objective | Status | Deliverable | Estimate |
| :--- | :--- | :--- | :--- | :--- |
| S01 | Add Split-Screen Layout Modes | Planned | New SceneMode variants + layout calculations | 3-5 days |
| S02 | Text Keyframe Animation System | Planned | Generic keyframe structs + interpolation for text | 1-2 days |
| S03 | Staggered Bullet Point Rendering | Planned | Multi-text segments with timed appearance | 0.5 day |
| S04 | JSON Timeline Import | Planned | Import command with validation | 2-3 days |
| S05 | Editor UI Polish | Planned | UI for new modes + keyframe editing | 1-4 days |

**Total Estimate: 7.5-14.5 days**

---

## 4. Story Details

### S01 - Add Split-Screen Layout Modes

**Objective:** Extend SceneMode with split-screen layouts where camera occupies one side and text/content area occupies the other.

**Acceptance Criteria:**
- [ ] Add `SplitScreenLeft` mode (camera left 40%, content right 60%)
- [ ] Add `SplitScreenRight` mode (camera right 40%, content left 60%)
- [ ] Transitions use existing 300ms bezier easing system
- [ ] Camera fills its area with cropping (like CameraOnly mode)
- [ ] Display layer scales to fit content area
- [ ] All 3 `same_mode` pattern matches updated in scene.rs
- [ ] Handle no-camera case gracefully (fall back to Default or show placeholder)

**Pre-Implementation Checklist:**
- [ ] Map all `same_mode` pattern locations in scene.rs (3 occurrences)
- [ ] Create test matrix for mode transitions (5x5 = 25 combinations)
- [ ] Decide camera behavior: fill-crop vs letterbox (recommend: fill-crop)

**Files to Modify:**

1. `crates/project/src/configuration.rs` (lines 762-769):
```rust
pub enum SceneMode {
    #[default]
    Default,
    CameraOnly,
    HideCamera,
    SplitScreenLeft,   // NEW
    SplitScreenRight,  // NEW
}
```

2. `crates/rendering/src/scene.rs` - **THREE locations need updating:**

   a. Lines 93-127 - `same_mode` pattern in transition logic:
   ```rust
   let same_mode = matches!(
       (&prev_seg.mode, &segment.mode),
       (SceneMode::CameraOnly, SceneMode::CameraOnly)
           | (SceneMode::Default, SceneMode::Default)
           | (SceneMode::HideCamera, SceneMode::HideCamera)
           | (SceneMode::SplitScreenLeft, SceneMode::SplitScreenLeft)   // NEW
           | (SceneMode::SplitScreenRight, SceneMode::SplitScreenRight) // NEW
   );
   ```

   b. Lines 167-174 - second `same_mode` pattern

   c. Lines 286-292 - `get_scene_values`:
   ```rust
   fn get_scene_values(mode: &SceneMode) -> (f64, f64, f64) {
       match mode {
           SceneMode::Default => (1.0, 1.0, 1.0),
           SceneMode::CameraOnly => (1.0, 1.0, 1.0),
           SceneMode::HideCamera => (0.0, 1.0, 1.0),
           SceneMode::SplitScreenLeft => (1.0, 1.0, 1.0),   // NEW
           SceneMode::SplitScreenRight => (1.0, 1.0, 1.0),  // NEW
       }
   }
   ```

3. `crates/rendering/src/lib.rs` (lines 1050-1170, 1496-1733):
   - Modify `display_offset` and `display_size` for split-screen content area
   - Add new layout calculation branch for split-screen camera bounds
   - Camera bounds: 40% width, full height, positioned left or right
   - Display bounds: 60% width, full height, opposite side
   - Use CameraOnly-style aspect ratio handling (fill with crop)

4. `apps/desktop/src/routes/editor/Timeline/SceneTrack.tsx` (lines 62-82):
   - Add icons: `IconLucideLayoutPanelLeft`, `IconLucideLayoutPanelRight`
   - Add labels: "Split Left", "Split Right"

5. `apps/desktop/src/routes/editor/ConfigSidebar.tsx` (lines 3523-3627):
   - Extend KTabs with two new mode options
   - Update visual indicator positions (5 stops instead of 3)

**Edge Cases:**
- No camera feed: Fall back to Default mode, log warning
- Camera aspect ratio mismatch: Use fill-crop (same as CameraOnly)
- Transition CameraOnly ↔ SplitScreen: Animate camera size/position smoothly

**Technical Notes:**
- Split-screen camera crops to fill its area (like CameraOnly aspect handling)
- Display layer scales proportionally; may letterbox if aspect differs significantly
- Transition from Default → SplitScreen animates camera sliding to side
- Zoom segments apply only to content area in split-screen mode

---

### S02 - Text Keyframe Animation System

**Objective:** Create a generic keyframe system and apply it to text segments for animated position and opacity.

**Acceptance Criteria:**
- [ ] Generic `Keyframe<T>` type usable by both masks and text
- [ ] TextSegment supports `keyframes` field with position and opacity arrays
- [ ] Keyframes use relative time (seconds from segment start)
- [ ] Linear interpolation between keyframes (matching mask behavior)
- [ ] Keyframes sorted once on load, not per-frame
- [ ] Fade duration still works as additional multiplier
- [ ] Backwards compatible (no keyframes = current behavior)
- [ ] TypeScript types updated in text.ts

**Pre-Implementation Checklist:**
- [ ] Decide: refactor mask to use generic types, or add new generic types alongside
- [ ] Verify TypeScript type generation works after Rust changes

**Files to Modify:**

1. `crates/project/src/configuration.rs` - Add generic keyframe types:
```rust
#[derive(Type, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScalarKeyframe {
    pub time: f64,
    pub value: f64,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct VectorKeyframe {
    pub time: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TextKeyframes {
    #[serde(default)]
    pub position: Vec<VectorKeyframe>,
    #[serde(default)]
    pub opacity: Vec<ScalarKeyframe>,
}
```

2. `crates/project/src/configuration.rs` (TextSegment, add field):
```rust
#[serde(default)]
pub keyframes: TextKeyframes,
```

3. `crates/rendering/src/text.rs` - Add interpolation functions:
```rust
fn interpolate_vector(base: XY<f64>, keys: &[VectorKeyframe], time: f64) -> XY<f64> {
    if keys.is_empty() { return base; }
    // Sort once assumption: keys should be pre-sorted
    // ... linear interpolation logic from mask.rs
}

fn interpolate_scalar(base: f64, keys: &[ScalarKeyframe], time: f64) -> f64 {
    if keys.is_empty() { return base; }
    // ... linear interpolation logic from mask.rs
}
```

4. `crates/rendering/src/text.rs` (modify prepare_texts, lines 66-90):
```rust
let relative_time = (frame_time - segment.start).max(0.0);
let center = interpolate_vector(segment.center, &segment.keyframes.position, relative_time);
let keyframe_opacity = interpolate_scalar(1.0, &segment.keyframes.opacity, relative_time);

let opacity = if fade_duration > 0.0 {
    let fade_in = (time_since_start / fade_duration).min(1.0);
    let fade_out = (time_until_end / fade_duration).min(1.0);
    (fade_in * fade_out * keyframe_opacity) as f32
} else {
    keyframe_opacity as f32
};
```

5. **NEW** `apps/desktop/src/routes/editor/text.ts` - Update TypeScript type:
```typescript
export type ScalarKeyframe = {
  time: number;
  value: number;
};

export type VectorKeyframe = {
  time: number;
  x: number;
  y: number;
};

export type TextKeyframes = {
  position: VectorKeyframe[];
  opacity: ScalarKeyframe[];
};

export type TextSegment = {
  // ... existing fields ...
  keyframes: TextKeyframes;
};

export function defaultTextSegment(start: number, end: number): TextSegment {
  return {
    // ... existing defaults ...
    keyframes: { position: [], opacity: [] },
  };
}
```

**Edge Cases:**
- Keyframe time < 0: Treat as 0 (clamp)
- Keyframe time > segment duration: Use last keyframe value
- Single keyframe: Return that value for all times
- Keyframes out of order: Sort on load (defensive, but validate in import)

**Technical Notes:**
- Consider refactoring MaskKeyframes to use shared types (optional, can defer)
- Keyframes stored in project config are validated on import
- Empty keyframes array = use base segment values (backward compatible)

---

### S03 - Staggered Bullet Point Rendering

**Objective:** Enable multiple text segments to appear sequentially as bullet points with controlled timing.

**Acceptance Criteria:**
- [ ] Text segments can be grouped visually (same X position, stacked Y positions)
- [ ] Each bullet has independent appear time via opacity keyframes
- [ ] Text content supports multi-line with bullet prefix (•, -, 1., etc.)
- [ ] Default bullet animation: fade in over 0.15s
- [ ] Document recommended patterns for LLM output

**Implementation Approach:**

This is a usage pattern, not new code. Leverage existing text segments with keyframes:

1. Each bullet = separate TextSegment with:
   - Same `center.x` position (e.g., 0.7 for right side in split-screen)
   - Incrementing `center.y` positions (0.25, 0.35, 0.45, etc.)
   - Opacity keyframes for staggered appearance

2. Alternative: Single text segment with multi-line content, entire block fades in

**LLM Output Format:**
```json
{
  "text_segments": [
    {
      "start": 15.0, "end": 45.0,
      "content": "• Step 1: Load footage",
      "center": { "x": 0.7, "y": 0.25 },
      "size": { "x": 0.5, "y": 0.08 },
      "fontSize": 48,
      "fontWeight": 700,
      "color": "#ffffff",
      "keyframes": {
        "position": [],
        "opacity": [
          { "time": 0.0, "value": 0.0 },
          { "time": 0.15, "value": 1.0 }
        ]
      }
    },
    {
      "start": 15.0, "end": 45.0,
      "content": "• Step 2: Cut mistakes",
      "center": { "x": 0.7, "y": 0.35 },
      "size": { "x": 0.5, "y": 0.08 },
      "fontSize": 48,
      "fontWeight": 700,
      "color": "#ffffff",
      "keyframes": {
        "position": [],
        "opacity": [
          { "time": 0.0, "value": 0.0 },
          { "time": 2.5, "value": 0.0 },
          { "time": 2.65, "value": 1.0 }
        ]
      }
    }
  ]
}
```

**Files to Modify:**
- None beyond S02 - this is documentation and LLM prompt engineering
- Optional: Add `createStaggeredBullets()` helper in text.ts

**Coordinate System Note:**
Text coordinates are **absolute** (full-frame normalized 0-1), not relative to content area. In split-screen right mode, text in the left content area uses X coordinates ~0.0-0.55.

---

### S04 - JSON Timeline Import

**Objective:** Allow importing LLM-generated JSON to populate scene and text segments with comprehensive validation.

**Acceptance Criteria:**
- [ ] New Tauri command `import_timeline_json` accepts JSON string
- [ ] JSON format matches Cap's internal segment structures (camelCase)
- [ ] Import can replace or merge with existing segments
- [ ] Comprehensive validation with clear error messages
- [ ] Command registered in `collect_commands!` macro
- [ ] Editor refreshes automatically via project config watcher
- [ ] TypeScript types generated for import command

**Pre-Implementation Checklist:**
- [ ] Define JSON schema validation rules
- [ ] Add command to `collect_commands!` macro
- [ ] Test project config watcher updates frontend

**JSON Import Schema:**
```json
{
  "scene_segments": [
    { "start": 0.0, "end": 15.0, "mode": "default" },
    { "start": 15.0, "end": 45.0, "mode": "splitScreenRight" }
  ],
  "text_segments": [
    {
      "start": 15.0,
      "end": 45.0,
      "enabled": true,
      "content": "Key Points",
      "center": { "x": 0.25, "y": 0.2 },
      "size": { "x": 0.4, "y": 0.1 },
      "fontFamily": "sans-serif",
      "fontSize": 64,
      "fontWeight": 700,
      "italic": false,
      "color": "#ffffff",
      "fadeDuration": 0.15,
      "keyframes": {
        "position": [],
        "opacity": [
          { "time": 0.0, "value": 0.0 },
          { "time": 0.3, "value": 1.0 }
        ]
      }
    }
  ]
}
```

**Validation Rules:**
- `end > start` for all segments
- `start >= 0` for all segments
- No overlapping scene segments (warn, don't error)
- Valid mode strings: `default`, `cameraOnly`, `hideCamera`, `splitScreenLeft`, `splitScreenRight`
- Coordinates in valid ranges: center 0-1, size 0-2, opacity 0-1
- fontSize 8-200, fontWeight 100-900
- Valid hex color format
- Keyframe times >= 0
- Keyframes sorted by time (sort if not, warn)

**Files to Create/Modify:**

1. `apps/desktop/src-tauri/src/lib.rs` (new command):
```rust
#[tauri::command]
#[specta::specta]
async fn import_timeline_json(
    editor_instance: WindowEditorInstance,
    json_str: String,
    merge: bool,
) -> Result<(), String> {
    let import: TimelineImport = serde_json::from_str(&json_str)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // Validate segments
    import.validate().map_err(|e| format!("Validation error: {}", e))?;

    let mut config = ProjectConfiguration::load(&editor_instance.project_path)
        .unwrap_or_default();

    if let Some(timeline) = &mut config.timeline {
        if merge {
            timeline.scene_segments.extend(import.scene_segments);
            timeline.text_segments.extend(import.text_segments);
            // Sort by start time after merge
            timeline.scene_segments.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
            timeline.text_segments.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        } else {
            timeline.scene_segments = import.scene_segments;
            timeline.text_segments = import.text_segments;
        }
    }

    config.write(&editor_instance.project_path).map_err(|e| e.to_string())?;
    editor_instance.project_config.0.send(config).ok();

    Ok(())
}
```

2. `apps/desktop/src-tauri/src/lib.rs` - **Register command** (lines 2596-2704):
```rust
tauri_specta::collect_commands![
    // ... existing commands ...
    import_timeline_json,  // NEW
]
```

3. `crates/project/src/configuration.rs` (new structs):
```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineImport {
    #[serde(default)]
    pub scene_segments: Vec<SceneSegment>,
    #[serde(default)]
    pub text_segments: Vec<TextSegment>,
}

impl TimelineImport {
    pub fn validate(&self) -> Result<(), String> {
        for (i, seg) in self.scene_segments.iter().enumerate() {
            if seg.end <= seg.start {
                return Err(format!("Scene segment {}: end must be > start", i));
            }
            if seg.start < 0.0 {
                return Err(format!("Scene segment {}: start must be >= 0", i));
            }
        }
        for (i, seg) in self.text_segments.iter().enumerate() {
            if seg.end <= seg.start {
                return Err(format!("Text segment {}: end must be > start", i));
            }
            // ... additional validation
        }
        Ok(())
    }
}
```

4. `apps/desktop/src/routes/editor/` (UI trigger):
   - Add "Import Timeline" button in editor header or menu
   - File picker for JSON or paste textarea dialog
   - Call `commands.importTimelineJson(jsonStr, merge)`
   - Show toast on success/error

**Edge Cases:**
- Empty JSON: Valid, clears segments (in replace mode)
- JSON with only scene_segments: Valid, text_segments defaults to []
- Malformed JSON: Clear error message with line/column
- Very long segment arrays: No limit, but may affect performance

**Technical Notes:**
- Uses existing serde infrastructure (camelCase matching)
- Merge mode preserves existing segments, extends arrays, re-sorts
- Replace mode overwrites scene/text segments only (preserves zoom, mask, clips)
- Project config watcher (`editor_instance.project_config.0`) automatically updates frontend
- Rebuild desktop app after adding command to regenerate tauri.ts

---

### S05 - Editor UI Polish

**Objective:** Expose new functionality in the editor UI with intuitive controls.

**Acceptance Criteria:**
- [ ] Scene mode selector shows all 5 modes with clear icons
- [ ] Import button accessible from editor header
- [ ] Preview shows split-screen layouts correctly
- [ ] Caption positioning adjusted for split-screen modes
- [ ] (MVP) Keyframes edited via JSON import only
- [ ] (Post-MVP) Visual keyframe editor for text

**UI Components:**

1. **Scene Mode Selector** (ConfigSidebar.tsx):
   - 5-option selector (tabs or dropdown)
   - Icons: Monitor, Video, EyeOff, LayoutPanelLeft, LayoutPanelRight
   - Visual preview showing layout arrangement

2. **Import Timeline Dialog** (new component):
   - Modal with file picker OR paste textarea
   - Preview count: "X scene segments, Y text segments"
   - Merge vs Replace radio buttons
   - Import button with loading state
   - Error display area

3. **Caption Positioning** (deferred or simple fix):
   - In split-screen modes, adjust caption Y positions to stay in content area
   - Or: document that captions should be disabled in split-screen

**Files to Modify:**
- `apps/desktop/src/routes/editor/ConfigSidebar.tsx` - Extend SceneSegmentConfig with 5 modes
- `apps/desktop/src/routes/editor/Header.tsx` or similar - Add Import button
- New: `apps/desktop/src/routes/editor/ImportTimelineDialog.tsx`

**MVP Scope:**
- Scene modes: 5-option dropdown (simpler than tabs)
- Import: Basic paste dialog with merge/replace
- No visual keyframe editor (use JSON import)
- Caption positioning: Document limitation

**Post-MVP Enhancements:**
- Visual keyframe editor with timeline scrubbing
- Drag-to-position text on canvas
- Split ratio configuration (40/60 → configurable)
- Text size keyframes

---

## 5. Technical Considerations

### Architecture Decisions

1. **Split-screen as SceneMode variants** (not separate system)
   - Leverages existing transition infrastructure
   - Consistent with current architecture
   - Requires updating 3 pattern matches in scene.rs

2. **Generic keyframe types** (shared between mask and text)
   - Reduces code duplication
   - Enables future reuse (zoom keyframes, etc.)
   - May require minor mask refactor or parallel types

3. **JSON import over custom format**
   - Matches existing project-config.json structure
   - Easy to generate from external tools (LLM)
   - Validated via serde + custom validation

4. **Text coordinates are absolute (full-frame)**
   - Consistent with existing behavior
   - LLM must know split-screen layout to position text correctly
   - Documented in JSON schema

### Performance Considerations

- Text keyframe interpolation runs per-frame (same as masks) - negligible overhead
- Keyframes should be sorted on load, not per-frame
- Split-screen rendering uses existing GPU pipeline - no new shaders needed
- JSON import is one-time operation - no runtime impact

### Migration & Compatibility

- New SceneMode variants are additive (old projects unaffected)
- TextKeyframes field has `#[serde(default)]` (old segments work)
- No database migrations needed (client-side only)
- TypeScript type regeneration required after Rust changes

### Interaction with Other Features

| Feature | Split-Screen Interaction |
|---------|-------------------------|
| Zoom segments | Apply only to content area (60% side) |
| Captions | May need repositioning; document limitation |
| Masks | Work as normal (full-frame coordinates) |
| Cursor | Visible only in content area |

### External Workflow Integration

The `kb edit` workflow (outside this repo) will:
1. Transcribe recording audio per-segment
2. Analyze transcript for structure (steps, key points)
3. Generate JSON with suggested:
   - Scene mode changes at topic boundaries
   - Text segments with bullet points (absolute coordinates)
   - Staggered appearance keyframes
4. User imports JSON into Cap
5. Cap renders with animations
6. User tweaks timing/content in editor

---

## 6. Risk Assessment

### High Risk: S01 Split-Screen Rendering
- Touches core rendering pipeline
- Multiple integration points (scene transitions, zoom, display bounds)
- Mode-specific code scattered across files
- **Mitigation:** Pre-implementation checklist, thorough testing of transition matrix

### Medium Risk: S04 JSON Import Validation
- Many edge cases for invalid input
- Must not corrupt project config
- **Mitigation:** Comprehensive validation, atomic writes, clear error messages

### Scope Creep Risks
- Configurable split ratios → Defer
- Text size/color keyframes → Defer
- Camera position in split (top/bottom) → Defer
- Visual keyframe editor → Post-MVP

---

## 7. Implementation Order

**Phase 1 (Core Rendering):** S02 → S01
- Text keyframes first (self-contained, testable immediately)
- Split-screen second (more dependencies, uses keyframes for testing)

**Phase 2 (Integration):** S04 → S03
- JSON import enables LLM workflow
- Staggered bullets are usage pattern, not code

**Phase 3 (Polish):** S05
- UI improvements based on Phase 1-2 usage
- Can be incremental/iterative

---

## 8. Relevant Rules

- `CLAUDE.md` - Project conventions, Rust clippy rules, no code comments
- Desktop editor patterns from CLAUDE.md:
  - `projectActions` for saved config mutations
  - `editorActions` for session state
  - Keyboard shortcuts via `normalizeCombo()`
  - Toast notifications via `solid-toast`

---

## 9. Files Summary

### Critical Files
| File | Changes |
|------|---------|
| `crates/project/src/configuration.rs` | SceneMode enum, keyframe structs, TextSegment, TimelineImport |
| `crates/rendering/src/scene.rs` | 3x same_mode patterns, get_scene_values |
| `crates/rendering/src/lib.rs` | Split-screen layout calculations (display_offset, camera bounds) |
| `crates/rendering/src/text.rs` | Keyframe interpolation, prepare_texts modification |
| `apps/desktop/src/routes/editor/text.ts` | TypeScript types for keyframes |
| `apps/desktop/src-tauri/src/lib.rs` | import_timeline_json command + registration |

### UI Files
| File | Changes |
|------|---------|
| `apps/desktop/src/routes/editor/Timeline/SceneTrack.tsx` | New mode icons/labels |
| `apps/desktop/src/routes/editor/ConfigSidebar.tsx` | 5-mode selector |
| `apps/desktop/src/routes/editor/ImportTimelineDialog.tsx` | New component |

---

## 10. Open Questions (Resolved)

| Question | Decision |
|----------|----------|
| Configurable split ratios? | Fixed 40/60 for MVP |
| Text size keyframes? | Defer to post-MVP |
| Zoom in split-screen? | Apply only to content area |
| Text coordinates? | Absolute (full-frame) |
| Caption positioning? | Document limitation for MVP |
| Generic vs duplicate keyframe types? | Generic (reduce duplication) |
