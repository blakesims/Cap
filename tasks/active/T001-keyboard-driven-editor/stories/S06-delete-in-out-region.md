# S06: Delete IN/OUT Region (Phased Approach)

## Status
✅ **IMPLEMENTED** (2026-01-14) - All 3 phases complete

## Summary
Implement deletion of clip segments using keyboard shortcuts (`x` or `Backspace`). This is split into three phases to minimize risk: basic segment deletion, IN/OUT region deletion, and ripple delete. A separate PR will handle the sliver fix.

## Technical Context

### Existing Infrastructure (from S01-S05)

**State (context.ts, lines 640-648)**:
```typescript
editorState: {
  inPoint: null as number | null,
  outPoint: null as number | null,
  mark: null as number | null,
  playbackTime: 0,
  // ... other state
}
```

**Existing Actions**:
- `projectActions.splitClipSegment(time: number)` - Lines 211-243
- `projectActions.deleteClipSegment(segmentIndex: number)` - Lines 245-267
- `projectActions.setClipSegmentTimescale(index, timescale)` - Lines 420-465
- `projectActions.deleteZoomSegments(indices)` - Lines 290-306
- `projectActions.deleteMaskSegments(indices)` - Lines 329-345
- `projectActions.deleteTextSegments(indices)` - Lines 368-384

**Existing Keyboard Handler (Player.tsx, lines 272-278)**:
```typescript
{
  combo: "Backspace",
  handler: handleDeleteSelection,
},
```

The `handleDeleteSelection` function (lines 198-217) currently only handles track selection deletion (zoom, mask, text, clip, scene segments).

### Key Constraint: deleteClipSegment

Lines 245-267 in context.ts:
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

**Critical Constraint**: Cannot delete a segment if it's the only segment from its recordingSegment. This prevents deleting the entire recording. We must work within this constraint for Phases 1 and 2.

### Time-to-Segment-Index Conversion Pattern

From `splitClipSegment` (lines 216-228):
```typescript
let searchTime = time;
let _prevDuration = 0;
const currentSegmentIndex = segments.findIndex((segment) => {
  const duration = (segment.end - segment.start) / segment.timescale;
  if (searchTime > duration) {
    searchTime -= duration;
    _prevDuration += duration;
    return false;
  }
  return true;
});
```

This pattern finds which segment contains a given timeline time by iterating through segments and subtracting durations. We'll reuse this pattern.

## Phase 1: Basic Segment Deletion (~3h)

### Objective
Enable `x` or `Backspace` to delete the clip segment under the playhead when no IN/OUT points are set.

### Implementation Checklist

#### 1.1 Add Helper Function to Find Segment at Time
**File**: `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/context.ts`
**Location**: Inside `projectActions` object, before `splitClipSegment` (around line 210)

```typescript
const findSegmentIndexAtTime = (time: number): number => {
  if (!project.timeline) return -1;
  const segments = project.timeline.segments;
  let searchTime = time;
  const index = segments.findIndex((segment) => {
    const duration = (segment.end - segment.start) / segment.timescale;
    if (searchTime > duration) {
      searchTime -= duration;
      return false;
    }
    return true;
  });
  return index;
};
```

This mirrors the pattern from `splitClipSegment` but returns only the index.

#### 1.2 Create Delete Action in editorActions
**File**: Same as above
**Location**: Inside `editorActions` object (after existing actions, around line 894)

```typescript
deleteSegmentAtPlayhead: () => {
  const time = editorState.previewTime ?? editorState.playbackTime;
  const segmentIndex = findSegmentIndexAtTime(time);
  if (segmentIndex === -1) return;
  projectActions.deleteClipSegment(segmentIndex);
},
```

#### 1.3 Add Keyboard Binding
**File**: `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/Player.tsx`
**Location**: Modify existing handler (lines 272-278)

**Current Code**:
```typescript
{
  combo: "Backspace",
  handler: handleDeleteSelection,
},
```

**New Code**:
```typescript
{
  combo: "Backspace",
  handler: () => {
    if (editorState.timeline.selection !== null) {
      handleDeleteSelection();
    } else if (editorState.inPoint === null && editorState.outPoint === null) {
      editorActions.deleteSegmentAtPlayhead();
    }
  },
},
```

Also add binding for `x`:
```typescript
{
  combo: "X",
  handler: () => {
    if (editorState.timeline.selection !== null) {
      handleDeleteSelection();
    } else if (editorState.inPoint === null && editorState.outPoint === null) {
      editorActions.deleteSegmentAtPlayhead();
    }
  },
},
```

