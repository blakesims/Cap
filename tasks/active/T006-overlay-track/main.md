# Task: T006 - Overlay Track System

## 0. Task Summary
- **Task Name:** Overlay Track System
- **Priority:** 1
- **Number of Stories:** 6
- **Current Status:** S05_COMPLETE
- **Dependencies:** T005 (split-screen + text keyframes - COMPLETE)
- **Rules Required:** CLAUDE.md

## 1. Goal / Objective

Create a unified **Overlay Track** that couples text overlays with scene layouts. This enforces the constraint that split-screen and text always go together, making editing simpler and less error-prone.

### Design Philosophy
See [editing-philosophy.md](../T005-split-screen-text-overlays/editing-philosophy.md) for the full context.

**Core principle:** Text overlay ↔ Layout change are coupled. Never independent.

## 2. Overlay Types

### Type 1: Split (50/50)
```
┌────────────────────────────┬────────────────────────────┐
│                            │                            │
│   BACKGROUND + TEXT        │   CAMERA (cropped/centered)│
│   • Bullet point           │   (uses crop settings)     │
│   • Another point          │                            │
│                            │                            │
└────────────────────────────┴────────────────────────────┘
```

- **Camera:** Right 50%, uses existing crop settings to frame subject
- **Text:** Left 50%, bullet points or numbered list
- **Background:** Cap's existing background system
- **Enter animation:** Camera slides right + text slides in from left (ease-in-out)
- **Exit animation:** Reverse (text slides out, camera expands)

### Type 2: FullScreen (Text + PiP)
```
┌─────────────────────────────────────────────────────────┐
│                                                         │
│   • Bullet point one                        ┌─────────┐ │
│   • Bullet point two                        │ CAMERA  │ │
│   • Bullet point three                      │  (PiP)  │ │
│                                             └─────────┘ │
│                    OR                                   │
│                                                         │
│              Centered Title Text                        │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

- **Camera:** PiP in corner (bottom-right)
- **Content:** Full-width text area (bullets, numbered, or centered title)
- **Background:** Cap's existing background
- **Audio:** Continues
- **Use case:** Chapter cards, full-width bullet lists

## 3. Stories Breakdown

| Story ID | Story Name | Status | Estimate |
|----------|------------|--------|----------|
| S01 | OverlaySegment type + configuration | Complete | 1 day |
| S02 | Overlay → Scene+Text generation | Planned | 1-2 days |
| S03 | Split overlay enter/exit animations | Planned | 1-2 days |
| S04 | OverlayTrack.tsx UI component | Planned | 2-3 days |
| S05 | Overlay JSON import | Complete | 1 day |
| S06 | Item timing editor UI | Planned | 1-2 days |

**Total estimate:** 7-11 days

---

## 4. Story Details

### S01 - OverlaySegment Type + Configuration

**Objective:** Define the data model for overlay segments.

**Deliverables:**
- Add `OverlaySegment` type to `crates/project/src/configuration.rs`
- Add `overlay_segments: Vec<OverlaySegment>` to `TimelineConfiguration`
- TypeScript types generated via tauri-specta

**Data Model:**
```rust
pub enum OverlayType {
    Split,      // 50/50 camera + text
    FullScreen, // No camera, just background + text
}

pub enum OverlayItemStyle {
    Title,    // Large, centered
    Bullet,   // • prefixed, left-aligned
    Numbered, // 1. prefixed, left-aligned
}

pub struct OverlayItem {
    pub delay: f64,           // Seconds after segment start
    pub content: String,
    pub style: OverlayItemStyle,
}

pub struct OverlaySegment {
    pub start: f64,
    pub end: f64,
    pub overlay_type: OverlayType,
    pub items: Vec<OverlayItem>,
}
```

**Acceptance Criteria:**
- [x] Types compile and serialize correctly
- [x] Existing projects without overlays still load (`#[serde(default)]`)
- [ ] TypeScript bindings generated (auto-generated on desktop app load)

---

### S02 - Overlay → Scene+Text Generation

