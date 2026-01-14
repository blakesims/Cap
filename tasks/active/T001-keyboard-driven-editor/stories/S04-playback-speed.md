# S04 - Playback Speed Control (Frontend-Only)

## Overview
- **Complexity:** Medium
- **Estimated Time:** ~4-5 hours
- **Lines Added:** ~120
- **Lines Modified:** ~30
- **Lines Removed:** 0
- **Net Change:** ~150 lines
- **Files Changed:** 3 (`context.ts`, `Player.tsx`, `Editor.tsx`)

## Goal
Implement JKL-style playback speed control (1x, 2x, 4x, 8x) using frontend-only simulation. Do NOT modify Rust playback. At non-1x speeds, use `setInterval` to update playhead and trigger frame renders. Audio is naturally muted since Rust playback is not active.

## Current Architecture

### Playback Mechanism

**Two playback modes:**

| Mode | Control | Speed | Audio | When Used |
|------|---------|-------|-------|-----------|
| Rust playback | `commands.startPlayback()` | 1x only | Yes | Normal playback |
| Frontend seek | `setEditorState("playbackTime", t)` | Any | No | Scrubbing/paused |

**Key observations:**
- Rust playback runs at FPS=60, handles audio sync, optimized for 1x speed
- Frontend sets `playbackTime` → triggers `renderFrame()` in Editor.tsx (lines 307-316)
- `renderFrame()` calls `events.renderFrameEvent.emit()` when not playing
- Frame rendering is throttled/debounced to prevent excessive GPU calls

### Speed Control Requirements

**Acceptance criteria:**
- `Ctrl+l` plays forward, cycles speed: 1x → 2x → 4x → 8x (wraps to 8x)
- `Ctrl+j` decreases speed: 8x → 4x → 2x → 1x (stops at 1x)
- `k` pauses and resets speed to 1x
- Speed indicator visible during playback
- Audio muted at non-1x speeds (automatic, no Rust playback)

### Current Play/Pause Logic

**Location:** `Player.tsx` `handlePlayPauseClick()` (lines 165-186)

```typescript
if (isAtEnd()) {
	await commands.stopPlayback();
	setEditorState("playbackTime", 0);
	await commands.seekTo(0);
	await commands.startPlayback(FPS, previewResolutionBase());
	setEditorState("playing", true);
} else if (editorState.playing) {
	await commands.stopPlayback();
	setEditorState("playing", false);
} else {
	await commands.seekTo(Math.floor(editorState.playbackTime * FPS));
	await commands.startPlayback(FPS, previewResolutionBase());
	setEditorState("playing", true);
}
```

---

## Implementation Strategy

**Approach:** Add `playbackSpeed` state. When speed != 1x, use `setInterval` to increment playhead. When speed == 1x, use normal Rust playback.

**Key design decisions:**
1. **State location:** `editorState.playbackSpeed` alongside `playing`
2. **Interval management:** Store interval ID in `editorState.playbackInterval` (or use effect cleanup)
3. **Frame rendering:** Frontend updates `playbackTime` → existing effect in Editor.tsx renders frames
4. **Speed indicator:** Add UI component in Player.tsx controls area
5. **Pause behavior:** `k` always resets to 1x (NLE convention)

---

## Step 1: Add Playback Speed State

**File:** `apps/desktop/src/routes/editor/context.ts`

**In the `createStore` call for `editorState` (around line 637), add:**

```typescript
const [editorState, setEditorState] = createStore({
	previewTime: null as number | null,
	playbackTime: 0,
	playing: false,
	playbackSpeed: 1 as 1 | 2 | 4 | 8,
	playbackInterval: null as number | null,
	inPoint: null as number | null,
	outPoint: null as number | null,
	mark: null as number | null,
	captions: {
		...
	},
	timeline: {
		...
	},
});
```

**Changes:**
- Add `playbackSpeed: 1 as 1 | 2 | 4 | 8` (restricts to valid speeds)
- Add `playbackInterval: null as number | null` (stores setInterval ID)

---

