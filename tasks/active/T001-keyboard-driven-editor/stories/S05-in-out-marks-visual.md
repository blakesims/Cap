# S05: IN/OUT Points and Marks - Visual Feedback

## Status
✅ **IMPLEMENTED** (2026-01-14)

## Summary
Add visual overlays to the Timeline for IN/OUT region, IN/OUT point indicators, and mark indicator. All state and keyboard actions already exist in context.ts (S01) and Player.tsx (S02-S04). This story is purely about rendering the visual feedback.

## Technical Context

### Existing State (context.ts)
```typescript
editorState: {
  inPoint: null as number | null,
  outPoint: null as number | null,
  mark: null as number | null,
}
```

### Existing Actions (context.ts)
- `editorActions.setInPoint()` - sets inPoint to current time
- `editorActions.setOutPoint()` - sets outPoint to current time
- `editorActions.clearInOut()` - clears both points
- `editorActions.setMark()` - sets mark
- `editorActions.jumpToMark()` - jumps to mark
- `editorActions.clearMark()` - clears mark

### Existing Keyboard Bindings (Player.tsx)
All bindings already wired up:
- `I` → setInPoint (line 288)
- `O` → setOutPoint (line 292)
- `M` → setMark (line 296)
- `'` and `` ` `` → jumpToMark (lines 299-304)
- `Escape` → clearInOut + clear selection (lines 280-284)

## Implementation Checklist

### 1. Add IN/OUT Region Overlay
**File**: `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/Timeline/index.tsx`

**Location**: Inside the scrollable div at line 472 (`<div class="absolute inset-0 overflow-y-auto overflow-x-hidden pr-1">`), specifically between lines 479-480, before the first `<TrackRow>`. This ensures the overlays scroll with the tracks and render above them.

```tsx
<Show when={inOutRegion()}>
  {(region) => (
    <div
      class="absolute pointer-events-none z-[6]"
      style={{
        left: `${TRACK_GUTTER + region().left}px`,
        width: `${region().width}px`,
        top: "0px",
        bottom: "0px",
        "background-color": "rgba(74, 158, 255, 0.15)",
        "border-left": "2px solid rgb(107, 203, 119)",
        "border-right": "2px solid rgb(226, 64, 64)",
      }}
    />
  )}
</Show>
```

**Add computed signal** (around line 103, near `secsPerPixel`):
```tsx
const inOutRegion = () => {
  const inP = editorState.inPoint;
  const outP = editorState.outPoint;
  if (inP === null || outP === null) return null;

  const pos = transform().position;
  const left = (Math.min(inP, outP) - pos) / secsPerPixel();
  const right = (Math.max(inP, outP) - pos) / secsPerPixel();
  return { left, width: right - left };
};
```

**CRITICAL**: The calculation must subtract `transform().position` (scroll offset) from the time values. Without this, the region would be positioned incorrectly when the timeline is scrolled.

**Key decisions**:
- Use `z-[6]` to sit above tracks but below playhead (playhead is z-10)
- Green border (left) matches IN point color
- Red border (right) matches OUT point color and playhead red
- Semi-transparent blue background (0.15 opacity) for subtle highlighting
- Position relative to TRACK_GUTTER to align with tracks

### 2. Add IN/OUT Point Flags
**File**: Same as above

**Location**: Add immediately after the IN/OUT region overlay (still between lines 479-480, before TrackRow components):

```tsx
<Show when={editorState.inPoint !== null}>
  <div
    class="absolute pointer-events-none z-[7]"
    style={{
      left: `${TRACK_GUTTER}px`,
      transform: `translateX(${(editorState.inPoint! - transform().position) / secsPerPixel()}px)`,
      top: "-20px",
    }}
  >
    <div
      class="px-1.5 py-0.5 rounded text-[10px] font-bold"
      style={{
        "background-color": "rgb(107, 203, 119)",
        color: "#000",
      }}
      title={`IN: ${formatTime(editorState.inPoint!)}`}
    >
      I
    </div>
  </div>
</Show>

<Show when={editorState.outPoint !== null}>
  <div
    class="absolute pointer-events-none z-[7]"
    style={{
      left: `${TRACK_GUTTER}px`,
      transform: `translateX(${(editorState.outPoint! - transform().position) / secsPerPixel()}px)`,
      top: "-20px",
    }}
  >
    <div
      class="px-1.5 py-0.5 rounded text-[10px] font-bold"
      style={{
        "background-color": "rgb(226, 64, 64)",
        color: "#fff",
      }}
      title={`OUT: ${formatTime(editorState.outPoint!)}`}
    >
      O
    </div>
  </div>
</Show>
```

**Key decisions**:
- Position at `top: -20px` to sit above timeline tracks
- Use inline styles to match exact colors from prototype and existing playhead
- IN flag: green background `rgb(107, 203, 119)`, black text
- OUT flag: red background `rgb(226, 64, 64)` (matches playhead), white text
- Use `formatTime` utility (already imported) for tooltip
- Transform based on scroll position like the playhead does
- Z-index 7 to be above region (6) but below playhead (10)

### 3. Add Mark Indicator
**File**: Same as above

**Location**: Add after the OUT point indicator:

```tsx
<Show when={editorState.mark !== null}>
  <div
    class="absolute pointer-events-none z-[7]"
    style={{
      left: `${TRACK_GUTTER}px`,
      transform: `translateX(${(editorState.mark! - transform().position) / secsPerPixel()}px)`,
      top: "-20px",
    }}
  >
    <div
      class="px-1.5 py-0.5 rounded text-[10px] font-bold"
      style={{
        "background-color": "rgb(155, 89, 182)",
        color: "#fff",
      }}
      title={`Mark: ${formatTime(editorState.mark!)}`}
    >
      M
    </div>
  </div>
