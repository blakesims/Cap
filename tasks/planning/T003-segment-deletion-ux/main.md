# Task: [T003] - Segment Deletion UX Improvements

## 0. Task Summary
-  **Task Name:** Segment Deletion UX Improvements
-  **Priority:** 2
-  **Number of Stories:** 3
-  **Current Status:** PLANNING
-  **Platform:** macOS only
-  **Dependencies:** `apps/desktop/src/routes/editor/context.ts`, `apps/desktop/src/routes/editor/Player.tsx`, `apps/desktop/src/routes/editor/Timeline/ClipTrack.tsx`
-  **Rules Required:** CLAUDE.md (no comments, use existing patterns)
-  **Related Task:** T001-keyboard-driven-editor (extends keyboard editing work)
-  **Acceptance Criteria:**
    - Delete/Backspace/X all behave identically
    - Can delete any segment as long as 1+ segment remains (pause/resume segments deletable)
    - IN/OUT deletion removes overlays fully contained within the region
    - Visual feedback when deletion is blocked
    - Clear visual indicator when segment is selected
    - `?` key shows keyboard shortcuts help modal

## 1. Goal / Objective
Fix the broken segment deletion behavior in the Cap desktop editor. The core issue is a constraint that prevents deleting pause/resume recording segments. Users should be able to easily delete "bad takes" from a recording session without manual workarounds.

## 2. Overall Status
Research complete. Root cause identified. Plan reviewed by subagent. Ready for implementation.

---

## 3. Root Cause Analysis

### 3.1 The Broken Constraint

**Location:** `context.ts:280-287`

```typescript
if (
  !segment ||
  !segment.recordingSegment === undefined ||  // BUG: Always evaluates to false
  project.timeline.segments.filter(
    (s) => s.recordingSegment === segment.recordingSegment,
  ).length < 2  // PROBLEM: Requires 2+ segments from SAME recording chunk
)
  return;  // Silent failure
```

**Why it's broken:**

When recording with pause/resume, each resumed section gets a unique `recordingSegment` value:
- First recording chunk: `recordingSegment: 0`
- After pause/resume: `recordingSegment: 1`
- After another pause/resume: `recordingSegment: 2`

The constraint checks for 2+ segments from the **same** chunk. Since each pause/resume chunk starts as 1 segment, **none of them can be deleted** until manually split.

**The user's workflow:**
1. Record a session with multiple takes (pause/resume between takes)
2. Open editor to delete bad takes
3. Press delete → nothing happens (silent failure)
4. Frustration ensues

### 3.2 Secondary Issues

| Issue | Location | Impact |
|-------|----------|--------|
| Delete key only works on selection | `Player.tsx:302-303` | Mac users expect Delete to work like Backspace |
| No deletion feedback | `context.ts:287` | User sees nothing when deletion is blocked |
| Overlays not deleted with IN/OUT | `context.ts:952-1018` | Effects remain after clip content is deleted |
| Invisible selection state | `ClipTrack.tsx` | Users don't know when segments are selected |

### 3.3 Logical Bug Found

Line 282: `!segment.recordingSegment === undefined`

This evaluates as `(!segment.recordingSegment) === undefined` which is always `false` (a boolean is never `undefined`). Likely intended as `segment.recordingSegment === undefined`.

### 3.4 Review Findings (Critical Issues Found)

Plan was reviewed by subagent. The following critical issues were identified:

**1. Duplicate Constraint in ConfigSidebar.tsx**

The delete button in the segment settings panel (ConfigSidebar.tsx:3378-3384) has its own copy of the broken constraint:
```typescript
disabled={
  (project.timeline?.segments.filter(
    (s) => s.recordingSegment === props.segment.recordingSegment,
  ) ?? []).length < 2
}
```
This must be updated to match the new constraint or the button will remain incorrectly disabled.

**2. Missing `sceneSegments` in Overlay Handling**

The plan only handles zoomSegments, maskSegments, and textSegments. But `sceneSegments` (camera layouts, splits) also exists and is NOT handled by:
- The proposed overlay deletion in IN/OUT region
- The existing `rippleAdjustOverlays` function

This means scene segment timings will be wrong after deletion.

**3. Selection-Based Deletion Behavior Change**

Currently Backspace/X check selection FIRST before IN/OUT. The plan removes this, which would break the ability to select segments and press Backspace to delete them.

