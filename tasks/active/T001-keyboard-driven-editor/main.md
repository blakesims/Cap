# Task: [T001] - Keyboard-Driven Editor

## 0. Task Summary
-  **Task Name:** Keyboard-Driven Editor
-  **Priority:** 2
-  **Number of Stories:** 6
-  **Current Status:** PLANNING
-  **Platform:** macOS only
-  **Estimated Effort:** 24-31 hours (3-4 days)
-  **Dependencies:** `apps/desktop/src/routes/editor/`, `apps/desktop/src/routes/editor/Timeline/`, `apps/desktop/src/routes/editor/useEditorShortcuts.ts`
-  **Rules Required:** CLAUDE.md (no comments, use existing patterns)
-  **Executor Ref:** See Stories S01-S06
-  **Acceptance Criteria:**
    - All keyboard bindings from prototype work in main editor
    - IN/OUT point system functional with visual feedback
    - Mark system functional
    - Playback speed control (JKL-style) working
    - Frame-stepping and boundary jumping work correctly
    - No regression in existing editor functionality

## 1. Goal / Objective
Implement a comprehensive keyboard-driven editing workflow in the Cap desktop editor, enabling power users to navigate, mark regions, cut, and delete without using the mouse. Based on the working prototype in `keyboard-prototype/`.

**Key Constraint:** This is a fork of an active upstream repo - minimize code changes where possible to ease future syncing.

## 2. Overall Status
Planning phase complete. Prototype validated. Codebase reviewed. Ready for implementation.

## 3. Stories Breakdown

| Story ID | Story Name / Objective | Complexity | Est. Hours | Status | Link |
| :--- | :--- | :--- | :--- | :--- | :--- |
| S01 | State management for IN/OUT points and marks | Low | ~2h | **Done** | [S01-state-management.md](./stories/S01-state-management.md) |
| S02 | Core keyboard bindings infrastructure | Medium | ~4-6h | **Done** | [S02-keyboard-bindings.md](./stories/S02-keyboard-bindings.md) |
| S03 | Playhead navigation (frame step, boundary jump) | Low-Medium | ~3-4h | Planned | Inline |
| S04 | Playback speed control (frontend-only) | Medium | ~4-5h | Planned | Inline |
| S05 | IN/OUT points and marks with visual feedback | Low-Medium | ~3-4h | Planned | Inline |
| S06 | Delete IN/OUT region (phased approach) | High | ~8-10h | Planned | Inline |

## 4. Story Details

### S01 - State Management for IN/OUT Points and Marks
**Complexity: Low (~2h)** | [Detailed Plan](./stories/S01-state-management.md)

-   **Summary:** Add `inPoint`, `outPoint`, `mark` to `editorState` + create `editorActions` object
-   **Upstream Impact:** Minimal - ~50 lines added, 0 modified
-   **Key Decision:** Create new `editorActions` alongside `projectActions` (not mixed in)

### S02 - Core Keyboard Bindings Infrastructure
**Complexity: Medium (~4-6h)**

-   **Acceptance Criteria:**
    -   [ ] Standalone letter keys work (h, l, w, b, etc.)
    -   [ ] Shift modifier support (Shift+h, Shift+l, Shift+4 for $)
    -   [ ] Ctrl modifier support (Ctrl+j, Ctrl+l)
    -   [ ] Bindings disabled when focus is on input/textarea elements
-   **Tasks/Subtasks:**
    -   [ ] Extend `normalizeCombo()` in `useEditorShortcuts.ts` to track Shift modifier
    -   [ ] Handle raw key values without modifiers
    -   [ ] Centralize input focus guard (currently duplicated in Timeline and Player)
    -   [ ] Define all new bindings
-   **Known Findings:**
    -   `c` key already does cut at playhead (no conflict)
    -   Escape currently clears selection - extend to also clear IN/OUT