</Show>
```

**Key decisions**:
- Purple color `rgb(155, 89, 182)` to distinguish from IN/OUT points
- Same positioning pattern as IN/OUT flags
- White text on purple background

### 4. Verify Escape Handler (No Changes Needed)
The Escape handler in Player.tsx (lines 280-284) already does exactly what's needed:
```typescript
{
  combo: "Escape",
  handler: () => {
    setEditorState("timeline", "selection", null);
    editorActions.clearInOut();
  },
}
```
This clears both selection and IN/OUT points in a single action.

## Color Palette

Match existing editor colors:
- **IN point**: `rgb(107, 203, 119)` (green) - for growth/start
- **OUT point**: `rgb(226, 64, 64)` (red) - matches existing playhead color
- **Mark**: `rgb(155, 89, 182)` (purple) - distinct from IN/OUT
- **Region overlay**: `rgba(74, 158, 255, 0.15)` (semi-transparent blue)

## Implementation Notes

### Z-Index Strategy
- Tracks: default (1-5)
- IN/OUT region: 6
- IN/OUT/Mark flags: 7
- Preview playhead: 10 (existing, gray)
- Playback playhead: 10 (existing, red)

**Note**: These z-index values should be verified visually during implementation. The overlays render inside the scrollable div (`overflow-y-auto`) so they need to be positioned correctly to appear above tracks but not interfere with playheads.

### Scroll Behavior
All indicators must transform with timeline scroll:
```tsx
transform: `translateX(${(time - transform().position) / secsPerPixel()}px)`
```
This matches the existing playhead implementation (lines 461-467).

### Integration Points

1. **Import formatTime** - Already imported at line 20
2. **Access editorState** - Already available via `useEditorContext()` at line 82
3. **TRACK_GUTTER constant** - Already defined at line 30 (64px)
4. **secsPerPixel function** - Already defined at line 102
5. **transform accessor** - Already defined at line 97

### Constants Already Available
```typescript
const TRACK_GUTTER = 64;
const secsPerPixel = () => transform().zoom / (timelineBounds.width ?? 1);
const transform = () => editorState.timeline.transform;
```

## Testing Scenarios

### Visual Testing
1. **No marks set**: Timeline should look normal with no overlays
2. **Set IN point (press I)**: Green "I" flag appears above timeline at playhead position
3. **Set OUT point (press O)**: Red "O" flag appears above timeline at playhead position
4. **Both IN and OUT set**:
   - Both flags visible
   - Semi-transparent blue region between them
   - Green border on left side of region
   - Red border on right side of region
5. **Set mark (press M)**: Purple "M" flag appears at playhead position
6. **Scroll timeline**: All indicators should scroll with timeline content
7. **Zoom timeline**: Region width should adjust, flags should maintain position relative to time
8. **Press Escape**: All IN/OUT indicators disappear, selection clears
9. **Hover flags**: Tooltips show formatted time (e.g., "IN: 0:05:15")

### Interaction Testing
1. **Press I twice at different times**: IN flag moves to new position
2. **Set OUT before IN**: Both appear correctly (min/max calculation handles order)
3. **Press ' or `**: Playhead jumps to mark position
4. **Set mark multiple times**: Mark flag moves to latest position
5. **Play through IN/OUT region**: Region remains visible, playhead moves through it

### Edge Cases
1. **IN and OUT at same time**: Region width = 0 (borderline visible)
2. **Mark overlaps with IN or OUT**: All three flags stack vertically if needed (natural CSS flow)
3. **Indicator off-screen left**: Hidden by transform (negative translateX)
4. **Indicator off-screen right**: Hidden by overflow
5. **Very zoomed in**: Region could span entire viewport
6. **Very zoomed out**: Region could be a thin line

## Visual Reference

Prototype structure (from keyboard-prototype/src/Timeline.tsx):
```tsx
<Show when={inOutRegion()}>
  <div class="in-out-region" style={{ left, width }} />
</Show>

<Show when={markPosition() !== null}>
  <div class="mark-indicator" style={{ left }}>
    <div class="mark-flag">M</div>
  </div>
</Show>

<Show when={inPoint !== null}>
  <div class="in-point-indicator" style={{ left }}>
    <div class="point-flag in">I</div>
  </div>
</Show>

<Show when={outPoint !== null}>
  <div class="out-point-indicator" style={{ left }}>
    <div class="point-flag out">O</div>
  </div>
</Show>
```

## Implementation Order

1. Add `inOutRegion` computed signal near other computed values
2. Add IN/OUT region overlay first (easiest to see and test)
3. Add IN point flag
4. Add OUT point flag
5. Add mark indicator
6. Test all scenarios
7. Test scroll and zoom behavior
8. Verify Escape behavior (should already work)

## Definition of Done

- [ ] IN/OUT region shaded area appears when both points set
- [ ] Green "I" flag appears when IN point set
- [ ] Red "O" flag appears when OUT point set
- [ ] Purple "M" flag appears when mark set
- [ ] All indicators scroll with timeline
- [ ] All indicators scale correctly when zooming
- [ ] Tooltips show formatted time on hover
- [ ] Escape clears IN/OUT points and selection
- [ ] No visual regressions in timeline appearance
- [ ] All indicators positioned relative to TRACK_GUTTER

## Files Modified

- `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/Timeline/index.tsx` - Add visual overlays

## Files NOT Modified

- `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/context.ts` - State already exists
- `/home/blake/repos/cap-repo-fork/Cap/apps/desktop/src/routes/editor/Player.tsx` - Keyboard bindings already exist
- No CSS files needed - all styling inline to minimize changes

## Estimated Complexity
**Low** - Pure presentation layer, all logic already implemented. ~50 lines of JSX.