**Decision needed**: Keep selection check, or intentionally remove it?
- **Keep**: More flexible, but adds complexity
- **Remove**: Simpler mental model (IN/OUT or playhead only)

**Recommendation**: Remove selection from delete flow (as planned). Selection will remain for visual feedback only. Users who want to delete specific segments can use IN/OUT markers.

**4. Toast System Already Exists**

Good news: `solid-toast` is already installed and used throughout the app. The `Toaster` component is already mounted in `App.tsx`. Just import and use:
```typescript
import toast from "solid-toast";
toast("Cannot delete the only remaining segment");
```

**5. isSelected Memo Already Exists**

`ClipTrack.tsx:417-429` already has an `isSelected` memo. We just need to use it for styling, not create it.

---

## 4. Stories Breakdown

| Story ID | Story Name / Objective | Complexity | Est. Hours | Status |
| :--- | :--- | :--- | :--- | :--- |
| S01 | Core Deletion Logic Fix | Medium | ~3-4h | **Done** |
| S02 | User Feedback & Visual State | Medium | ~3-4h | **Done** |
| S03 | Keyboard Shortcuts Help | Low | ~2h | Planned |

---

## 5. Story Details

### S01 - Core Deletion Logic Fix

**Objective:** Fix all deletion-related bugs so segments can be deleted reliably.

#### Acceptance Criteria
- [x] Any segment can be deleted as long as 1+ segment remains after deletion
- [x] Delete, Backspace, and X keys all behave identically
- [x] Deletion priority: IN/OUT region → Segment at playhead
- [x] IN/OUT deletion removes overlay segments (zoom/mask/text/scene) fully contained within the region
- [x] Playhead moves to start of deleted region after deletion
- [x] IN/OUT points cleared after IN/OUT deletion
- [x] ConfigSidebar delete button uses new constraint

#### Technical Changes

**1. Fix the constraint (`context.ts:277-300`)**

Current:
```typescript
deleteClipSegment: (segmentIndex: number) => {
  if (!project.timeline) return;
  const segment = project.timeline.segments[segmentIndex];
  if (
    !segment ||
    !segment.recordingSegment === undefined ||
    project.timeline.segments.filter(
      (s) => s.recordingSegment === segment.recordingSegment,
    ).length < 2
  )
    return;
  // ... deletion logic
}
```

New:
```typescript
deleteClipSegment: (segmentIndex: number) => {
  if (!project.timeline) return;
  const segment = project.timeline.segments[segmentIndex];
  if (!segment) return;
  if (project.timeline.segments.length < 2) {
    // Return indicator that deletion was blocked (for S02 feedback)
    return { blocked: true, reason: "cannot_delete_last_segment" };
  }
  // ... deletion logic
  return { blocked: false };
}
```

**2. Unify Delete/Backspace/X (`Player.tsx:272-303`)**

Current:
```typescript
{ combo: "Backspace", handler: () => { /* full logic */ } },
{ combo: "X", handler: () => { /* duplicate full logic */ } },
{ combo: "Delete", handler: handleDeleteSelection },  // Different!
```

New:
```typescript
const handleDelete = () => {
  if (editorState.inPoint !== null && editorState.outPoint !== null) {
    editorActions.deleteInOutRegion();
  } else {
    editorActions.deleteSegmentAtPlayhead();
  }
};

{ combo: "Backspace", handler: handleDelete },
{ combo: "X", handler: handleDelete },
{ combo: "Delete", handler: handleDelete },
```

**Note:** Selection-based deletion is removed from this flow. Selection remains for:
- Visual indication of clicked segment
- Potential future batch operations
- Overlay track manipulation via right-click menus

**3. Add overlay deletion to IN/OUT region (`context.ts:952-1018`)**

Add before `rippleAdjustOverlays()` call:

```typescript
// Delete overlay segments fully contained within [inTime, outTime]
setProject("timeline", "zoomSegments", (segments) =>
  segments.filter((s) => !(s.start >= inTime && s.end <= outTime))
);
setProject("timeline", "maskSegments", (segments) =>
  segments.filter((s) => !(s.start >= inTime && s.end <= outTime))
);
setProject("timeline", "textSegments", (segments) =>
  segments.filter((s) => !(s.start >= inTime && s.end <= outTime))
);
setProject("timeline", "sceneSegments", (segments) =>
  segments?.filter((s) => !(s.start >= inTime && s.end <= outTime))
);
```