-   **Bug Fixed (2026-01-14):**
    -   Space bar shortcut was broken by S02 implementation. The `normalizeCombo()` function added a branch for single-character keys (`e.key.length === 1`) to handle case-insensitive letters, but this accidentally intercepted Space (`" ".length === 1`), returning `" "` instead of `"Space"`. Fix: exclude `e.code === "Space"` from that branch.

### S03 - Playhead Navigation
**Complexity: Low-Medium (~3-4h)**

-   **Acceptance Criteria:**
    -   [ ] `h`/`l` step playhead by 1 frame (1/30th second)
    -   [ ] `Shift+h`/`Shift+l` step by 1 second
    -   [ ] `w` jumps to next segment boundary
    -   [ ] `b` jumps to previous segment boundary
    -   [ ] `0` jumps to timeline start
    -   [ ] `$` (Shift+4) jumps to timeline end
-   **Tasks/Subtasks:**
    -   [ ] Implement `stepFrames(count: number)` action
    -   [ ] Implement `stepSeconds(count: number)` action
    -   [ ] Add `segmentBoundaries` computed helper (calculate from `project.timeline.segments`)
    -   [ ] Implement `jumpToNextBoundary()` and `jumpToPrevBoundary()`
    -   [ ] Implement `jumpToStart()` and `jumpToEnd()`
-   **Implementation Note:**
    ```typescript
    // Segment boundary calculation pattern
    const boundaries = segments.reduce((acc, seg, idx) => {
      const prevDuration = segments.slice(0, idx).reduce(
        (sum, s) => sum + (s.end - s.start) / s.timescale, 0
      );
      acc.push(prevDuration);
      acc.push(prevDuration + (seg.end - seg.start) / seg.timescale);
      return acc;
    }, [0]);
    ```

### S04 - Playback Speed Control (Frontend-Only)
**Complexity: Medium (~4-5h)**

**Decision: Frontend-only simulation (do NOT modify Rust playback)**

-   **Acceptance Criteria:**
    -   [ ] `Ctrl+l` plays forward, repeated presses increase speed (1x→2x→4x→8x)
    -   [ ] `Ctrl+j` decreases speed (8x→4x→2x→1x)
    -   [ ] `k` pauses and resets speed to 1x
    -   [ ] Speed indicator visible in UI during playback
    -   [ ] Audio muted at non-1x speeds
-   **Tasks/Subtasks:**
    -   [ ] Add `playbackSpeed: number` to editor state
    -   [ ] When speed != 1x: use frontend `setInterval` to update playhead
    -   [ ] When speed != 1x: do NOT call `commands.startPlayback()` (no Rust playback)
    -   [ ] Trigger preview frame renders via existing events
    -   [ ] Add speed indicator to Player.tsx
    -   [ ] Consider frame skipping at 8x (render every N frames for performance)
-   **Rationale:**
    -   Rust playback is tightly optimized for 1x with complex audio sync
    -   Modifying Rust would be invasive and high-risk
    -   At 2x-8x speeds users are scrubbing, not watching smooth video
    -   Many professional NLEs use this approach