**Decision Rationale**:
- Check selection first (highest priority)
- Then check IN/OUT deletion (Phase 2)
- Finally fall back to segment-at-playhead deletion
- This creates a clear precedence hierarchy

### Testing Scenarios (Phase 1)

1. **Basic deletion**:
   - Load a recording with 3+ segments
   - Move playhead to middle segment
   - Press `x` or `Backspace`
   - Verify: Segment deleted, gap remains (no ripple yet)

2. **Constraint respected**:
   - Create a single segment from one recordingSegment
   - Try to delete it
   - Verify: Deletion blocked (constraint enforced)

3. **Selection takes precedence**:
   - Select a zoom segment
   - Press `Backspace`
   - Verify: Zoom segment deleted (not clip segment)

4. **Edge cases**:
   - Playhead at time 0: First segment deleted
   - Playhead at end: Last segment deleted
   - Empty timeline: Nothing happens

### Phase 1 Definition of Done
- [ ] `x` deletes segment under playhead when no selection/IN/OUT
- [ ] `Backspace` deletes segment under playhead when no selection/IN/OUT
- [ ] Existing constraint (2+ segments from same source) respected
- [ ] Gap left behind (no ripple)
- [ ] Selection deletion still works (precedence)
- [ ] No crashes on edge cases

## Phase 2: IN/OUT Region Deletion (~3h)

### Objective
When IN and OUT points are both set, `x`/`Backspace` deletes the content within the IN/OUT range. This may require splitting segments at the boundaries.

### Implementation Strategy

The IN/OUT region may span multiple segments or part of one segment. We need to:
1. Find all segments that overlap the [inPoint, outPoint] range
2. Split at IN boundary if needed
3. Split at OUT boundary if needed
4. Delete all segments fully contained within the range
5. Clear IN/OUT points after deletion

### Implementation Checklist

#### 2.1 Add Helper to Calculate Absolute Segment Times
**File**: `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/context.ts`
**Location**: Inside `projectActions`, after `findSegmentIndexAtTime`

```typescript
const getSegmentAbsoluteTimes = (): Array<{ start: number; end: number; index: number }> => {
  if (!project.timeline) return [];
  const segments = project.timeline.segments;
  let accumulated = 0;
  return segments.map((segment, index) => {
    const duration = (segment.end - segment.start) / segment.timescale;
    const start = accumulated;
    const end = accumulated + duration;
    accumulated = end;
    return { start, end, index };
  });
};
```

This creates a mapping of absolute timeline times for each segment, making it easy to find overlaps with the IN/OUT range.

#### 2.2 Add Delete IN/OUT Region Action
**File**: Same as above
**Location**: Inside `editorActions`, after `deleteSegmentAtPlayhead`

**IMPORTANT**: The original algorithm had critical bugs with index management after splits. This corrected version recalculates segment times AFTER each split to avoid index corruption.

```typescript
deleteInOutRegion: () => {
  const inP = editorState.inPoint;
  const outP = editorState.outPoint;
  if (inP === null || outP === null) return;

  const inTime = Math.min(inP, outP);
  const outTime = Math.max(inP, outP);

  batch(() => {
    const initialTimes = getSegmentAbsoluteTimes();
    if (initialTimes.length === 0) return;

    const wouldDeleteAll = initialTimes.every(
      (seg) => seg.start >= inTime && seg.end <= outTime
    );
    if (wouldDeleteAll) return;

    for (const seg of initialTimes) {
      if (seg.start < inTime && seg.end > inTime) {
        projectActions.splitClipSegment(inTime);
        break;
      }
    }

    const afterInSplit = getSegmentAbsoluteTimes();
    for (const seg of afterInSplit) {
      if (seg.start < outTime && seg.end > outTime) {
        projectActions.splitClipSegment(outTime);
        break;
      }
    }

    const afterBothSplits = getSegmentAbsoluteTimes();
    const toDelete: number[] = [];

    for (const seg of afterBothSplits) {
      if (seg.start >= inTime && seg.end <= outTime) {
        toDelete.push(seg.index);
      }
    }

    toDelete.sort((a, b) => b - a);
    for (const idx of toDelete) {
      projectActions.deleteClipSegment(idx);
    }

    editorActions.clearInOut();
  });
},
```

**Key Decisions**:
- Use `batch()` to ensure atomic updates
- **Recalculate segment times AFTER each split** - This is critical! The original approach built indices before splits and reused them after, causing corruption.
- Validation guard prevents deleting ALL segments (would violate constraints everywhere)
- Split at IN first, then recalculate, then split at OUT
- Find segments fully contained in [IN, OUT] AFTER both splits complete
- Delete segments in descending order to maintain index validity
- Clear IN/OUT points after deletion (user expectation)