**4. Update `rippleAdjustOverlays` to include `sceneSegments` (`context.ts:1020-1072`)**

Add after textSegments handling:

```typescript
setProject(
  "timeline",
  "sceneSegments",
  produce((segments) => {
    if (!segments) return;
    for (const seg of segments) {
      if (seg.start >= startTime) {
        seg.start += timeDelta;
        seg.end += timeDelta;
      } else if (seg.end > startTime) {
        seg.end = Math.max(seg.start, seg.end + timeDelta);
      }
    }
  }),
);
```

**5. Fix duplicate constraint in ConfigSidebar.tsx (line 3378-3384)**

Current:
```typescript
disabled={
  (project.timeline?.segments.filter(
    (s) => s.recordingSegment === props.segment.recordingSegment,
  ) ?? []).length < 2
}
```

New:
```typescript
disabled={(project.timeline?.segments.length ?? 0) < 2}
```

#### Test Scenarios

| Scenario | Expected Result |
|----------|-----------------|
| Delete segment from single-chunk recording (no pause/resume) | Blocked if only 1 segment, works if 2+ |
| Delete segment from multi-chunk recording (pause/resume) | Works - can delete any chunk |
| Delete last remaining segment | Blocked with feedback (toast) |
| IN/OUT spanning single segment | Deletes that segment |
| IN/OUT spanning multiple segments | Deletes all fully contained segments |
| IN/OUT with overlay inside | Overlay deleted too (zoom/mask/text/scene) |
| IN/OUT with overlay partially overlapping | Overlay shrunk, not deleted |
| Press Delete key (no selection, no IN/OUT) | Deletes segment at playhead |
| Press Backspace key | Same as Delete |
| Press X key | Same as Delete |
| ConfigSidebar delete button on pause/resume segment | Button enabled, deletion works |
| Scene segment timing after deletion | Scene segments shifted correctly |

---

### S02 - User Feedback & Visual State

**Objective:** Give users clear feedback when actions are blocked and clear indication of selection state.

#### Acceptance Criteria
- [x] Toast notification appears when deletion is blocked
- [x] Toast explains why deletion was blocked
- [x] Toast auto-dismisses (solid-toast default)
- [x] Selected segments have visible border/highlight (blue outline)
- [x] Selection is visually distinct from hover state

#### Technical Changes

**1. Use existing toast system**

`solid-toast` is already installed and configured. The `Toaster` is mounted in `App.tsx`. Simply import and use:

```typescript
import toast from "solid-toast";
toast.error("Cannot delete the only remaining segment");
```

**2. Return feedback from deletion functions**

Modify `deleteClipSegment` to return status (done in S01).

Modify `deleteSegmentAtPlayhead` to handle response:
```typescript
deleteSegmentAtPlayhead: () => {
  const time = editorState.previewTime ?? editorState.playbackTime;
  const segmentIndex = findSegmentIndexAtTime(time);
  if (segmentIndex === -1) {
    showToast("No segment at playhead position");
    return;
  }

  const result = projectActions.deleteClipSegment(segmentIndex);
  if (result?.blocked) {
    if (result.reason === "cannot_delete_last_segment") {
      showToast("Cannot delete the only remaining segment");
    }
    return;
  }
  // ... rest of deletion logic
};
```

**3. Add selection visual indicator (`ClipTrack.tsx`)**

The `isSelected` memo already exists at lines 417-429. Find segment rendering and add conditional styling using the existing memo:

```typescript
// In segment div styling (around line 540):
style={{
  ...existingStyles,
  outline: isSelected() ? "2px solid #3b82f6" : "none",
  "outline-offset": "-2px",
}}
```

#### Visual Design

