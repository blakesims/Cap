# S02 - Core Keyboard Bindings Infrastructure

## Overview
- **Complexity:** Medium
- **Estimated Time:** ~4-6 hours
- **Lines Added:** ~105
- **Lines Modified:** ~20
- **Lines Removed:** ~55
- **Net Change:** ~70 lines
- **Files Changed:** 3 (`useEditorShortcuts.ts`, `Player.tsx`, `Timeline/index.tsx`)

## Goal
Extend the keyboard binding infrastructure to support standalone letter keys, Shift modifier, and Ctrl modifier. Consolidate handlers from Timeline into Player. Connect `editorActions` from S01.

## Current Architecture

### Three Separate Keyboard Handlers

| Location | Target | Guard | Handlers |
|----------|--------|-------|----------|
| `useEditorShortcuts` (Player) | `document` | `getScopeActive()` | S, Mod+=, Mod+-, Space |
| Timeline `createEventListener` | `window` | Inline input check | Backspace, Delete, C, Escape |
| Context history listener | `window` | None | Ctrl/Cmd+Z/Y |

### Problems with Current `normalizeCombo()`

1. `h` and `Shift+h` both produce `"H"` (no Shift tracking)
2. `Ctrl+l` produces `"Mod+L"` (Ctrl conflated with Cmd)
3. `$` (Shift+4) not handled (produces `"Digit4"`)

---

## Implementation Strategy

**Approach:** Extend existing `normalizeCombo()` + consolidate handlers (minimal upstream diff)

---

## Step 1: Update `normalizeCombo()`

**File:** `apps/desktop/src/routes/editor/useEditorShortcuts.ts`

**Replace the existing `normalizeCombo` function with:**

```typescript
function normalizeCombo(e: KeyboardEvent): string {
	const parts: string[] = [];

	if (e.ctrlKey && !e.metaKey) parts.push("Ctrl");
	if (e.metaKey) parts.push("Mod");
	if (e.shiftKey) parts.push("Shift");
	if (e.altKey) parts.push("Alt");

	let key: string;

	if (e.key.length === 1 && !e.ctrlKey && !e.metaKey) {
		key = e.key.toUpperCase();
	} else {
		switch (e.code) {
			case "Equal":
				key = "=";
				break;
			case "Minus":
				key = "-";
				break;
			case "Space":
				key = "Space";
				break;
			case "Escape":
				key = "Escape";
				break;
			case "Backspace":
				key = "Backspace";
				break;
			case "Delete":
				key = "Delete";
				break;
			default:
				key = e.code.startsWith("Key") ? e.code.slice(3) : e.code;
		}
	}

	parts.push(key);
	return parts.join("+");
}
```