**NOT IN SCOPE:** Reverse playback (video decoders don't handle it efficiently)

### S05 - IN/OUT Points and Marks with Visual Feedback
**Complexity: Low-Medium (~3-4h)**

-   **Acceptance Criteria:**
    -   [ ] `i` sets IN point at playhead
    -   [ ] `o` sets OUT point at playhead
    -   [ ] IN/OUT region highlighted on timeline (shaded area)
    -   [ ] IN/OUT point indicators visible (I/O flags)
    -   [ ] `m` sets mark at playhead
    -   [ ] `'` or `` ` `` jumps to mark
    -   [ ] Mark indicator visible on timeline (M flag)
    -   [ ] `Escape` clears IN/OUT points AND selection (combined behavior)
-   **Tasks/Subtasks:**
    -   [ ] Add IN/OUT region overlay to Timeline/index.tsx (copy pattern from prototype)
    -   [ ] Add IN/OUT point markers (flags above timeline)
    -   [ ] Add mark indicator with distinct styling (purple flag)
    -   [ ] Extend Escape handler to clear both selection and IN/OUT
    -   [ ] Connect keyboard bindings to state actions

### S06 - Delete IN/OUT Region (Phased Approach)
**Complexity: High (~8-10h)**

**Phased implementation to reduce risk:**

#### Phase 1: Basic segment deletion (~3h)
-   [ ] `x` or `Backspace` deletes segment under playhead
-   [ ] Respects existing constraint (must have 2+ segments from same source)
-   [ ] No ripple (gap left behind initially)

#### Phase 2: IN/OUT region deletion (~3h)
-   [ ] If IN/OUT set, `x` deletes content within IN/OUT range
-   [ ] Auto-splits at IN/OUT boundaries if needed
-   [ ] Clears IN/OUT after deletion

#### Phase 3: Ripple delete (~3h)
-   [ ] After deletion, shift subsequent segments earlier to close gap
-   [ ] Adjust zoom/mask/text segments accordingly
-   [ ] Handle edge cases (deletion at start/end of timeline)

#### Separate PR: Sliver fix
-   [ ] Snap cut position to frame boundary: `Math.round(time * FPS) / FPS`
-   [ ] Enforce minimum segment size (e.g., 3 frames / 0.1s)
-   [ ] Keep as separate small PR to isolate risk

-   **Known Constraint:**
    Current `deleteClipSegment()` requires 2+ segments from same recordingSegment. Keep this constraint initially - IN/OUT deletion works around it by splitting first.

## 5. Technical Considerations

### Architecture Decisions
- **State location:** IN/OUT points and marks are session-only state in `editorState` (not persisted)
- **Keyboard handling:** Extend existing `useEditorShortcuts.ts` (don't create new system)
- **Playback speed:** Frontend-only simulation (don't touch Rust)
- **Timeline rendering:** IN/OUT region overlay renders below segments, above track background

### Key Files to Modify
1. `apps/desktop/src/routes/editor/context.ts` - State + actions
2. `apps/desktop/src/routes/editor/useEditorShortcuts.ts` - Keyboard bindings (extend normalizeCombo)
3. `apps/desktop/src/routes/editor/Timeline/index.tsx` - Visual overlays
4. `apps/desktop/src/routes/editor/Player.tsx` - Speed indicator

### Files NOT Modified (minimizing upstream diff)
- `crates/editor/src/playback.rs` - No Rust changes for speed control

### Suggestions for Minimizing Code Changes
1. Extend `useEditorShortcuts.ts` rather than creating new system
2. Copy prototype patterns directly where possible
3. Centralize input focus guard into shared utility
4. Phase S06 to isolate complex ripple logic
5. Keep sliver fix as separate PR

## 6. Resolved Questions

| Question | Decision | Rationale |
| :--- | :--- | :--- |
| Playback speed | Frontend-only | Rust is too complex/risky to modify |
| Reverse playback | Not in scope | Video decoders don't support it efficiently |
| Platform | macOS only | Personal feature, no need for cross-platform |
| Persistence | Session-only | Minimize changes, user didn't need it |
| Segment deletion constraint | Keep existing | IN/OUT deletion works around it |

## 7. Risks and Mitigations

| Risk | Impact | Mitigation |
| :--- | :--- | :--- |
| Upstream merge conflicts | Medium | Keep changes modular, minimize file touches |
| Ripple delete edge cases | High | Phase implementation, test extensively |
| Performance at 8x speed | Low | Add frame skipping if needed |
| State not clearing on project switch | Low | Add explicit reset in switch handler |

## 8. Reference Implementation
Working prototype available at: `keyboard-prototype/`
- Run with: `cd keyboard-prototype && npm run dev`
- Access via: `http://zen:5173/` (Tailscale)
- Demonstrates all keyboard bindings and UI patterns
- Copy patterns directly where applicable