**Toast styling:**
- Position: Bottom center of editor
- Background: Dark gray (#1f2937)
- Text: White
- Icon: Warning/info icon
- Duration: 3 seconds
- Animation: Fade in/out

**Selection styling:**
- Border: 2px solid blue (#3b82f6)
- Inset so it doesn't change segment size
- Should be visible on both light and dark waveforms

---

### S03 - Keyboard Shortcuts Help

**Objective:** Add `?` key to show a modal with all keyboard shortcuts.

#### Acceptance Criteria
- [ ] `?` (Shift+/) opens shortcuts modal
- [ ] Modal shows all editor shortcuts grouped by function
- [ ] Escape or click outside dismisses modal
- [ ] Modal is styled consistently with rest of editor

#### Technical Changes

**1. Add keyboard binding (`Player.tsx`)**

```typescript
{
  combo: "Shift+/",  // ? key
  handler: () => setShowShortcutsModal(true),
},
```

**2. Create modal component**

New file: `apps/desktop/src/routes/editor/KeyboardShortcutsModal.tsx`

Structure:
```typescript
export function KeyboardShortcutsModal(props: {
  open: boolean;
  onClose: () => void;
}) {
  return (
    <Show when={props.open}>
      <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
        <div class="bg-gray-900 rounded-lg p-6 max-w-2xl max-h-[80vh] overflow-auto">
          <h2>Keyboard Shortcuts</h2>
          {/* Grouped shortcuts */}
        </div>
      </div>
    </Show>
  );
}
```

**3. Shortcuts content**

Group by function:
- **Playback**: Space (play/pause), K (pause), Ctrl+L/J (speed)
- **Navigation**: H/L (frame step), Shift+H/L (second step), W/B (boundaries), 0/$ (start/end)
- **Editing**: C (split), Delete/Backspace/X (delete), I/O (markers), M (mark), Escape (clear)
- **View**: +/- (zoom), ? (this help)

---

## 6. Implementation Order

1. **S01 first** - This is the root cause. Without this fix, deletion doesn't work.
2. **S02 second** - User feedback depends on S01's return values.
3. **S03 last** - Independent, can be done anytime.

---

## 7. Files Modified

| File | Story | Changes |
|------|-------|---------|
| `context.ts` | S01, S02 | Fix constraint, add overlay deletion, add sceneSegments to ripple, return feedback |
| `Player.tsx` | S01, S03 | Unify delete handlers, add ? binding, integrate modal |
| `ConfigSidebar.tsx` | S01 | Fix duplicate constraint at line 3378-3384 |
| `ClipTrack.tsx` | S02 | Add selection styling using existing `isSelected` memo |
| `KeyboardShortcutsModal.tsx` (new) | S03 | Create modal component |

---

## 8. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Changing constraint breaks something else | High | All callers identified: Player.tsx, ConfigSidebar.tsx (2 places), context.ts. All updated in plan. |
| ConfigSidebar duplicate constraint missed | High | Identified in review. Must update line 3378-3384. |
| sceneSegments timing wrong after deletion | Medium | Add sceneSegments to rippleAdjustOverlays and overlay deletion. |
| Selection styling conflicts with existing hover | Low | Use distinct colors (blue vs gray), test visually |
| Overlay deletion too aggressive | Medium | Only delete if FULLY contained, not partial overlap |
| Selection-based delete removed | Low | Intentional simplification. Users use IN/OUT markers instead. |

---

## 9. Out of Scope

- Keyboard-based segment selection (using Enter or arrow keys)
- Undo/redo for deletions
- Batch delete operations
- Changes to Rust/backend code
- Reverse playback

---

## 10. Testing Checklist

Before marking complete:

**Core Deletion (S01):**
- [ ] Can delete any segment from pause/resume recording
- [ ] Cannot delete last remaining segment (shows toast)
- [ ] Delete/Backspace/X all work identically
- [ ] IN/OUT deletion works across multiple segments
- [ ] Overlays (zoom/mask/text/scene) inside IN/OUT region are deleted
- [ ] Overlays partially overlapping are shrunk, not deleted
- [ ] Scene segments adjust correctly after deletion (ripple)
- [ ] ConfigSidebar delete button enabled for pause/resume segments

**User Feedback (S02):**
- [ ] Selected segments show blue border
- [ ] Toast appears for blocked deletions
- [ ] Toast auto-dismisses after ~3 seconds

**Keyboard Help (S03):**
- [ ] ? key opens shortcuts modal
- [ ] Escape closes modal
- [ ] All shortcuts listed and accurate

**Regression:**
- [ ] Split (C key) still works
- [ ] Playback still works
- [ ] IN/OUT markers still work
- [ ] Existing mouse selection still works