**Key changes:**
- `Ctrl` tracked separately from `Mod` (for Ctrl+j/l on macOS)
- `Shift` and `Alt` now tracked
- For single printable chars without Ctrl/Cmd, use `e.key.toUpperCase()`
- Handles `$`, `'`, `` ` `` correctly

---

## Step 2: Add Input Focus Guard Export

**File:** `apps/desktop/src/routes/editor/useEditorShortcuts.ts`

**Add after the imports:**

```typescript
export function isInputFocused(): boolean {
	const el = document.activeElement;
	if (!el) return false;
	const tagName = el.tagName.toLowerCase();
	const isContentEditable = el.getAttribute("contenteditable") === "true";
	return tagName === "input" || tagName === "textarea" || isContentEditable;
}
```

---

## Step 3: Expand Bindings in Player.tsx

**File:** `apps/desktop/src/routes/editor/Player.tsx`

**Update import:**
```typescript
import { useEditorShortcuts, isInputFocused } from "./useEditorShortcuts";
```

**Add helper function (inside PlayerContent):**
```typescript
const handleDeleteSelection = () => {
	const selection = editorState.timeline.selection;
	if (!selection) return;

	if (selection.type === "zoom") {
		projectActions.deleteZoomSegments(selection.indices);
	} else if (selection.type === "mask") {
		projectActions.deleteMaskSegments(selection.indices);
	} else if (selection.type === "text") {
		projectActions.deleteTextSegments(selection.indices);
	} else if (selection.type === "clip") {
		[...selection.indices]
			.sort((a, b) => b - a)
			.forEach((idx) => projectActions.deleteClipSegment(idx));
	} else if (selection.type === "scene") {
		[...selection.indices]
			.sort((a, b) => b - a)
			.forEach((idx) => projectActions.deleteSceneSegment(idx));
	}
};
```

**Expand useEditorShortcuts bindings array:**
```typescript
useEditorShortcuts(() => !isInputFocused(), [
	// === Existing bindings ===
	{
		combo: "S",
		handler: () =>
			setEditorState(
				"timeline",
				"interactMode",
				editorState.timeline.interactMode === "split" ? "seek" : "split"
			),
	},
	{
		combo: "Mod+=",
		handler: () =>
			editorState.timeline.transform.updateZoom(
				editorState.timeline.transform.zoom / 1.1,
				editorState.playbackTime
			),
	},
	{
		combo: "Mod+-",
		handler: () =>
			editorState.timeline.transform.updateZoom(
				editorState.timeline.transform.zoom * 1.1,
				editorState.playbackTime
			),
	},
	{
		combo: "Space",
		handler: async () => {
			const prevTime = editorState.previewTime;
			if (!editorState.playing) {
				if (prevTime !== null) setEditorState("playbackTime", prevTime);
				await commands.seekTo(Math.floor(editorState.playbackTime * FPS));
			}
			await handlePlayPauseClick();
		},
	},

	// === Migrated from Timeline ===
	{
		combo: "C",
		handler: () => {
			const time = editorState.previewTime ?? editorState.playbackTime;
			if (time !== null && time !== undefined) {
				projectActions.splitClipSegment(time);
			}
		},
	},
	{
		combo: "Backspace",
		handler: handleDeleteSelection,
	},
	{
		combo: "Delete",
		handler: handleDeleteSelection,
	},
	{
		combo: "Escape",
		handler: () => {
			setEditorState("timeline", "selection", null);
			editorActions.clearInOut();
		},
	},

	// === New S05 bindings (IN/OUT and marks) ===
	{
		combo: "I",
		handler: () => editorActions.setInPoint(),
	},
	{
		combo: "O",
		handler: () => editorActions.setOutPoint(),
	},
	{
		combo: "M",
		handler: () => editorActions.setMark(),
	},
	{
		combo: "'",
		handler: () => editorActions.jumpToMark(),
	},
	{
		combo: "`",
		handler: () => editorActions.jumpToMark(),
	},
]);
```

**Note:** Get `editorActions` from context:
```typescript
const { editorActions, projectActions, ... } = useEditorContext();
```

---

## Step 4: Remove Duplicate Handler from Timeline

**File:** `apps/desktop/src/routes/editor/Timeline/index.tsx`

**Remove the entire `createEventListener(window, "keydown", ...)` block (approximately lines 309-354).**

This removes ~45 lines of duplicated keyboard handling.

---

## Binding Reference Table

| Key | Combo String | Action | Story |
|-----|--------------|--------|-------|
| `s` | `"S"` | Toggle split mode | Existing |
| `c` | `"C"` | Cut at playhead | Migrated |
| `Space` | `"Space"` | Play/pause | Existing |
| `Backspace` | `"Backspace"` | Delete selection | Migrated |
| `Delete` | `"Delete"` | Delete selection | Migrated |
| `Escape` | `"Escape"` | Clear selection + IN/OUT | Extended |
| `i` | `"I"` | Set IN point | S05/New |
| `o` | `"O"` | Set OUT point | S05/New |
| `m` | `"M"` | Set mark | S05/New |
| `'` | `"'"` | Jump to mark | S05/New |
| `` ` `` | `"\`"` | Jump to mark | S05/New |

**Bindings prepared for S03-S04 (infrastructure ready, handlers added later):**
- `h`, `l`, `Shift+H`, `Shift+L`, `w`, `b`, `0`, `$` (S03)
- `Ctrl+J`, `Ctrl+L`, `k` (S04)

---

## Conflict Analysis

| Existing | New | Resolution |
|----------|-----|------------|
| `Escape` - clear selection | + clear IN/OUT | Extended behavior |
| `Cmd+L` - browser address bar | `Ctrl+L` - playback speed | Use Ctrl, not Cmd |

---

## Test Scenarios

### Regression Tests
- [ ] `S` toggles split mode
- [ ] `Cmd+=` zooms in
- [ ] `Cmd+-` zooms out
- [ ] `Space` toggles play/pause
- [ ] `C` cuts at playhead (only in seek mode)
- [ ] `Backspace` deletes selected segment
- [ ] `Delete` deletes selected segment
- [ ] `Escape` clears selection
- [ ] `Cmd+Z` undoes
- [ ] `Cmd+Shift+Z` redoes

### New Binding Tests
- [ ] `i` sets IN point at playhead
- [ ] `o` sets OUT point at playhead
- [ ] `m` sets mark at playhead
- [ ] `'` jumps to mark
- [ ] `` ` `` jumps to mark
- [ ] `Escape` clears both selection AND IN/OUT

### Input Focus Guard Tests
- [ ] Typing `i` in text field does NOT trigger IN point
- [ ] After clicking outside input, shortcuts work

### Modifier Discrimination Tests
- [ ] `h` is different from `Shift+h` (prepared for S03)
- [ ] `Ctrl+l` (macOS) is different from `l`

---

## Implementation Checklist

- [x] Update `normalizeCombo()` to track Ctrl, Shift, Alt separately
- [x] Add `isInputFocused()` export
- [x] Add `handleDeleteSelection` helper in Player.tsx
- [x] Update import to include `isInputFocused`
- [x] Get `editorActions` and `projectActions` from context in Player.tsx
- [x] Expand bindings array with migrated + new bindings
- [x] Remove duplicate handler from Timeline/index.tsx
- [ ] Test all existing bindings still work
- [ ] Test new bindings work
- [ ] Test input focus guard works

---

## Potential Issues

1. **Browser `Cmd+L`:** Opens address bar. We use `Ctrl+L` for playback speed (S04).
2. **Key repeat:** Current code has `if (e.repeat) return;`. May need to allow for navigation keys in S03.
3. **Backspace behavior change:** Timeline currently checks `Cmd+Backspace` separately for full delete. After migration, only plain `Backspace` triggers delete. This is intentional - `Cmd+Backspace` is a macOS system shortcut (delete line) and shouldn't be overridden.
