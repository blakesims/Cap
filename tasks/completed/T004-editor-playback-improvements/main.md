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
    - Timeline auto-scrolls to keep playhead visible during navigation/playback
    - ✅ Audio playback starts immediately when pressing play (reduced from 7s to ~120ms)

## 1. Goal / Objective
Fix two editor UX issues: (1) timeline doesn't follow the playhead, requiring manual scrolling; (2) audio has significant latency on playback start while video plays immediately.

## 2. Overall Status
**S02 COMPLETED (2026-01-26)**: Audio latency reduced from ~7 seconds to ~120ms (98% improvement) via background pre-decoding at output device sample rate.

S01 remains planned for future work.

---

## 3. Stories Breakdown

| Story ID | Story Name / Objective | Complexity | Est. Hours | Status |
| :--- | :--- | :--- | :--- | :--- |
| S01 | Auto-scroll timeline to keep playhead visible | Medium | TBD | Planned |
| S02 | Fix audio playback latency | High | 8 | ✅ COMPLETED |

---

## 4. Story Details

### S01 - Auto-Scroll Timeline

**Problem:** When navigating with keyboard (w/b, h/l) or during playback, the playhead can move outside the visible viewport. User must manually scroll.

**Desired Behavior:**
- When playhead exits visible viewport, auto-scroll so playhead is ~1/3 from left edge
- Should work during: keyboard navigation, playback, segment jumps

**Key Files to Investigate:**
- `apps/desktop/src/routes/editor/context.ts` - `editorState.timeline.transform` (zoom, position)
- `apps/desktop/src/routes/editor/Timeline/index.tsx` - viewport calculations

**Approach (estimated):**
- Add `createEffect` watching `playbackTime`
- Calculate if out of bounds: `playbackTime < position || playbackTime > position + zoom`
- If so, set `position = playbackTime - (zoom * 0.33)`

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
