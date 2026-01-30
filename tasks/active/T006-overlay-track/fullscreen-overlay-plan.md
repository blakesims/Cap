# FullScreen Overlay Enhancement Plan

## Objective

Modify the FullScreen overlay type to show:
- **Background** (dark wave wallpaper) instead of screen recording
- **Text** on top of the background
- **PiP camera** in corner (same position and styling as current)

Currently, FullScreen overlay shows: screen recording + PiP camera + text overlay.
The user wants: background + text + PiP camera (no screen recording visible).

## Current Implementation Analysis

### Key Files
- `crates/project/src/configuration.rs` (lines 798-807) - SceneMode enum definition
- `crates/rendering/src/overlay.rs` (lines 29-31) - Maps overlay types to scene modes
- `crates/rendering/src/scene.rs` - Scene interpolation and `get_scene_values()`
- `crates/rendering/src/lib.rs` (lines 2183-2188) - Display rendering condition

### Current Behavior
```rust
// overlay.rs line 30-31
OverlayType::Split => SceneMode::SplitScreenRight,
OverlayType::FullScreen => SceneMode::Default,
```

### Key Insight: How Display Rendering Works
```rust
// lib.rs lines 2183-2184
let should_render = uniforms.scene.should_render_screen() && !uniforms.scene.is_split_screen();

// scene.rs - should_render_screen()
pub fn should_render_screen(&self) -> bool {
    self.screen_opacity > 0.01 || self.screen_blur > 0.01
}
```

Setting `screen_opacity = 0.0` in `get_scene_values()` will automatically:
1. Skip display rendering
2. Skip cursor rendering (correct - cursor shouldn't show on background-only view)
3. Background and PiP camera still render normally

---

## Implementation Plan (Revised)

### Phase 1: Add HideScreen SceneMode and Configure

**Goal:** Add `SceneMode::HideScreen` and wire it up completely.

**Files to modify:**

1. **`crates/project/src/configuration.rs`** - Add `HideScreen` variant:
```rust
pub enum SceneMode {
    Default,
    CameraOnly,
    HideCamera,
    SplitScreenLeft,
    SplitScreenRight,
    HideScreen,  // NEW
}
```

2. **`crates/rendering/src/overlay.rs`** - Change line 31:
```rust
OverlayType::FullScreen => SceneMode::HideScreen,  // Was: SceneMode::Default
```

3. **`crates/rendering/src/scene.rs`** - Multiple changes:

   a. Add `HideScreen` to `same_mode` patterns (3 locations around lines 97, 123, 173):
   ```rust
   // Where SceneMode::Default is grouped with others for "same mode" transitions
   SceneMode::Default | SceneMode::HideCamera | SceneMode::HideScreen => { ... }
   ```

   b. Add `HideScreen` case to `get_scene_values()`:
   ```rust
   SceneMode::HideScreen => (1.0, 0.0, 1.0),  // camera_opacity=1.0, screen_opacity=0.0, scale=1.0
   ```

**Acceptance Criteria:**
- [ ] SceneMode::HideScreen is defined in configuration.rs
- [ ] Serde serialization works (camelCase: "hideScreen")
- [ ] FullScreen overlay maps to HideScreen mode
- [ ] HideScreen added to all 3 same_mode patterns in scene.rs
- [ ] get_scene_values() returns (1.0, 0.0, 1.0) for HideScreen
- [ ] Code compiles without errors
- [ ] Unit tests pass

---

### Phase 2: Testing and Visual Verification

**Goal:** No lib.rs changes needed. Verify the feature works through CLI export.

**Why no lib.rs changes:**
The existing `should_render_screen()` logic automatically skips display and cursor rendering when `screen_opacity = 0.0`.

**Test Cases:**
1. Export video with FullScreen overlay
2. Verify:
   - Background visible (dark wave wallpaper) ✓
   - Screen recording NOT visible ✓
   - PiP camera visible in corner ✓
   - Text visible on background ✓
   - Cursor NOT visible (expected) ✓
   - Transitions in/out are smooth ✓

**Test JSON:**
```json
{
  "version": "2.0.0",
  "overlays": [
    {
      "type": "fullScreen",
      "start": 3.0,
      "end": 12.0,
      "items": [
        { "delay": 0.0, "text": "Chapter Title", "style": "title" },
        { "delay": 1.5, "text": "Key point one", "style": "bullet" },
        { "delay": 3.0, "text": "Key point two", "style": "bullet" }
      ]
    }
  ]
}
```

**Regression Tests:**
- [ ] Split overlay still works correctly
- [ ] Default mode still works correctly
- [ ] CameraOnly mode still works correctly

**Acceptance Criteria:**
- [ ] Visual verification passes
- [ ] No regression in other overlay/scene behaviors
- [ ] Transitions are smooth (opacity fade, no jarring cuts)

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Transition artifacts | Low | Medium | Using opacity=0.0 leverages existing transition system |
| Cursor hidden unexpectedly | N/A | N/A | Expected behavior - cursor tied to display |
| Background rendering issues | Very Low | Medium | Background renders unconditionally |
| TypeScript types mismatch | Very Low | Low | Types auto-generated from Rust |

## Files Summary

| File | Change Type | Lines |
|------|-------------|-------|
| `crates/project/src/configuration.rs` | Add enum variant | ~1 line |
| `crates/rendering/src/overlay.rs` | Change mapping | 1 line |
| `crates/rendering/src/scene.rs` | Add to patterns + get_scene_values | ~10 lines |
| `crates/rendering/src/lib.rs` | NO CHANGES | 0 lines |

Total: ~12 lines of code changes

## Notes from Review

- Cursor not rendering during FullScreen is correct/expected behavior
- TypeScript types in `tauri.ts` auto-update after Rust rebuild
- The `same_mode` patterns are critical for smooth transitions
- Background renders unconditionally, so full-frame coverage is guaranteed