## Step 2: Add Speed Control Actions

**File:** `apps/desktop/src/routes/editor/context.ts`

**Add these functions to `editorActions` object (after navigation actions, around line 816):**

```typescript
increaseSpeed: async () => {
	const speeds: Array<1 | 2 | 4 | 8> = [1, 2, 4, 8];
	const currentIndex = speeds.indexOf(editorState.playbackSpeed);
	const nextSpeed = speeds[Math.min(currentIndex + 1, speeds.length - 1)];

	if (nextSpeed === editorState.playbackSpeed) return;

	const wasPlaying = editorState.playing;

	if (wasPlaying) {
		await editorActions.stopPlayback();
	}

	setEditorState("playbackSpeed", nextSpeed);

	if (wasPlaying || nextSpeed > 1) {
		await editorActions.startPlayback();
	}
},

decreaseSpeed: async () => {
	const speeds: Array<1 | 2 | 4 | 8> = [1, 2, 4, 8];
	const currentIndex = speeds.indexOf(editorState.playbackSpeed);
	const prevSpeed = speeds[Math.max(currentIndex - 1, 0)];

	if (prevSpeed === editorState.playbackSpeed) return;

	const wasPlaying = editorState.playing;

	if (wasPlaying) {
		await editorActions.stopPlayback();
	}

	setEditorState("playbackSpeed", prevSpeed);

	if (wasPlaying) {
		await editorActions.startPlayback();
	}
},

pause: async () => {
	await editorActions.stopPlayback();
	setEditorState("playbackSpeed", 1);
},

startPlayback: async () => {
	if (editorState.playing) return;

	const speed = editorState.playbackSpeed;
	const currentTime = editorState.playbackTime;

	if (speed === 1) {
		await commands.seekTo(Math.floor(currentTime * FPS));
		await commands.startPlayback(FPS, previewResolutionBase());
		setEditorState("playing", true);
	} else {
		setEditorState("playing", true);

		const frameTime = 1 / 30;
		const intervalMs = (frameTime / speed) * 1000;

		const intervalId = window.setInterval(() => {
			const newTime = editorState.playbackTime + frameTime * speed;
			const duration = totalDuration();

			if (newTime >= duration) {
				setEditorState("playbackTime", duration);
				void editorActions.stopPlayback();
			} else {
				setEditorState("playbackTime", newTime);
			}
		}, intervalMs);

		setEditorState("playbackInterval", intervalId);
	}
},

stopPlayback: async () => {
	if (!editorState.playing) return;

	if (editorState.playbackSpeed === 1) {
		await commands.stopPlayback();
	} else {
		if (editorState.playbackInterval !== null) {
			clearInterval(editorState.playbackInterval);
			setEditorState("playbackInterval", null);
		}
	}

	setEditorState("playing", false);
},
```

**Implementation notes:**
- **Speed cycling:** `increaseSpeed` caps at 8x, `decreaseSpeed` floors at 1x
- **State transitions:** Always stop before changing speed, then restart if needed
- **Frame time:** Uses 1/30s base (30 FPS navigation from S03)
- **Interval calculation:** At 2x, interval is (1/30 / 2) * 1000 = ~16.67ms
- **Auto-stop:** Interval checks for end of timeline and stops automatically
- **Cleanup:** `stopPlayback` clears interval and nullifies ID

---

## Step 3: Update Existing Play/Pause Handler

**File:** `apps/desktop/src/routes/editor/Player.tsx`

**Replace `handlePlayPauseClick` (lines 165-186) with:**

```typescript
const handlePlayPauseClick = async () => {
	try {
		if (isAtEnd()) {
			await editorActions.stopPlayback();
			setEditorState("playbackTime", 0);
			setEditorState("playbackSpeed", 1);
			await editorActions.startPlayback();
		} else if (editorState.playing) {
			await editorActions.stopPlayback();
		} else {
			await editorActions.startPlayback();
		}
		if (editorState.playing) setEditorState("previewTime", null);
	} catch (error) {
		console.error("Error handling play/pause:", error);
		setEditorState("playing", false);
	}
};
```

