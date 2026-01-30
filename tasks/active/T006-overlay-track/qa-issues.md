# T006 QA Issues

**Reported:** 2026-01-29
**Reporter:** Blake (smoke testing)
**Status:** Partially Resolved

---

## Issue 1: v2.0.0 JSON Import Doesn't Auto-Show Overlay Track

**Status:** ✅ FIXED
**Fix Location:** `apps/desktop/src/routes/editor/Header.tsx`

The import handler now:
1. Auto-enables the overlay track when `overlaySegmentsImported > 0`
2. Updates the toast message to include overlay count

---

## Issue 2: Text Overlapping Bug (CRITICAL)

**Status:** ✅ FIXED
**Root Cause:** In `crates/rendering/src/overlay.rs`, the animation keyframes were hard-coding Y position to 0.5 for ALL items, overriding each item's configured center.y position.

**Fix:** Updated `create_animation_keyframes()` to accept and use the actual `center_y` value for each item.

---

## Issue 3: Text Animation Timing Delay

**Status:** ✅ FIXED
**Root Cause:** The keyframes had an unnecessary delay built into them - animation wasn't starting until `relative_time` after segment start.

**Fix:** Simplified keyframes to start animation immediately at time=0.0 when segment begins.

---

## Issue 4: Title/Bullet Ordering Reversed

**Status:** ✅ FIXED
**Root Cause:** Title was at Y=0.5 (middle), bullets started at Y=0.25 (top), causing bullets to appear above title.

**Fix:**
- Changed `TITLE_Y = 0.20` (top of text area)
- Changed `FIRST_BULLET_Y = 0.40` (below title)
- Track bullet index separately from item index

---

## Issue 5: Split Ratio is 60/40 instead of 50/50

**Status:** ✅ FIXED
**Description:** User requested 50/50 split for overlays.

**Fix Location:** `crates/rendering/src/scene.rs`
- Changed `split_camera_x_ratio()` from 0.6 to 0.5 for SplitScreenRight
- Changed `split_display_x_ratio()` from 0.4 to 0.5 for SplitScreenLeft

**Test frame:** `test-frames/12.0s_5050_split.png`

---

## Issue 6: Overlay Editing Requires Double-Click (Modal Only)

**Status:** ⚠️ OPEN (UX improvement)
**Description:** Other track types have sidebar editing, overlays only have modal editing via double-click.

---

## Test Results

### Test 1: Split Overlay with Title + Bullets

**Test Config:**
- Split overlay from 5.0s to 20.0s
- Title "TEST TITLE" at delay 0.5s
- First bullet at delay 3.0s
- Second bullet at delay 6.0s

**Results:**
| Frame | Time | Expected | Actual | Status |
|-------|------|----------|--------|--------|
| 01 | 3.0s | Normal video | Normal video | ✅ |
| 02 | 5.5s | Split starting | Split layout active | ✅ |
| 03 | 6.0s | Title visible | Title at top | ✅ |
| 04 | 9.0s | Title + bullet 1 | Both visible, correct order | ✅ |
| 05 | 12.0s | All 3 items | All visible, proper stacking | ✅ |
| 06 | 21.0s | Back to normal | Normal video | ✅ |

**Test frames saved to:** `tasks/active/T006-overlay-track/test-frames/test1-v3/`

---

## Files Modified

1. `apps/desktop/src/routes/editor/Header.tsx` - Auto-enable overlay track on import
2. `crates/rendering/src/overlay.rs` - Fixed text positioning and animation timing
3. `crates/rendering/src/scene.rs` - Changed split ratio from 60/40 to 50/50
4. `crates/recording/src/recovery.rs` - Added overlay_segments field
5. `apps/desktop/src-tauri/src/recording.rs` - Added overlay_segments field

---

## Remaining Work

1. **Overlay Sidebar Editing** - Add ConfigSidebar section for overlays (UX improvement)
2. **FullScreen Overlay Test** - Run visual verification test
3. **Multiple Overlays Test** - Run visual verification test
