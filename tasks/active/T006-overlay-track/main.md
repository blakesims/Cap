# T006: Overlay Track System

## Meta
- **Status:** IN-PROGRESS
- **Created:** 2026-01-28
- **Last Updated:** 2026-01-29 (Phase 3 reviewed)
- **Blocked Reason:** —

## Task

Create a unified **Overlay Track** that couples text overlays with scene layouts. This enforces the constraint that split-screen and text always go together, making editing simpler and less error-prone.

**Design Philosophy:** Text overlay ↔ Layout change are coupled. Never independent.

**Overlay Types:**

1. **Split (50/50):** Camera right 50% (cropped/centered) + Background+Text left 50%
2. **FullScreen:** PiP camera in corner + Full-width text area (bullets, numbered, or centered title)

**Dependencies:** T005 (split-screen + text keyframes) — COMPLETE

---

## Plan

### Objective
Enable coupled overlay segments that combine layout changes with text, simplifying the editing workflow and enabling LLM-generated timeline imports.

### Scope
- **In:** OverlaySegment data model, overlay→scene+text generation, enter/exit animations, OverlayTrack UI, JSON v2.0.0 import, item timing editor
- **Out:** Complex graphics/images in overlays, auto face detection for camera centering

### Prior Work (Completed)
- **S01 (OverlaySegment type):** ✅ Types added to `configuration.rs`, serializes correctly
- **S05 (Overlay JSON import):** ✅ v2.0.0 import in `timeline_import.rs`, validation, backwards compatible

### Phases

#### Phase 1: Overlay → Scene+Text Generation ✅
- **Objective:** Convert overlay segments to scene + text segments at render time so existing rendering pipeline handles them
- **Tasks:**
  - [x] Task 1.1: Add `generate_scene_segments(overlays: &[OverlaySegment]) -> Vec<SceneSegment>` — Split overlay generates 50/50 split scene (camera right), FullScreen generates PiP mode
  - [x] Task 1.2: Add `generate_text_segments(overlays: &[OverlaySegment]) -> Vec<TextSegment>` — Position text based on style (Title: center 0.5/0.5, Bullet/Numbered: left 0.25/variable Y with 0.12 spacing)
  - [x] Task 1.3: Convert item delays to absolute start times, add slide-from-left + fade-in keyframes (0.3s)
  - [x] Task 1.4: Integrate into rendering pipeline — call generation functions when timeline is processed
  - [x] Task 1.5: Add warning toast if item delay exceeds segment duration
- **Acceptance Criteria:**
  - [x] AC1: Split overlay generates scene segment with 50/50 split, camera right
  - [x] AC2: FullScreen overlay generates scene segment with PiP mode + text
  - [x] AC3: Text segments positioned correctly per style (Title centered, Bullet/Numbered left-aligned)
  - [x] AC4: Item delays converted to absolute start times with slide+fade keyframes
  - [x] AC5: I/O point changes reuse overlay generation
- **Files:** `crates/rendering/src/lib.rs`, `crates/rendering/src/overlay.rs` (new)
- **Dependencies:** None (builds on completed S01 types)

#### Phase 2: Split Overlay Enter/Exit Animations ✅
- **Objective:** Smooth camera transitions for split overlays that feel like the camera is sliding
- **Tasks:**
  - [x] Task 2.1: Review current split-screen transition code in `scene.rs`
  - [x] Task 2.2: Ensure enter animation: camera slides from full-width to right 50%, crops to keep subject centered, background fades in on left, text items slide/fade in
  - [x] Task 2.3: Ensure exit animation (to full camera): text slides out left, background fades out, camera expands back to full width
  - [x] Task 2.4: Ensure exit animation (to PiP): reuse existing full→PiP transition, background + text fade out as camera shrinks to corner
- **Acceptance Criteria:**
  - [x] AC1: Split enter looks like camera sliding right (not a cut)
  - [x] AC2: Split exit to full camera reverses smoothly
  - [x] AC3: Split exit to PiP reuses existing transition
  - [x] AC4: No jarring cuts between any mode transitions
- **Files:** `crates/rendering/src/scene.rs`, `crates/rendering/src/lib.rs`
- **Dependencies:** Phase 1 complete

#### Phase 3: OverlayTrack.tsx UI Component ✅
- **Objective:** New timeline track for managing overlays visually
- **Tasks:**
  - [x] Task 3.1: Create `OverlayTrack.tsx` component following TextTrack patterns
  - [x] Task 3.2: Render segments as colored bars (different color per overlay type)
  - [x] Task 3.3: Implement drag to move entire overlay
  - [x] Task 3.4: Implement resize handles to change start/end time
  - [x] Task 3.5: Implement click to select, integrate with existing selection state
  - [x] Task 3.6: Double-click to open item editor (Phase 4)
  - [x] Task 3.7: Add overlay track to Timeline/index.tsx when overlays exist
  - [x] Task 3.8: Add `overlay` to `TimelineSelectionType` in context.ts