**Changes:**
- Use new `editorActions.startPlayback()` / `stopPlayback()`
- Reset speed to 1x when restarting from end
- Remove direct `commands.startPlayback()` calls

---

## Step 4: Update Prev/Next Buttons

**File:** `apps/desktop/src/routes/editor/Player.tsx`

**Update the "previous" button handler (around line 418):**

```typescript
<button
	type="button"
	class="transition-opacity hover:opacity-70 will-change-[opacity]"
	onClick={async () => {
		await editorActions.stopPlayback();
		setEditorState("playbackTime", 0);
	}}
>
	<IconCapPrev class="text-gray-12 size-3" />
</button>
```

**Update the "next" button handler (around line 442):**

```typescript
<button
	type="button"
	class="transition-opacity hover:opacity-70 will-change-[opacity]"
	onClick={async () => {
		await editorActions.stopPlayback();
		setEditorState("playbackTime", totalDuration());
	}}
>
	<IconCapNext class="text-gray-12 size-3" />
</button>
```

**Changes:**
- Use `editorActions.stopPlayback()` instead of `commands.stopPlayback()`

---

## Step 5: Update Timeline Playback Trigger

**File:** `apps/desktop/src/routes/editor/Timeline/index.tsx`

**Find the createEventListener for click (around lines 276-300) and update the playback section:**

```typescript
if (!editorState.playing) {
	setEditorState("playbackTime", targetTime);
	await commands.seekTo(targetFrame);
	await editorActions.startPlayback();
} else if (editorState.playbackSpeed === 1) {
	await commands.stopPlayback();
	setEditorState("playbackTime", targetTime);
	await commands.seekTo(targetFrame);
	await commands.startPlayback(FPS, previewResolutionBase());
	setEditorState("playing", true);
} else {
	setEditorState("playbackTime", targetTime);
}
```

**Changes:**
- Use `editorActions.startPlayback()` for consistency
- Handle fast playback (just update time, interval continues)

---

## Step 6: Add Speed Indicator UI

**File:** `apps/desktop/src/routes/editor/Player.tsx`

**Add component before `PlayerContent` export (around line 30):**

```typescript
function SpeedIndicator(props: { speed: 1 | 2 | 4 | 8; visible: boolean }) {
	return (
		<div
			class="absolute top-20 right-6 px-3 py-1.5 rounded-lg bg-gray-900/90 text-white text-sm font-medium transition-opacity duration-200"
			style={{
				opacity: props.visible ? 1 : 0,
				"pointer-events": "none",
			}}
		>
			{props.speed}x
		</div>
	);
}
```

**In `PlayerContent`, add after the canvas container (around line 400):**

```typescript
<PreviewCanvas />
<SpeedIndicator
	speed={editorState.playbackSpeed}
	visible={editorState.playing && editorState.playbackSpeed > 1}
/>
<div class="flex overflow-hidden z-10 flex-row gap-3 justify-between items-center p-5">
```