**Objective:** Convert overlay segments to scene + text segments at render time.

**Approach:** Overlay is a higher-level abstraction. The rendering pipeline doesn't change - we generate the low-level segments it already understands.

**Deliverables:**
- Function: `generate_scene_segments(overlays: &[OverlaySegment]) -> Vec<SceneSegment>`
- Function: `generate_text_segments(overlays: &[OverlaySegment]) -> Vec<TextSegment>`
- Integration point in rendering pipeline

**Text Positioning:**
| Style | Position | Font Size | Alignment |
|-------|----------|-----------|-----------|
| Title | center (0.5, 0.5) | 64 | Center |
| Bullet | left (0.25, variable Y) | 40 | Left |
| Numbered | left (0.25, variable Y) | 40 | Left |

**Vertical Spacing:**
- First item: Y = 0.25
- Subsequent items: Y += 0.12

**Acceptance Criteria:**
- [ ] Split overlay generates scene segment (50/50 split, camera right)
- [ ] FullScreen overlay generates scene segment with PiP mode + text
- [ ] Text segments generated with correct positions
- [ ] Item delays converted to absolute start times
- [ ] Slide-from-left + fade-in keyframes added (0.3s)
- [ ] Warning toast if item delay exceeds segment duration

---

### S03 - Split Overlay Enter/Exit Animations

**Objective:** Smooth camera transitions for split overlays.

**Enter Animation (from full camera):**
1. Camera slides from full-width to right 50%
2. Camera crops to keep subject centered
3. Background fades in on left
4. Text items slide/fade in

**Exit Animation (to full camera):**
1. Text slides out left
2. Background fades out
3. Camera expands back to full width

**Exit Animation (to PiP):**
- Reuse existing full→PiP transition
- Background + text fade out as camera shrinks to corner

**Deliverables:**
- Review current split-screen transition code
- Add "slide" feel if not already present
- Ensure text exit animation works

**Acceptance Criteria:**
- [ ] Split enter looks like camera sliding right
- [ ] Split exit to full camera reverses smoothly
- [ ] Split exit to PiP reuses existing transition
- [ ] No jarring cuts

---

### S04 - OverlayTrack.tsx UI Component

**Objective:** New timeline track for managing overlays.

**Behavior:**
- Segments displayed as colored bars (different color per type)
- Drag segment: moves entire overlay
- Resize handles: change start/end time
- Click segment: select it
- Double-click: open item editor

**Visual Design:**
```
Overlay Track:
├─[Split: 3 items]──────┤    ├─[Full: "Step 2"]─┤
```

**Deliverables:**
- `OverlayTrack.tsx` component
- Segment rendering with type indicators
- Drag/resize functionality
- Selection state integration

**Acceptance Criteria:**
- [ ] Track appears in timeline when overlays exist
- [ ] Segments draggable
- [ ] Segments resizable
- [ ] Selection works
- [ ] Visual distinction between Split and FullScreen

---

### S05 - Overlay JSON Import

**Objective:** Simplified import format for LLM-generated overlays.

**JSON Schema:**
```json
{
  "version": "2.0.0",
  "overlays": [
    {
      "type": "split",
      "start": 10.0,
      "end": 45.0,
      "items": [
        { "delay": 0.5, "text": "Overview", "style": "title" },
        { "delay": 2.0, "text": "First point", "style": "bullet" },
        { "delay": 4.0, "text": "Second point", "style": "bullet" }
      ]
    },
    {
      "type": "fullscreen",
      "start": 45.0,
      "end": 48.0,
      "items": [
        { "delay": 0.0, "text": "Step 2", "style": "title" }
      ]
    }
  ]
}
```

**Deliverables:**
- Update `timeline_import.rs` to handle v2.0.0 format
- Validation for overlay-specific rules
- Convert to OverlaySegment types

**Acceptance Criteria:**
- [x] v2.0.0 imports create overlay segments
- [x] v1.0.0 still works (backwards compatible)
- [x] Validation errors are clear
- [x] Success toast shows overlay count (via ImportResult.overlay_segments_imported)