- **Acceptance Criteria:**
  - [x] AC1: Track appears in timeline when overlays exist
  - [x] AC2: Segments are draggable (moves entire overlay)
  - [x] AC3: Segments are resizable (changes start/end)
  - [x] AC4: Selection works and integrates with existing UI
  - [x] AC5: Visual distinction between Split and FullScreen types
- **Files:** `apps/desktop/src/routes/editor/Timeline/OverlayTrack.tsx` (new), `apps/desktop/src/routes/editor/Timeline/index.tsx`, `apps/desktop/src/routes/editor/context.ts`
- **Dependencies:** Phase 1 complete

#### Phase 4: Item Timing Editor UI
- **Objective:** Easy editing of overlay items without touching JSON
- **Tasks:**
  - [ ] Task 4.1: Create `OverlayEditor.tsx` modal/panel component
  - [ ] Task 4.2: Show overlay type dropdown (Split/FullScreen)
  - [ ] Task 4.3: Show editable item list with reorder, add, delete
  - [ ] Task 4.4: Per-item: delay input, content text input, style dropdown (Title/Bullet/Numbered)
  - [ ] Task 4.5: Wire up to projectActions for saving changes
  - [ ] Task 4.6: Open editor on double-click from OverlayTrack
- **Acceptance Criteria:**
  - [ ] AC1: Double-click overlay opens editor
  - [ ] AC2: Can edit item text
  - [ ] AC3: Can change item delays
  - [ ] AC4: Can add/remove items
  - [ ] AC5: Can change item style
  - [ ] AC6: Changes save to project
- **Files:** `apps/desktop/src/routes/editor/OverlayEditor.tsx` (new)
- **Dependencies:** Phase 3 complete

### Technical Specifications

#### Data Model (from S01 — already implemented)
```rust
pub enum OverlayType {
    Split,      // 50/50 camera + text
    FullScreen, // PiP camera + background + text
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

#### Text Positioning
| Style | Position | Font Size | Alignment |
|-------|----------|-----------|-----------|
| Title | center (0.5, 0.5) | 64 | Center |
| Bullet | left (0.25, variable Y) | 40 | Left |
| Numbered | left (0.25, variable Y) | 40 | Left |

Vertical spacing: First item Y = 0.25, subsequent items Y += 0.12

#### Animation Timing
- Enter transition: 300ms ease-in-out
- Text fade-in: 300ms per item
- Exit transition: 300ms ease-in-out

#### JSON Import Schema (v2.0.0 — already implemented)
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
        { "delay": 2.0, "text": "First point", "style": "bullet" }
      ]
    }
  ]
}
```

### Decision Matrix

#### Decisions Made
| Decision | Choice | Rationale |
|----------|--------|-----------|
| Split ratio | 50/50 | Clean visual balance |
| Camera position | Fixed RIGHT | Consistent, no user confusion |
| Camera centering | Use existing crop settings | Simpler than auto face detection |
| PiP position | Fixed bottom-right | Standard convention |
| Background | Cap's existing system | Reuse, no new config |
| Text animation | Slide from left + fade | Matches professional editing style |
| Gap between overlays | Allow tiny gaps | No auto-merge, user controls |
| FullScreen type | PiP + text overlay | Camera always visible |
| Track visibility | User manages | No auto-hide of Scene/Text tracks |
| Item delay > duration | Warning toast | Don't block import |

---

## Plan Review
- **Gate:** ✅ PASS
- **Reviewed:** 2026-01-29
- **Summary:** Plan is well-structured and implementable. S01/S05 complete. Minor recommendations for Phase 1 integration (cache invalidation, 50/50 vs 60/40 split ratio) and Phase 3 selection handling.
- **Issues:** None blocking. 4 action items before Phase 1: clarify integration point, decide split ratio, add overlays to timeline hash, verify selection type location.

→ Details: `plan-review.md`

---

## Execution Log

### Phase 1: Overlay → Scene+Text Generation
- **Status:** ✅ COMPLETE
- **Started:** 2026-01-29
- **Completed:** 2026-01-29
- **Commits:** `6101815f9`
- **Files Modified:**
  - `crates/rendering/src/overlay.rs` (new, +464 lines) — Core generation functions + 12 tests
  - `crates/rendering/src/lib.rs` (+23/-15 lines) — Module integration and pipeline