#### 2.3 Update Keyboard Handler
**File**: `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/Player.tsx`
**Location**: Modify handlers from Phase 1

```typescript
{
  combo: "Backspace",
  handler: () => {
    if (editorState.timeline.selection !== null) {
      handleDeleteSelection();
    } else if (editorState.inPoint !== null && editorState.outPoint !== null) {
      editorActions.deleteInOutRegion();
    } else {
      editorActions.deleteSegmentAtPlayhead();
    }
  },
},
{
  combo: "X",
  handler: () => {
    if (editorState.timeline.selection !== null) {
      handleDeleteSelection();
    } else if (editorState.inPoint !== null && editorState.outPoint !== null) {
      editorActions.deleteInOutRegion();
    } else {
      editorActions.deleteSegmentAtPlayhead();
    }
  },
},
```

### Testing Scenarios (Phase 2)

1. **Single segment, partial deletion**:
   - One segment, 10 seconds long
   - Set IN at 3s, OUT at 7s
   - Press `x`
   - Verify: Segment split into [0-3s] and [7-10s], middle deleted

2. **Multiple segments, full deletion**:
   - Three segments: [0-5s], [5-10s], [10-15s]
   - Set IN at 0s, OUT at 15s
   - Press `x`
   - Verify: All segments deleted (if constraint allows)

3. **Multiple segments, partial deletion**:
   - Three segments: [0-5s], [5-10s], [10-15s]
   - Set IN at 3s, OUT at 12s
   - Verify:
     - First segment split: [0-3s] remains, [3-5s] deleted
     - Second segment: fully deleted
     - Third segment split: [10-12s] deleted, [12-15s] remains

4. **OUT before IN (order swap)**:
   - Set OUT at 3s, then IN at 7s
   - Press `x`
   - Verify: Range [3-7s] deleted (min/max handles order)

5. **IN/OUT on segment boundaries**:
   - Segments at [0-5s], [5-10s]
   - Set IN at 5s, OUT at 10s
   - Verify: Second segment deleted cleanly (no unnecessary splits)

6. **Constraint violation**:
   - Only one segment from a recordingSegment
   - Try to delete it via IN/OUT
   - Verify: Deletion blocked by existing constraint

### Phase 2 Definition of Done
- [ ] IN/OUT region deletion works for single segment
- [ ] IN/OUT region deletion works for multiple segments
- [ ] Auto-splits at IN boundary when needed
- [ ] Auto-splits at OUT boundary when needed
- [ ] Segments fully within range deleted
- [ ] IN/OUT points cleared after deletion
- [ ] Order-independent (OUT before IN works)
- [ ] Existing constraint still respected
- [ ] No ripple yet (gaps left behind)

## Phase 3: Ripple Delete (~3h)

### Objective
After deletion, shift subsequent segments earlier to close the gap. Also adjust zoom/mask/text segments to maintain their relative positions.

### Implementation Strategy

Ripple delete has two parts:
1. **Clip segment ripple**: Shift all segments after the deletion point
2. **Overlay ripple**: Adjust zoom/mask/text segments that start after the deletion point

### Implementation Checklist

#### 3.1 Calculate Gap Duration
After deletion, we need to know how much time was removed. Store this before deleting:

**File**: `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/context.ts`
**Location**: Modify `deleteInOutRegion` action

Add to beginning of `deleteInOutRegion`:
```typescript
const totalDurationBefore = project.timeline?.segments.reduce(
  (acc, s) => acc + (s.end - s.start) / s.timescale,
  0,
) ?? 0;
```

Add after all deletions (before `clearInOut()`):
```typescript
const totalDurationAfter = project.timeline?.segments.reduce(
  (acc, s) => acc + (s.end - s.start) / s.timescale,
  0,
) ?? 0;

const gapDuration = totalDurationBefore - totalDurationAfter;
const rippleStartTime = inTime;

editorActions.rippleAdjustOverlays(rippleStartTime, -gapDuration);
```

#### 3.2 Add Ripple Adjust Overlays Action
**File**: Same as above
**Location**: Inside `editorActions`, after `deleteInOutRegion`

```typescript
rippleAdjustOverlays: (startTime: number, timeDelta: number) => {
  if (timeDelta === 0) return;

  batch(() => {
    setProject(
      "timeline",
      "zoomSegments",
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
      })
    );

    setProject(
      "timeline",
      "maskSegments",
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
      })
    );

    setProject(
      "timeline",
      "textSegments",
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
      })
    );
  });
},
```

