# Task: [T004] - Editor Playback Improvements

## 0. Task Summary
-  **Task Name:** Editor Playback Improvements
-  **Priority:** 2
-  **Number of Stories:** 2
-  **Current Status:** COMPLETED
-  **Platform:** macOS
-  **Dependencies:** `crates/editor/`, `apps/desktop/src/routes/editor/`
-  **Rules Required:** CLAUDE.md
-  **Acceptance Criteria:**
    - ✅ Timeline auto-scrolls to keep playhead visible during navigation/playback
    - ✅ Audio playback starts immediately when pressing play (reduced from 7s to ~120ms)

## 1. Goal / Objective
Fix two editor UX issues: (1) timeline doesn't follow the playhead, requiring manual scrolling; (2) audio has significant latency on playback start while video plays immediately.

## 2. Overall Status
**S01 COMPLETED (2026-01-26)**: Auto-scroll timeline to keep playhead visible via reactive effect.

**S02 COMPLETED (2026-01-26)**: Audio latency reduced from ~7 seconds to ~120ms (98% improvement) via background pre-decoding at output device sample rate.

---

## 3. Stories Breakdown

| Story ID | Story Name / Objective | Complexity | Est. Hours | Status |
| :--- | :--- | :--- | :--- | :--- |
| S01 | Auto-scroll timeline to keep playhead visible | Low | 1 | ✅ COMPLETED |
| S02 | Fix audio playback latency | High | 8 | ✅ COMPLETED |

---

## 4. Story Details

### S01 - Auto-Scroll Timeline

**Problem:** When navigating with keyboard (w/b, h/l) or during playback, the playhead can move outside the visible viewport. User must manually scroll.

**Desired Behavior:**
- When playhead exits visible viewport, jump viewport so playhead is at 1/3 from left edge
- This gives ~2/3 of viewport as "runway" before next jump needed
- Should work during: keyboard navigation, playback, segment jumps
- Instant jump (no animation)

**Key Files:**
- `apps/desktop/src/routes/editor/context.ts` - `editorState.timeline.transform` (zoom, position)

**Implementation Plan:**

1. Add `createEffect` in `EditorContextProvider` watching `playbackTime`
2. Check if playhead is outside visible range:
   - `playbackTime < position` (scrolled left of view)
   - `playbackTime > position + zoom` (scrolled right of view)
3. If out of bounds, reposition viewport:
   - `newPosition = playbackTime - (zoom * 0.33)`
   - Clamp to valid range: `max(0, min(newPosition, totalDuration - zoom))`
4. Use `transform.setPosition()` to apply (already handles clamping)

**Edge Cases:**
- Near video start: position clamps to 0, playhead may be closer to left edge
- Near video end: position clamps so viewport doesn't extend past end

---

### S02 - Audio Playback Latency

**Problem:** Video plays immediately when pressing play, but audio has 2-4 second delay. Makes scrubbing/editing difficult.

**Symptoms:**
- Video frame updates immediately
- Audio starts after noticeable delay
- Delay is consistent regardless of audio output device

**Key Files to Investigate:**
- `crates/editor/src/` - playback logic, audio pipeline
- `crates/media/` - audio decoding
- Look for audio buffer preloading, decoder initialization

**Questions to Answer:**
1. Is audio decoder lazily initialized?
2. Is there audio buffering that needs to fill before playback?
3. Can audio be pre-decoded/cached when project opens?
4. Is this a known upstream issue?

---

## 5. Technical Notes

(To be filled during investigation)

---

## 6. Investigation Log

(Use this section to record findings during investigation)

---

## 7. Post-Completion Bug Fix (2026-01-28)

**Critical bug discovered:** After T004, deleting a segment caused audio to go out of sync with video.

**Root cause:** Pre-decoded audio used `segment_count` for cache invalidation, but i/o marker deletion (split+delete) can leave count unchanged while altering content.

**Fix:** Replaced `segment_count` with `timeline_hash` that hashes all segment boundaries with quantized floats.

**Additional optimizations:**
- Partial pre-decode (first 2 min) for fast initial playback
- Quick render (60s) after edits for immediate playback without 20s wait

**Status:** Core fix complete, optimizations in testing.

**See:** `audio-sync-bug-fix.md` for full details, investigation process, and learnings.

**New work needed:**
- Background re-pre-decode after timeline edits
- Extend quick render if playback approaches buffer end