---

### S06 - Item Timing Editor UI

**Objective:** Easy editing of overlay items without touching JSON.

**UI Concept:**
```
┌─────────────────────────────────────────┐
│ Edit Overlay                            │
├─────────────────────────────────────────┤
│ Type: [Split ▼]                         │
│                                         │
│ Items:                                  │
│ ┌─────────────────────────────────────┐ │
│ │ [0.5s] Overview          [Title ▼]  │ │
│ │ [2.0s] First point       [Bullet ▼] │ │
│ │ [4.0s] Second point      [Bullet ▼] │ │
│ │                      [+ Add Item]   │ │
│ └─────────────────────────────────────┘ │
│                                         │
│           [Cancel]  [Save]              │
└─────────────────────────────────────────┘
```

**Deliverables:**
- Modal/panel component for editing overlay
- Editable item list (reorder, add, delete)
- Delay input per item
- Style dropdown per item
- Content text input

**Acceptance Criteria:**
- [ ] Double-click overlay opens editor
- [ ] Can edit item text
- [ ] Can change item delays
- [ ] Can add/remove items
- [ ] Can change item style
- [ ] Changes save to project

---

## 5. Technical Considerations

### Rendering Pipeline
- Overlay segments are **converted** to scene + text segments
- No changes to actual rendering code
- Conversion happens when timeline is processed

### Track Precedence
- When overlay segment is active, it controls the scene
- Existing scene track segments are ignored during overlay
- Gap between overlays → default PiP mode

### State Management
- `OverlaySegment` stored in `ProjectConfiguration`
- Editing overlays uses same `projectActions` pattern
- Selection state: add `overlay` to `TimelineSelectionType`

### Animation Timing
- Enter transition: 300ms ease-in-out
- Text fade-in: 300ms per item
- Exit transition: 300ms ease-in-out

---

## 6. Decisions Made

| Question | Decision |
|----------|----------|
| Split ratio | **50/50** |
| Camera position | Fixed: **RIGHT** |
| Camera centering | Use **existing crop settings** (not auto face detection) |
| PiP position | Fixed: **bottom-right** |
| Background | Cap's existing system |
| Text animation | **Slide from left + fade** (ease-in-out) |
| Gap between overlays | **Allow tiny gaps** (no auto-merge) |
| FullScreen type | **PiP + text overlay** (not camera hidden) |
| Track visibility | **User manages** (no auto-hide of Scene/Text tracks) |
| Item delay > duration | **Warning toast** (don't block import) |
| Complex graphics | Deferred (simple text first) |

---

## 7. Files to Create/Modify

### New Files
- `apps/desktop/src/routes/editor/Timeline/OverlayTrack.tsx`
- `apps/desktop/src/routes/editor/OverlayEditor.tsx`

### Modified Files
- `crates/project/src/configuration.rs` - Add OverlaySegment types
- `crates/rendering/src/lib.rs` - Generate segments from overlays
- `apps/desktop/src-tauri/src/timeline_import.rs` - v2.0.0 import
- `apps/desktop/src/routes/editor/Timeline/index.tsx` - Add overlay track
- `apps/desktop/src/routes/editor/context.ts` - Overlay selection type

---

## 8. Reuse from T005

| Component | Reuse | Notes |
|-----------|-------|-------|
| `timeline_import.rs` | 80% | Adapt for v2.0.0 schema |
| `scene.rs` | 100% | Split-screen rendering works |
| `text.rs` | 100% | Keyframe interpolation works |
| TextTrack.tsx patterns | 70% | Adapt for OverlayTrack |
| Scene transitions | 100% | Already have smooth transitions |

---

## 9. Success Criteria

After T006 is complete:
1. [ ] Can import LLM-generated overlay JSON
2. [ ] Overlays appear as single track segments
3. [ ] Dragging overlay moves layout + text together
4. [ ] Can edit overlay items in UI
5. [ ] Smooth enter/exit animations
6. [ ] Exit to PiP works correctly
7. [ ] Gaps between overlays show default PiP view