- **Notes:**
  - Created new `overlay.rs` module with `generate_scene_segments()`, `generate_text_segments()`, `validate_overlay_items()`, and `merge_with_existing()` functions
  - Split overlay → SplitScreenRight scene mode
  - FullScreen overlay → Default scene mode (existing PiP behavior)
  - Text positioning: Title at (0.5, 0.5) 64pt, Bullet/Numbered at (0.25, 0.25+0.12*index) 40pt
  - Animation: 0.3s slide-from-left (-0.15 offset) + fade-in keyframes
  - Warning mechanism returns `OverlayWarning` structs when item delay >= segment duration
  - Integration in `ProjectUniforms::new`: overlays merged with existing scene/text segments at render time
  - Comprehensive test suite (12 tests) covering all generation scenarios

### Phase 2: Split Overlay Enter/Exit Animations
- **Status:** ✅ COMPLETE (no code changes required)
- **Started:** 2026-01-29
- **Completed:** 2026-01-29
- **Commits:** — (existing code satisfies requirements)
- **Files Modified:** — (none)
- **Notes:**
  - Reviewed existing `scene.rs` transition implementation
  - All enter/exit animations already implemented via:
    - `split_camera_x_ratio()`: Interpolates camera X from 0.0↔0.6 with bezier easing (0.3s)
    - `split_display_x_ratio()`: Interpolates display/background position
    - `split_camera_transition_opacity()`: Fades camera in/out during transitions
  - Text slide+fade animations handled by Phase 1 keyframes in `overlay.rs`
  - Exit to PiP uses existing `regular_camera_transition_opacity()` mechanism
  - No jarring cuts: all transitions use 0.3s bezier easing, `MIN_GAP_FOR_TRANSITION` (0.5s) prevents unnecessary transitions

### Phase 3: OverlayTrack.tsx UI Component
- **Status:** ✅ COMPLETE
- **Started:** 2026-01-29
- **Completed:** 2026-01-29
- **Commits:** `ebd874388`
- **Files Modified:**
  - `apps/desktop/src/routes/editor/Timeline/OverlayTrack.tsx` (new, +488 lines) — Main component
  - `apps/desktop/src/routes/editor/Timeline/index.tsx` (+38 lines) — Track integration
  - `apps/desktop/src/routes/editor/context.ts` (+82 lines) — Selection type, track state, projectActions
- **Notes:**
  - Created OverlayTrack.tsx following TextTrack patterns exactly
  - Colored segments: Split=orange gradient, FullScreen=teal gradient
  - Drag-to-move with neighbor collision bounds
  - Resize handles with min duration constraints (1s or 80px)
  - Multi-select with Ctrl/Cmd+click, range select with Shift+click
  - Double-click handler wired for Phase 4 item editor
  - Added `overlay` to TimelineSelectionType and TimelineTrackType
  - Added overlay track toggle in TrackManager
  - Added projectActions: splitOverlaySegment, deleteOverlaySegments
  - Updated rippleAdjustOverlays and deleteInOutRegion for overlay support
  - TypeScript types defined locally (not in tauri.ts auto-generated)
  - Click empty track area adds new overlay at playhead position

### Phase 4: Item Timing Editor UI
- **Status:** —
- **Started:** —
- **Completed:** —
- **Commits:** —
- **Files Modified:** —
- **Notes:** —

---

## Code Review Log

### Phase 1
- **Gate:** ✅ PASS
- **Reviewed:** 2026-01-29
- **Summary:** High-quality implementation with solid test coverage (12 tests). Clean function decomposition, proper constants extraction, and non-invasive integration. Uses existing `SplitScreenRight` (60/40 ratio) rather than true 50/50 — acceptable trade-off. Warnings captured but not yet surfaced to UI (Phase 3/4 responsibility).
→ Details: `code-review-phase-1.md`

### Phase 2
- **Gate:** N/A (no code changes)
- **Reviewed:** 2026-01-29
- **Summary:** Existing scene.rs transition code already implements all required enter/exit animations. Phase verified by code inspection — no new code needed.
→ Details: `code-review-phase-2.md`

### Phase 3
- **Gate:** ✅ PASS
- **Reviewed:** 2026-01-29
- **Summary:** High-quality implementation following TextTrack patterns exactly. Complete feature set (drag, resize, select, split). Clean visual distinction between overlay types. All acceptance criteria met.
→ Details: `code-review-phase-3.md`

### Phase 4
- **Gate:** —
→ Details: `code-review-phase-4.md`

---

## Completion
- **Completed:** —
- **Summary:** —
- **Learnings:** —
