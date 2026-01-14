# S03 - Playhead Navigation

## Overview
- **Complexity:** Low-Medium
- **Estimated Time:** ~3-4 hours
- **Lines Added:** ~150
- **Lines Modified:** ~15
- **Lines Removed:** 0
- **Net Change:** ~165 lines
- **Files Changed:** 2 (`context.ts`, `Player.tsx`)

## Goal
Implement frame-stepping, second-stepping, and boundary jumping navigation for the playhead using keyboard shortcuts. Enable quick, precise playhead positioning without mouse interaction.

## Current Architecture

### Playback Time Management

**State location:** `editorState.playbackTime` in `context.ts`

The playback time is managed in two modes:
1. **Playback mode:** Rust backend controls time via `commands.startPlayback()`
2. **Seek mode:** Frontend directly updates `editorState.playbackTime`

**Key observations:**
- FPS is 60 (defined as `export const FPS = 60` in `context.ts`)
- Task requirements specify 1/30th second per frame (30 FPS navigation)
- Total duration calculated via `totalDuration()`: sums all segment durations
- Valid time range: `[0, totalDuration()]`

### Segment Structure

**Location:** `project.timeline.segments` (type `TimelineSegment[]`)

```typescript
type TimelineSegment = {
	recordingSegment?: number;
	timescale: number;
	start: number;
	end: number;
}
```

**Segment duration formula:**
```typescript
const duration = (segment.end - segment.start) / segment.timescale;
```

**Segment boundaries:** Start/end times of each segment in timeline coordinates

---

## Implementation Strategy

**Approach:** Add navigation actions to `editorActions` object and register keyboard bindings in Player.tsx

---

## Step 1: Add Navigation Actions to `editorActions`

**File:** `apps/desktop/src/routes/editor/context.ts`

**Add these functions to the `editorActions` object (after `clearAll`):**

```typescript
stepFrames: (count: number) => {
	const frameTime = 1 / 30;
	const newTime = Math.max(
		0,
		Math.min(
			editorState.playbackTime + count * frameTime,
			totalDuration()
		)
	);
	setEditorState("playbackTime", newTime);
},

stepSeconds: (count: number) => {
	const newTime = Math.max(
		0,
		Math.min(
			editorState.playbackTime + count,
			totalDuration()
		)
	);
	setEditorState("playbackTime", newTime);
},

jumpToStart: () => {
	setEditorState("playbackTime", 0);
},

jumpToEnd: () => {
	setEditorState("playbackTime", totalDuration());
},

jumpToNextBoundary: () => {
	if (!project.timeline?.segments) return;

	const currentTime = editorState.playbackTime;
	const boundaries: number[] = [];
	let accumulatedTime = 0;

	for (const segment of project.timeline.segments) {
		boundaries.push(accumulatedTime);
		accumulatedTime += (segment.end - segment.start) / segment.timescale;
		boundaries.push(accumulatedTime);
	}

	const nextBoundary = boundaries.find(b => b > currentTime + 0.001);
	if (nextBoundary !== undefined) {
		setEditorState("playbackTime", nextBoundary);
	}
},

jumpToPrevBoundary: () => {
	if (!project.timeline?.segments) return;

	const currentTime = editorState.playbackTime;
	const boundaries: number[] = [];
	let accumulatedTime = 0;

	for (const segment of project.timeline.segments) {
		boundaries.push(accumulatedTime);
		accumulatedTime += (segment.end - segment.start) / segment.timescale;
		boundaries.push(accumulatedTime);
	}

	const prevBoundary = boundaries.reverse().find(b => b < currentTime - 0.001);
	if (prevBoundary !== undefined) {
		setEditorState("playbackTime", prevBoundary);
	}
},
```

**Implementation notes:**
- **Frame step:** Uses 1/30th second (30 FPS for navigation as per requirements)
- **Clamping:** All actions clamp to `[0, totalDuration()]` range
- **Boundary calculation:** Accumulates segment durations to find boundary points
- **Tolerance:** 0.001 second tolerance to avoid floating-point issues
- **Reverse search:** `jumpToPrevBoundary` reverses array to find closest previous boundary

---

## Step 2: Register Keyboard Bindings

**File:** `apps/desktop/src/routes/editor/Player.tsx`

**Add these bindings to the `useEditorShortcuts` array (after the existing S05 bindings):**