**Key Decisions**:
- Segments that start after deletion: shift entirely
- Segments that straddle deletion point: trim end time
- Use `Math.max(seg.start, ...)` to prevent invalid segments (end < start)
- Apply same logic to zoom, mask, and text segments

#### 3.3 Adjust Playhead Position
After ripple delete, the playhead may be in the deleted region. Move it to the IN point:

**File**: Same as above
**Location**: At end of `deleteInOutRegion`, after `clearInOut()`

```typescript
setEditorState("playbackTime", rippleStartTime);
setEditorState("previewTime", null);
```

#### 3.4 Update Phase 1 Delete (Basic Segment)
Phase 1 deletion should also ripple. Modify `deleteSegmentAtPlayhead`:

```typescript
deleteSegmentAtPlayhead: () => {
  const time = editorState.previewTime ?? editorState.playbackTime;
  const segmentIndex = findSegmentIndexAtTime(time);
  if (segmentIndex === -1) return;

  const absoluteTimes = getSegmentAbsoluteTimes();
  const seg = absoluteTimes[segmentIndex];
  const segmentDuration = seg.end - seg.start;
  const rippleStartTime = seg.start;

  batch(() => {
    projectActions.deleteClipSegment(segmentIndex);
    editorActions.rippleAdjustOverlays(rippleStartTime, -segmentDuration);
    setEditorState("playbackTime", rippleStartTime);
    setEditorState("previewTime", null);
  });
},
```

### Testing Scenarios (Phase 3)

1. **Basic ripple**:
   - Three segments: [0-5s], [5-10s], [10-15s]
   - Delete middle segment
   - Verify: Third segment now at [5-10s] (shifted 5s earlier)

2. **Overlay ripple - full shift**:
   - Segments: [0-5s], [5-10s], [10-15s]
   - Zoom segment: [11s-13s]
   - Delete middle segment (5-10s)
   - Verify: Zoom segment now at [6s-8s] (shifted 5s earlier)

3. **Overlay ripple - partial trim**:
   - Segments: [0-5s], [5-10s], [10-15s]
   - Mask segment: [8s-12s] (straddles deletion)
   - Delete middle segment (5-10s)
   - Verify: Mask segment becomes [3s-7s] (end trimmed by 5s)

4. **Playhead adjustment**:
   - Playhead at 7s
   - Delete segment containing 7s
   - Verify: Playhead moved to deletion start point

5. **IN/OUT region ripple**:
   - Segments: [0-5s], [5-10s], [10-15s], [15-20s]
   - Text segment: [16s-18s]
   - Delete IN=5s, OUT=15s (removes 2 segments)
   - Verify: Last segment at [5-10s], text at [6s-8s]

6. **Edge case - deletion at start**:
   - Delete first segment
   - Verify: All subsequent segments shift to start at 0

7. **Edge case - deletion at end**:
   - Delete last segment
   - Verify: No ripple needed, no other segments affected

### Phase 3 Definition of Done
- [ ] Clip segments ripple after deletion (gap closed)
- [ ] Zoom segments ripple correctly
- [ ] Mask segments ripple correctly
- [ ] Text segments ripple correctly
- [ ] Segments straddling deletion point are trimmed
- [ ] Playhead adjusts to deletion start
- [ ] Works for single segment deletion
- [ ] Works for IN/OUT region deletion
- [ ] Edge cases handled (start, end, straddling)

## Separate PR: Sliver Fix

### Objective
Prevent creation of tiny unusable segments ("slivers") when cutting or deleting.

### Implementation Checklist

#### 4.1 Add Frame Snapping Utility
**File**: `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/context.ts`
**Location**: Top-level, near FPS constant (line 64)

```typescript
const snapToFrame = (time: number): number => {
  const frameTime = 1 / 30;
  return Math.round(time / frameTime) * frameTime;
};

const MIN_SEGMENT_DURATION = 3 / 30;
```

**Decision**: Use 30 FPS (matches existing usage). Minimum segment = 3 frames (0.1s).

#### 4.2 Update Split Function
**File**: Same as above
**Location**: Modify `splitClipSegment` (line 211)

Add at beginning of function:
```typescript
time = snapToFrame(time);
```

Add before actual split (after finding segment):
```typescript
const duration = (segment.end - segment.start) / segment.timescale;
const leftDuration = searchTime;
const rightDuration = duration - searchTime;

if (leftDuration < MIN_SEGMENT_DURATION || rightDuration < MIN_SEGMENT_DURATION) {
  return;
}
```

#### 4.3 Update IN/OUT Region Deletion
**File**: Same as above
**Location**: Modify `deleteInOutRegion`

