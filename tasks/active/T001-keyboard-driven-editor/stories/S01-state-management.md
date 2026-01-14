# S01 - State Management for IN/OUT Points and Marks

## Overview
- **Complexity:** Low
- **Estimated Time:** ~2 hours
- **Lines Added:** ~50
- **Lines Modified:** 0
- **Files Changed:** 1 (`context.ts`)

## Goal
Add session-only state for IN/OUT points and marks to enable keyboard-driven region selection and navigation.

## Upstream Sync Impact
**Minimal** - All changes are additive:
- No existing interfaces modified
- No existing functions changed
- New state properties added at end of store
- New `editorActions` object added alongside existing `projectActions`

---

## Implementation Details

### Step 1: Add State Properties

**File:** `apps/desktop/src/routes/editor/context.ts`

**Location:** Inside `createStore` call (around line 50-80), add after `playing: false,`

```typescript
const [editorState, setEditorState] = createStore({
  previewTime: null as number | null,
  playbackTime: 0,
  playing: false,
  // ADD THESE THREE LINES:
  inPoint: null as number | null,
  outPoint: null as number | null,
  mark: null as number | null,
  // ... rest of existing state
  captions: { ... },
  timeline: { ... },
});
```

**Rationale:**
- Placed at top level (not inside `timeline`) because these relate to playback position
- Grouped with `playbackTime` and `previewTime` for logical consistency
- Using `number | null` pattern already established in the codebase

---

### Step 2: Add Editor Actions

**File:** `apps/desktop/src/routes/editor/context.ts`

**Location:** After `projectActions` definition (around line 467), before the `return` statement

```typescript
const editorActions = {
  setInPoint: () => {
    const time = editorState.previewTime ?? editorState.playbackTime;
    setEditorState("inPoint", time);
    if (editorState.outPoint !== null && editorState.outPoint < time) {
      setEditorState("outPoint", null);
    }
  },

  setOutPoint: () => {
    const time = editorState.previewTime ?? editorState.playbackTime;
    setEditorState("outPoint", time);
    if (editorState.inPoint !== null && editorState.inPoint > time) {
      setEditorState("inPoint", null);
    }
  },

  clearInOut: () => {
    batch(() => {
      setEditorState("inPoint", null);
      setEditorState("outPoint", null);
    });
  },

  setMark: () => {
    const time = editorState.previewTime ?? editorState.playbackTime;
    setEditorState("mark", time);
  },

  jumpToMark: () => {
    if (editorState.mark !== null) {
      setEditorState("playbackTime", editorState.mark);
    }
  },

  clearMark: () => {
    setEditorState("mark", null);
  },

  clearAll: () => {
    batch(() => {
      setEditorState("inPoint", null);
      setEditorState("outPoint", null);
      setEditorState("mark", null);
    });
  },
};
```

**Note:** Import `batch` from `solid-js` if not already imported:
```typescript
import { batch } from "solid-js";
```

---

### Step 3: Export Actions

**File:** `apps/desktop/src/routes/editor/context.ts`

**Location:** In the `return` statement (around line 709), add `editorActions`:

```typescript
return {
  ...editorInstanceContext,
  meta() { ... },
  // ... existing properties ...
  projectActions,
  editorActions,  // ADD THIS LINE
  // ... rest of existing properties ...
};
```

---

## Edge Cases Handled

| Edge Case | Behavior | Implementation |
|-----------|----------|----------------|
| Set OUT before IN exists | OUT is set normally | Direct `setEditorState` call |
| Set IN after OUT (IN > OUT) | OUT is cleared | Check in `setInPoint()` |
| Set OUT before IN (OUT < IN) | IN is cleared | Check in `setOutPoint()` |
| Set mark while playing | Uses `playbackTime` | `previewTime ?? playbackTime` |
| Set mark while hovering | Uses `previewTime` | `previewTime ?? playbackTime` |
| Jump to mark when null | No-op | Guard `if (mark !== null)` |
| Project switch | State resets | Context remounts automatically |

---

## State Reset Behavior

**No explicit reset needed.** The `EditorContextProvider` is created fresh when a project is opened:

1. `Editor.tsx` renders `EditorInstanceContextProvider`
2. When project path changes, the resource re-fetches
3. `EditorContextProvider` receives new `editorInstance` as prop
4. SolidJS creates new context value with fresh `editorState`

**Verified:** The state initialization is inside the provider callback, so it resets on remount.

---

## Test Scenarios

### Manual Testing Checklist

After S05 (keyboard bindings) is complete:

- [ ] **Set IN:** Press `i`, verify IN point appears at playhead position
- [ ] **Set OUT:** Press `o`, verify OUT point appears at playhead position
- [ ] **IN/OUT ordering:** Set OUT at 5s, then IN at 10s → OUT should clear
- [ ] **OUT/IN ordering:** Set IN at 10s, then OUT at 5s → IN should clear
- [ ] **Set Mark:** Press `m`, verify mark appears at playhead position
- [ ] **Jump to Mark:** Set mark at 5s, move to 0s, press `'` → playhead at 5s
- [ ] **Clear IN/OUT:** Press `Escape` → both IN and OUT are null
- [ ] **Project switch:** Set IN/OUT/mark, open different project → all null

### Programmatic Verification (during implementation)

In browser devtools, with editor open:
```javascript
// Access context (SolidJS devtools or manual)
const ctx = /* get editor context */;

// Test setInPoint
ctx.editorActions.setInPoint();
console.log(ctx.editorState.inPoint); // Should be current playhead

// Test clearInOut
ctx.editorActions.clearInOut();
console.log(ctx.editorState.inPoint, ctx.editorState.outPoint); // null, null
```

---

## Dependencies

**Required for S01:**
- None (standalone state addition)

**S01 enables:**
- S02 - Keyboard bindings will call these actions
- S05 - Timeline UI will read these state values
- S06 - Delete logic will use IN/OUT range

---

## Checklist

- [ ] Add `inPoint`, `outPoint`, `mark` to `editorState` store
- [ ] Verify `batch` is imported from `solid-js`
- [ ] Add `editorActions` object with 7 methods
- [ ] Add `editorActions` to context return value
- [ ] Verify no TypeScript errors
- [ ] Test in browser devtools that state updates work