```typescript
{
	combo: "H",
	handler: () => editorActions.stepFrames(-1),
},
{
	combo: "L",
	handler: () => editorActions.stepFrames(1),
},
{
	combo: "Shift+H",
	handler: () => editorActions.stepSeconds(-1),
},
{
	combo: "Shift+L",
	handler: () => editorActions.stepSeconds(1),
},
{
	combo: "W",
	handler: () => editorActions.jumpToNextBoundary(),
},
{
	combo: "B",
	handler: () => editorActions.jumpToPrevBoundary(),
},
{
	combo: "Digit0",
	handler: () => editorActions.jumpToStart(),
},
{
	combo: "Shift+Digit4",
	handler: () => editorActions.jumpToEnd(),
},
```

**Key notes:**
- `H`/`L` use single uppercase letter (no Shift in combo string)
- `Shift+H`/`Shift+L` explicitly include Shift modifier
- `0` maps to `"0"` because `normalizeCombo()` uses `e.key.toUpperCase()` for single printable chars
- `$` (Shift+4) maps to `"Shift+$"` because `e.key` returns the shifted character

---

## Binding Reference Table

| Key | Combo String | Action | Frame Delta | Time Delta |
|-----|--------------|--------|-------------|------------|
| `h` | `"H"` | Step backward | -1 frame | -1/30s (~0.033s) |
| `l` | `"L"` | Step forward | +1 frame | +1/30s (~0.033s) |
| `Shift+h` | `"Shift+H"` | Step backward | N/A | -1.0s |
| `Shift+l` | `"Shift+L"` | Step forward | N/A | +1.0s |
| `w` | `"W"` | Jump to next segment boundary | N/A | Variable |
| `b` | `"B"` | Jump to previous segment boundary | N/A | Variable |
| `0` | `"0"` | Jump to timeline start | N/A | 0.0s |
| `$` | `"Shift+$"` | Jump to timeline end | N/A | `totalDuration()` |

---

## Boundary Calculation Example

Given segments:
```typescript
[
	{ start: 0, end: 3000, timescale: 1000 },
	{ start: 0, end: 5000, timescale: 1000 },
	{ start: 2000, end: 4000, timescale: 1000 }
]
```

**Calculated boundaries:** `[0, 3, 3, 8, 8, 10]`

**Deduplicated:** `[0, 3, 8, 10]`

**Behavior:**
- From `1.5s`: `w` → `3.0s`, `b` → `0.0s`
- From `3.0s`: `w` → `8.0s`, `b` → `0.0s`
- From `8.0s`: `w` → `10.0s`, `b` → `3.0s`

---

## Edge Cases Handled

### 1. Clamping at Boundaries
```typescript
Math.max(0, Math.min(newTime, totalDuration()))
```
- Frame stepping stops at 0 and totalDuration
- No wraparound behavior

### 2. Empty Timeline
- `jumpToNextBoundary` / `jumpToPrevBoundary` return early if no segments
- Other actions clamp to `[0, recordingDuration]` fallback

### 3. Floating Point Tolerance
- Boundary search uses `> currentTime + 0.001` and `< currentTime - 0.001`
- Prevents getting stuck on current boundary due to floating-point precision

### 4. No Boundary Found
- `find()` returns `undefined` → no change to playhead
- User sees no visible effect (intentional)

### 5. Duplicate Boundaries
- Algorithm naturally produces duplicates at segment joins
- `find()` skips duplicates due to tolerance check

### 6. Playback State Interaction
- Navigation actions work in both playing and paused states
- If playing, Rust backend will update position on next `stopPlayback()`
- Preview mode (`editorState.previewTime`) not modified by navigation

---

## Test Scenarios

### Frame Stepping
- [ ] `h` steps backward 1 frame (~0.033s)
- [ ] `l` steps forward 1 frame (~0.033s)
- [ ] Repeated `h` at start stays at 0
- [ ] Repeated `l` at end stays at totalDuration

### Second Stepping
- [ ] `Shift+h` steps backward 1 second
- [ ] `Shift+l` steps forward 1 second
- [ ] Shift+h from 0.5s goes to 0
- [ ] Shift+l near end clamps to totalDuration

### Boundary Jumping
- [ ] `w` jumps to next segment boundary
- [ ] `b` jumps to previous segment boundary
- [ ] `w` at last boundary does nothing
- [ ] `b` at first boundary does nothing
- [ ] Boundaries align with visible segment edges on timeline

### Start/End Navigation
- [ ] `0` jumps to timeline start (0.0s)
- [ ] `$` (Shift+4) jumps to timeline end
- [ ] Actions work from any playhead position

### Integration
- [ ] Navigation works while playing (updates after stop)
- [ ] Navigation works while paused
- [ ] Timeline scrubber updates to reflect new position
- [ ] Frame preview updates immediately after navigation

### Input Focus Guard
- [ ] Typing `h` in text field does NOT trigger navigation
- [ ] After clicking outside input, shortcuts work