**Implementation notes:**
- Only shows when playing at non-1x speed
- Top-right overlay (doesn't interfere with controls)
- Fades in/out smoothly
- Uses semi-transparent background for visibility

---

## Step 7: Register Keyboard Bindings

**File:** `apps/desktop/src/routes/editor/Player.tsx`

**Add to `useEditorShortcuts` bindings array (after S03 bindings, around line 327):**

```typescript
{
	combo: "Ctrl+L",
	handler: async () => {
		await editorActions.increaseSpeed();
	},
},
{
	combo: "Ctrl+J",
	handler: async () => {
		await editorActions.decreaseSpeed();
	},
},
{
	combo: "K",
	handler: async () => {
		await editorActions.pause();
	},
},
```

**Note:** Use `Ctrl` not `Mod` to avoid conflicts with browser shortcuts (Cmd+L opens address bar)

---

## Step 8: Cleanup on Unmount

**File:** `apps/desktop/src/routes/editor/context.ts`

**Add cleanup in `EditorContextProvider` (in the `onCleanup` callback around line 510):**

```typescript
onCleanup(() => {
	if (projectSaveTimeout) {
		clearTimeout(projectSaveTimeout);
		projectSaveTimeout = undefined;
	}
	if (editorState.playbackInterval !== null) {
		clearInterval(editorState.playbackInterval);
	}
	void flushProjectConfig();
});
```

**Changes:**
- Clear interval on unmount to prevent memory leaks

---

## Binding Reference Table

| Key | Combo String | Action | Speed Change |
|-----|--------------|--------|--------------|
| `Ctrl+l` | `"Ctrl+L"` | Increase playback speed | 1x → 2x → 4x → 8x |
| `Ctrl+j` | `"Ctrl+J"` | Decrease playback speed | 8x → 4x → 2x → 1x |
| `k` | `"K"` | Pause and reset to 1x | Any → 1x (paused) |

---

## State Transition Diagram

```
[Paused @ 1x]
	│
	├─ Ctrl+L ──> [Playing @ 1x] (Rust playback)
	│
	├─ Space ────> [Playing @ 1x] (Rust playback)

[Playing @ 1x]
	│
	├─ Ctrl+L ──> [Playing @ 2x] (Frontend interval)
	│
	├─ k or Space ─> [Paused @ 1x]

[Playing @ 2x]
	│
	├─ Ctrl+L ──> [Playing @ 4x]
	│
	├─ Ctrl+J ──> [Playing @ 1x] (Rust playback)
	│
	├─ k ────────> [Paused @ 1x]

[Playing @ 8x]
	│
	├─ Ctrl+L ──> [Playing @ 8x] (no change)
	│
	├─ Ctrl+J ──> [Playing @ 4x]
	│
	├─ k ────────> [Paused @ 1x]
```

**Key behaviors:**
- Ctrl+L on paused starts playback at 1x (like Space)
- Ctrl+L cycles through speeds, maxes at 8x
- Ctrl+J decreases, floors at 1x
- k always pauses and resets to 1x
- Speed transitions stop current mode, change speed, restart

---

## Frame Rendering Performance

### Interval Timing at Each Speed

| Speed | Frame Time | Interval (ms) | Frames/Second | GPU Load |
|-------|------------|---------------|---------------|----------|
| 1x | 1/30s | Rust-controlled | 60 | Normal (Rust) |
| 2x | 1/30s | ~16.67ms | ~60 | Medium |
| 4x | 1/30s | ~8.33ms | ~120 | High |
| 8x | 1/30s | ~4.17ms | ~240 | Very High |

**Potential optimization (future):**

At 8x, render every Nth frame to reduce GPU load:

```typescript
let frameSkipCounter = 0;
const skipInterval = speed >= 4 ? Math.floor(speed / 2) : 1;

const intervalId = window.setInterval(() => {
	frameSkipCounter++;
	if (frameSkipCounter % skipInterval !== 0 && speed >= 4) {
		return;
	}

	const newTime = editorState.playbackTime + frameTime * speed;
	...
}, intervalMs);
```

**Decision:** Implement only if performance issues observed during testing.

---

## Edge Cases Handled

### 1. Playback at End
```typescript
if (newTime >= duration) {
	setEditorState("playbackTime", duration);
	void editorActions.stopPlayback();
}
```
- Interval detects end and stops automatically
- Speed resets to 1x ready for next play

### 2. Restart from End
```typescript
if (isAtEnd()) {
	await editorActions.stopPlayback();
	setEditorState("playbackTime", 0);
	setEditorState("playbackSpeed", 1);
	await editorActions.startPlayback();
}
```
- Always resets to 1x when replaying from end

### 3. Speed Change During Playback
```typescript
const wasPlaying = editorState.playing;
if (wasPlaying) {
	await editorActions.stopPlayback();
}
setEditorState("playbackSpeed", nextSpeed);
if (wasPlaying) {
	await editorActions.startPlayback();
}
```
- Stops current mode, updates speed, restarts
- Prevents interval leak or Rust playback mismatch

### 4. Multiple Rapid Key Presses
- Each handler checks `if (editorState.playing)` before state changes
- Async handlers prevent race conditions via sequential execution

### 5. Component Unmount During Fast Playback
- `onCleanup` clears interval to prevent memory leak
- Rust playback cleanup already exists

### 6. Timeline Scrubbing During Fast Playback
- Clicking timeline during fast playback:
	- Updates `playbackTime` directly
	- Interval continues from new position
	- No restart needed (smooth UX)

---

## Test Scenarios

### Speed Control
- [ ] `Ctrl+l` from paused starts 1x playback (Rust)
- [ ] `Ctrl+l` during 1x playback switches to 2x (frontend)
- [ ] `Ctrl+l` cycles through 2x → 4x → 8x
- [ ] `Ctrl+l` at 8x does nothing (stays at 8x)
- [ ] `Ctrl+j` at 8x decreases to 4x
- [ ] `Ctrl+j` cycles down to 1x (Rust playback)
- [ ] `Ctrl+j` at 1x does nothing
- [ ] `k` from any speed pauses and resets to 1x

### Speed Indicator
- [ ] Indicator shows "2x" during 2x playback
- [ ] Indicator shows "4x" during 4x playback
- [ ] Indicator shows "8x" during 8x playback
- [ ] Indicator hidden during 1x playback
- [ ] Indicator hidden when paused
- [ ] Indicator fades smoothly

### Playback Behavior
- [ ] Audio plays at 1x speed
- [ ] Audio muted at 2x/4x/8x speeds (no Rust playback)
- [ ] Video frames update smoothly at all speeds
- [ ] Playback stops at end of timeline
- [ ] Restart from end resets to 1x

### Integration
- [ ] Space bar still works for play/pause
- [ ] Timeline scrubbing works during fast playback
- [ ] Navigation shortcuts (h/l/w/b) work after stopping
- [ ] Speed persists across play/pause cycles (until k pressed)
- [ ] Prev/Next buttons work correctly

### Edge Cases
- [ ] Rapidly pressing Ctrl+l doesn't break state
- [ ] Component unmount clears interval
- [ ] Speed change during playback transitions smoothly
- [ ] Scrubbing during 8x playback doesn't cause lag

### Regression
- [ ] Normal 1x playback unchanged
- [ ] Play/pause button works
- [ ] Timeline interactions unchanged
- [ ] All existing shortcuts still work

---

## Implementation Checklist

- [ ] Add `playbackSpeed` to `editorState`
- [ ] Add `playbackInterval` to `editorState`
- [ ] Add `increaseSpeed` action to `editorActions`
- [ ] Add `decreaseSpeed` action to `editorActions`
- [ ] Add `pause` action to `editorActions`
- [ ] Add `startPlayback` action to `editorActions`
- [ ] Add `stopPlayback` action to `editorActions`
- [ ] Update `handlePlayPauseClick` in Player.tsx
- [ ] Update prev/next button handlers in Player.tsx
- [ ] Update timeline click handler in Timeline/index.tsx
- [ ] Add `SpeedIndicator` component
- [ ] Add speed indicator to canvas container
- [ ] Register `Ctrl+L` binding
- [ ] Register `Ctrl+J` binding
- [ ] Register `K` binding
- [ ] Add interval cleanup in `onCleanup`
- [ ] Test all speed transitions
- [ ] Test speed indicator visibility
- [ ] Test audio muting at non-1x speeds
- [ ] Test playback at end behavior
- [ ] Verify no regressions

---

## Potential Issues

### 1. Frame Rendering Performance at 8x

**Issue:** At 8x, interval fires every ~4ms, potentially overwhelming GPU

**Mitigation:**
- Existing throttle/debounce in Editor.tsx (lines 292-294) already limits GPU calls
- `renderFrame()` uses `throttle(emitRenderFrame, 1000 / FPS)` = ~16.67ms minimum
- So even at 8x interval, actual GPU renders capped at 60 FPS

**If issues persist:**
- Implement frame skipping (render every 2nd frame at 4x, every 4th at 8x)
- Add performance mode toggle (lower preview quality at high speeds)

### 2. Interval Drift Over Time

**Issue:** `setInterval` can drift, causing playback to desync from expected time

**Analysis:**
- At 2x over 10 seconds: drift < 100ms (acceptable for scrubbing)
- Users at 2x-8x are scanning, not watching precise content
- When they pause or return to 1x, position is accurate

**Mitigation:**
- Accept current behavior (standard for frontend-based speed control)
- If issues reported, switch to `requestAnimationFrame` with timestamp tracking

### 3. Browser Tab Backgrounding

**Issue:** Background tabs throttle `setInterval` to 1000ms minimum

**Impact:**
- Fast playback pauses when tab backgrounded
- Returns to normal when tab refocused

**Mitigation:**
- Document as expected behavior (desktop app usually focused)
- Consider `requestAnimationFrame` if users report issues

### 4. Ctrl+L Browser Shortcut

**Issue:** On some browsers, Ctrl+L might open location bar

**Resolution:**
- `preventDefault: true` (default in `useEditorShortcuts`) blocks browser action
- macOS users: Cmd+L is address bar, Ctrl+L is safe
- Windows/Linux: Ctrl+L is address bar, may conflict

**Testing:** Verify on Windows/Linux that preventDefault works

### 5. Speed Indicator Overlap

**Issue:** Speed indicator might overlap video content

**Mitigation:**
- Positioned top-right with margin (safe zone)
- Semi-transparent background ensures visibility
- Only shows during fast playback (temporary)

**If issues:** Move to bottom-right or integrate into timeline controls

---

## Alternative Approaches Considered

### 1. Modify Rust Playback for Variable Speed

**Approach:** Add speed parameter to `commands.startPlayback(fps, resolution, speed)`

**Rejected because:**
- Rust playback has complex audio sync logic optimized for 1x
- Audio resampling at 2x-8x requires significant FFmpeg work
- High risk of breaking existing playback
- Frontend simulation adequate for scrubbing use case
- Professional NLEs (Premiere, DaVinci) use similar approach

### 2. Reverse Playback Support

**Approach:** Add `Ctrl+j` for reverse playback (like some NLEs)

**Rejected because:**
- Video decoders don't efficiently support backward seeking
- Would require pre-decoding entire video into RAM
- Explicitly marked "NOT IN SCOPE" in main.md
- Can revisit if user requests it

### 3. Audio Pitch Preservation

**Approach:** At 2x speed, use time-stretch algorithm to preserve pitch

**Rejected because:**
- Adds significant complexity (audio resampling, FFmpeg integration)
- Users at 2x-8x are scrubbing, not listening
- Can add later if users request it

### 4. Smooth Speed Ramping

**Approach:** Gradually accelerate from 1x → 2x → 4x instead of instant switch

**Rejected because:**
- JKL convention is instant speed changes
- Adds UI complexity for minimal UX benefit
- Users expect discrete steps

---

## Future Enhancements

**Not in scope for S04:**

1. **Customizable speed steps:** Allow 1.5x, 3x, etc. via settings
2. **Audio at 2x:** Enable audio with pitch preservation at 2x speed
3. **Reverse playback:** `Ctrl+j` for backward playback (complex, requires caching)
4. **Frame skipping optimization:** Render every Nth frame at 8x for performance
5. **Speed presets:** Save favorite speeds to quick-access buttons
6. **Speed indicator customization:** Position, size, show/hide preference

---

## Dependencies

- **S01:** ✓ Complete (state management exists)
- **S02:** ✓ Complete (keyboard infrastructure ready, Ctrl modifier support)
- **S03:** ✓ Complete (navigation actions exist, frame stepping established)

---

## Validation

Before marking complete:
1. All checklist items checked
2. All test scenarios pass
3. Speed indicator shows/hides correctly
4. Audio muted at non-1x speeds
5. No regressions in normal playback
6. Frame rendering smooth at all speeds
7. Cleanup verified (no memory leaks)
8. Speed persists across play/pause cycles
