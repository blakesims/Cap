# T006 Plan Review

**Reviewer:** Claude Code
**Date:** 2026-01-29 (Updated)
**Previous Review:** 2026-01-28
**Gate:** ✅ PASS (with recommendations)

---

## Executive Summary

The T006 plan is **well-structured and implementable**. The data model and import infrastructure (S01, S05) are already complete and solid. The phased approach is logical with clear dependencies. Minor improvements recommended for Phase 1 integration and Phase 3 selection handling.

---

## Prior Work Status (Verified)

| Completed | Status | Notes |
|-----------|--------|-------|
| S01 - OverlaySegment type | ✅ | `configuration.rs:817-854` complete, serializes correctly |
| S05 - v2.0.0 import | ✅ | `timeline_import.rs` with 15 overlay-specific tests |

---

## Phase-by-Phase Analysis

### Phase 1: Overlay → Scene+Text Generation

**Assessment:** ✅ Ready to implement

**Strengths:**
- Clear separation: overlays stay stored as `OverlaySegment`, rendered via scene+text generation
- Reuses existing rendering pipeline — no changes to `scene.rs` internals needed
- Toast warning for delay > duration is a good UX touch

**Concerns & Recommendations:**

1. **Integration Point:** Plan says "integrate into rendering pipeline" but doesn't specify where. Rendering flows through `RecordingSegmentDecoders::get_frames()` in `crates/rendering/src/lib.rs`.

   **Recommendation:** Create a `TimelineResolver` that merges `overlay_segments` into existing `scene_segments` and `text_segments` before rendering. This keeps the renderer unchanged.

2. **Cache Invalidation:** Per CLAUDE.md, audio pre-renders entire timeline using hash-based cache. Overlay changes must invalidate this cache.

   **Recommendation:** Add `overlay_segments` to `compute_timeline_hash()`.

3. **Split 50/50 vs 60/40:** Plan says "50/50 split" but existing `SplitScreenRight` uses 60/40 ratio (`scene.rs:339` — camera at x=0.6).

   **Recommendation:** Either accept existing 60/40 ratio or add new `SceneMode::SplitScreen5050Right`.

4. **Text Positioning:** Y-spacing (first Y=0.25, subsequent Y+=0.12) and font sizes (Title=64, Bullet/Numbered=40) need integration with existing `TextSegment.font_size` field.

**Files:** `crates/rendering/src/overlay.rs` (new), `crates/rendering/src/lib.rs`

---

### Phase 2: Split Overlay Enter/Exit Animations

**Assessment:** ✅ Mostly covered by existing code

**Analysis:**
- Existing `scene.rs` already handles split-screen transitions with 300ms ease-in-out
- `split_camera_transition_opacity()` and `split_camera_x_ratio()` interpolate properly
- Current transitions may already satisfy "sliding" requirement

**Concerns:**

1. **Background Fade:** Current split transitions don't explicitly fade background. Plan says "background fades in on left."

2. **Text Exit:** Phase 2 mentions "text slides out left" but Phase 1 handles text keyframes. Clarify responsibility.

**Recommendation:** Phase 2 may be minimal. Plan visual verification step before implementing additional animation code.

**Files:** `crates/rendering/src/scene.rs`, `crates/rendering/src/lib.rs`

---

### Phase 3: OverlayTrack.tsx UI Component

**Assessment:** ✅ Ready to implement

**Strengths:**
- Clear pattern from `TextTrack.tsx`
- Existing `Track.tsx` provides reusable components (`SegmentRoot`, `SegmentHandle`, `SegmentContent`)
- Selection types already include: `clip`, `zoom`, `mask`, `text`, `scene`

**Concerns:**

1. **Selection Type Location:** Task 3.8 mentions `context.ts` but selection is typed inline:
   ```ts
   selection: null | { type: "clip" | "zoom" | "mask" | "text" | "scene"; indices: number[] }
   ```
   Verify exact location (may be in generated `tauri.ts` or inline in `editorState`).

2. **projectActions:** Need methods for overlay operations:
   - `updateOverlaySegment(index, updates)`
   - `deleteOverlaySegments(indices)`

**Files:** `apps/desktop/src/routes/editor/Timeline/OverlayTrack.tsx` (new), `Timeline/index.tsx`, `context.ts`

---

### Phase 4: Item Timing Editor UI

**Assessment:** ✅ Ready to implement

**Strengths:**
- Self-contained modal/panel
- Standard list management (reorder, add, delete)

**Concerns:**

1. **Real-time Preview:** Should delay changes trigger preview re-render?

2. **Undo/Redo:** Use `projectHistory.pause()` pattern during editing.

**Files:** `apps/desktop/src/routes/editor/OverlayEditor.tsx` (new)

---

## Data Model Verification (S01)

Implementation in `configuration.rs:817-854` is correct:

```rust
pub enum OverlayType { Split, FullScreen }
pub enum OverlayItemStyle { Title, Bullet, Numbered }
pub struct OverlayItem { delay: f64, content: String, style: OverlayItemStyle }
pub struct OverlaySegment { start: f64, end: f64, overlay_type: OverlayType, items: Vec<OverlayItem> }
```

All fields have `#[serde(default)]`. `TimelineConfiguration.overlay_segments` exists at line 868. ✅

---

## Import Verification (S05)

`timeline_import.rs` implementation complete:
- v2.0.0 version support (line 203)
- Full overlay validation (lines 300-345)
- Proper error types for overlay issues
- Warning for delay > duration (non-blocking)
- 15 overlay-specific tests ✅

---

## Technical Specification Gaps

| Gap | Impact | Recommendation |
|-----|--------|----------------|
| Exit animation keyframes not specified | Medium | Add slide-out-left + fade-out spec |
| FullScreen PiP exact position | Low | Specify x,y coordinates or "use existing" |
| FullScreen background handling | Low | Clarify: Cap's background system or dark overlay? |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Split 50/50 vs 60/40 mismatch | Medium | Low | Document decision or use existing ratio |
| Cache invalidation missed | Medium | Medium | Add overlays to hash computation |
| Text animation timing edge cases | Low | Medium | Test overlapping overlay/text scenarios |
| Selection type update location | Low | Low | Search codebase for type definition |

---

## Recommended Improvements

### For Phase 1:
- Add AC: "Overlay generation is idempotent (no duplicate scene/text)"
- Specify cache invalidation approach

### For Phase 3:
- Add AC: "Deleting overlay via track removes from project configuration"
- Verify selection type definition location before starting

---

## Conclusion

**Gate: ✅ PASS**

The plan is ready for implementation. S01 and S05 are complete. Proceed to Phase 1.

**Action Items Before Phase 1:**
1. Clarify integration point for overlay generation in rendering pipeline
2. Decide on 50/50 vs existing 60/40 split ratio
3. Add overlay_segments to timeline hash computation
4. Verify selection type definition location

No blocking issues.