---

## Implementation Checklist

- [ ] Add `stepFrames` action to `editorActions`
- [ ] Add `stepSeconds` action to `editorActions`
- [ ] Add `jumpToStart` action to `editorActions`
- [ ] Add `jumpToEnd` action to `editorActions`
- [ ] Add `jumpToNextBoundary` action to `editorActions`
- [ ] Add `jumpToPrevBoundary` action to `editorActions`
- [ ] Register `H` binding in Player.tsx
- [ ] Register `L` binding in Player.tsx
- [ ] Register `Shift+H` binding in Player.tsx
- [ ] Register `Shift+L` binding in Player.tsx
- [ ] Register `W` binding in Player.tsx
- [ ] Register `B` binding in Player.tsx
- [ ] Register `Digit0` binding in Player.tsx
- [ ] Register `Shift+4` binding in Player.tsx
- [ ] Test all navigation actions
- [ ] Test boundary calculations with multi-segment timeline
- [ ] Test edge cases (empty timeline, single segment)
- [ ] Verify no conflicts with existing shortcuts
- [ ] Verify input focus guard works

---

## Potential Issues

### 1. FPS Mismatch
**Issue:** FPS constant is 60, but task specifies 1/30th second frames

**Resolution:** Use 30 FPS for navigation (1/30s) as specified in requirements, even though playback is 60 FPS

### 2. Boundary Jump Doesn't Find Expected Points
**Issue:** User expects segment-start boundaries but algorithm includes segment-end too

**Analysis:** Algorithm creates boundaries at both start and end of each segment, which provides more navigation points

**Resolution:** Keep current behavior - more boundaries = finer control

### 3. Navigation During Playback
**Issue:** Playback time controlled by Rust backend, frontend changes may be overwritten

**Mitigation:**
- Navigation actions update `playbackTime` immediately
- Effect in `context.ts` (lines 591-601) calls `commands.setPlayheadPosition()` when playback stops
- User should pause before navigating for predictable behavior

### 4. Key Repeat
**Issue:** Current `normalizeCombo()` returns early on `e.repeat`

**Analysis:** Key repeat is helpful for frame stepping but disabled in current implementation

**Resolution:** Accept current behavior - user can press repeatedly if needed. Consider enabling repeat in future if requested.

### 5. `$` Key on Non-US Keyboards
**Issue:** `$` requires Shift+4 on US keyboard but may differ internationally

**Mitigation:** `normalizeCombo()` uses `e.code` which is keyboard-layout-independent. Will produce `"Shift+Digit4"` regardless of layout.

**Fallback:** Document as `Shift+4` rather than `$` in any user-facing text.

---

## Alternative Approaches Considered

### 1. Computed Memo for Boundaries
```typescript
const segmentBoundaries = createMemo(() => {
	if (!project.timeline?.segments) return [];
	let accumulatedTime = 0;
	const boundaries = [0];
	for (const segment of project.timeline.segments) {
		accumulatedTime += (segment.end - segment.start) / segment.timescale;
		boundaries.push(accumulatedTime);
	}
	return boundaries;
});
```

**Rejected because:**
- Adds complexity for minimal performance gain
- Boundary calculation is fast (< 1ms for typical timelines)
- Called only on user key press, not in hot loop
- Adds 15-20 lines vs current inline approach

### 2. Unified `stepTime(delta: number)` Action
```typescript
stepTime: (delta: number) => {
	const newTime = Math.max(0, Math.min(
		editorState.playbackTime + delta,
		totalDuration()
	));
	setEditorState("playbackTime", newTime);
}
```

**Rejected because:**
- Less explicit API (`stepFrames(1)` vs `stepTime(1/30)`)
- Mixing frame and second semantics in one function
- Separate functions make intent clearer in binding handlers

---

## Future Enhancements

**Not in scope for S03:**

1. **Customizable frame step rate:** Allow user to set 1/24, 1/30, 1/60
2. **Jump to IN/OUT points:** Add bindings to jump to set IN/OUT points
3. **Jump by percentage:** `Shift+0-9` jumps to 0%, 10%, ..., 90%
4. **Smart boundary detection:** Detect scene changes, not just segment edges
5. **Key repeat support:** Enable key repeat for frame stepping

---

## Dependencies

- **S01:** ✓ Complete (state management exists)
- **S02:** ✓ Complete (keyboard infrastructure ready)

---

## Validation

Before marking complete:
1. All checklist items checked
2. All test scenarios pass
3. No regressions in existing navigation (Space, timeline scrubbing)
4. Playhead updates reflected in timeline UI
5. Preview frame updates on navigation