Add at beginning:
```typescript
const inTime = snapToFrame(Math.min(inP, outP));
const outTime = snapToFrame(Math.max(inP, outP));

if (outTime - inTime < MIN_SEGMENT_DURATION) {
  return;
}
```

### Testing Scenarios (Sliver Fix)

1. **Snap to frame**:
   - Move playhead to 5.14s (between frames)
   - Press `c` to cut
   - Verify: Cut happens at 5.133s or 5.167s (nearest frame)

2. **Reject small split**:
   - 5-second segment
   - Try to cut at 0.01s into it
   - Verify: Split rejected (would create sliver)

3. **Reject small IN/OUT**:
   - Set IN at 5.00s, OUT at 5.05s (1.5 frames)
   - Try to delete
   - Verify: Deletion rejected (region too small)

4. **Valid split**:
   - 5-second segment
   - Cut at 0.5s (15 frames)
   - Verify: Split accepted, both segments >= 3 frames

### Sliver Fix Definition of Done
- [ ] All time values snapped to frame boundaries (1/30s)
- [ ] Minimum segment duration enforced (3 frames / 0.1s)
- [ ] Small splits rejected silently
- [ ] Small IN/OUT deletions rejected silently
- [ ] Valid operations unaffected
- [ ] Implemented in separate PR (isolated risk)

## Risk Analysis & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Index corruption during deletion | High | Use `batch()`, delete in descending order |
| Overlay segments become invalid (end < start) | Medium | Use `Math.max(seg.start, ...)` in ripple |
| Constraint violation (last segment) | Medium | Rely on existing constraint check |
| Ripple causes timeline desync | Medium | Test with all overlay types |
| Performance with many segments | Low | Operations are O(n), acceptable for editor |
| Sliver fix breaks existing cuts | Low | Separate PR, easy to revert |

## Implementation Order

1. **Phase 1 First**: Basic deletion without ripple. Test thoroughly.
2. **Phase 2 Second**: IN/OUT deletion without ripple. Test edge cases.
3. **Phase 3 Third**: Add ripple to both. Test interaction between clip/overlay ripple.
4. **Sliver Fix Last**: Separate PR after Phases 1-3 stable.

This order minimizes risk - each phase builds on the previous, and the sliver fix is isolated.

## Color Palette (from S05)

For visual feedback (IN/OUT region), already implemented in S05:
- **IN point**: `rgb(107, 203, 119)` (green)
- **OUT point**: `rgb(226, 64, 64)` (red)
- **Region overlay**: `rgba(74, 158, 255, 0.15)` (semi-transparent blue)

## Files Modified

### Phase 1
- `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/context.ts` - Add helper and action
- `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/Player.tsx` - Update keyboard bindings

### Phase 2
- `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/context.ts` - Add IN/OUT deletion logic

### Phase 3
- `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/context.ts` - Add ripple logic

### Sliver Fix (Separate PR)
- `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/context.ts` - Add frame snapping

## Files NOT Modified

- No Rust changes needed
- No Timeline visual changes (overlays already in S05)
- No new state (uses existing IN/OUT from S01)

## Total Estimated Effort

- Phase 1: ~3 hours
- Phase 2: ~3 hours
- Phase 3: ~3 hours
- Sliver Fix: ~1 hour (separate PR)
- **Total: ~10 hours** (includes testing buffer)

## Notes from S01-S04 Learnings

**A. State Definition Order**:
- Add helpers inside `projectActions` (defined after state)
- Add new actions inside `editorActions` (defined after state)

**B. Batch Updates**:
- Use `batch()` for atomic multi-state updates
- Critical for deletion + ripple in one operation

**C. Segment Index Management**:
- Delete in descending order when deleting multiple
- Recalculate indices after splits (they shift)

**D. Existing Patterns to Follow**:
- `deleteZoomSegments` (lines 290-306) shows batch deletion pattern
- `splitClipSegment` (lines 211-243) shows time-to-index conversion
- `setClipSegmentTimescale` (lines 420-465) shows overlay adjustment pattern

## Definition of Done (All Phases)

- [ ] Phase 1: Basic segment deletion works
- [ ] Phase 2: IN/OUT region deletion works
- [ ] Phase 3: Ripple delete closes gaps
- [ ] All test scenarios pass
- [ ] No code comments added
- [ ] No regressions in existing functionality
- [ ] Existing constraint (2+ segments) still enforced
- [ ] Keyboard shortcuts work as expected
- [ ] Selection deletion precedence maintained
- [ ] Sliver fix in separate PR (optional, can be done later)
