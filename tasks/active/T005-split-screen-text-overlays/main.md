# Task: T005 - Split-Screen with Animated Text Overlays

## 0. Task Summary
- **Task Name:** Split-Screen with Animated Text Overlays
- **Priority:** 2
- **Number of Stories:** 5
- **Current Status:** ACTIVE
- **Dependencies:** crates/rendering/, crates/project/, apps/desktop/src/routes/editor/
- **Rules Required:** CLAUDE.md
- **Acceptance Criteria:**
  - [x] Split-screen shows camera (40% right) + styled background (60% left)
  - [x] Camera cropped (configurable top/bottom, default 10%) in all modes
  - [x] PiP disabled when in split-screen mode
  - [x] Animated transitions between all layout modes (300ms, matching existing)
  - [x] Text segments support keyframe animation (position, opacity)
  - [ ] Bullet points can appear with staggered timing
  - [ ] JSON import allows LLM-generated timelines to be loaded
  - [ ] Text easily editable after import

## 1. Goal / Objective

Enable a "codified editing style" where recordings can use split-screen layouts with animated text bullet points. An external LLM workflow (`kb edit`) generates JSON describing when to switch layouts and when each text point appears; Cap renders this with smooth animations and allows manual tweaks.

## 2. Overall Status

**ACTIVE** - S01 complete, S02 complete. Ready for S03 (staggered bullet points).

### Session Log (2026-01-27)

1. **S02 TypeScript** - Added keyframe types to `text.ts` ✓
2. **S01 Initial** - Split-screen rendering implemented (needed rework)
3. **Unit Tests** - Added 8 tests for scene.rs split-screen helpers ✓
4. **Bug Fixes** - Fixed pre-existing test compilation errors in avassetreader.rs
5. **S02 Rust Complete** - Implemented keyframe animation backend ✓
6. **Bug Fixes** - Fixed pre-existing clippy errors in video-decode and rendering crates
7. **QA Testing** - Found S01 implementation doesn't match requirements
8. **S01 Rework Complete** ✓
   - Display layer now hidden in split-screen (shows background instead)
   - PiP camera disabled in split-screen mode
   - Added configurable camera crop (`cropTop`, `cropBottom`) - default 10%
   - Crop applies to ALL camera modes (PiP, camera-only, split-screen)

## 3. Stories Breakdown

| Story ID | Story Name / Objective | Status | Deliverable | Estimate |
| :--- | :--- | :--- | :--- | :--- |
| S01 | Add Split-Screen Layout Modes | **DONE** | Camera + styled background (no display) + crop | 1-2 days |
| S02 | Text Keyframe Animation System | **DONE** | TypeScript + Rust types, interpolation, tests | 1-2 days |
| S03 | Staggered Bullet Point Rendering | Planned | Multi-text segments with timed appearance | 0.5 day |
| S04 | JSON Timeline Import | Planned | Import command with validation | 2-3 days |
| S05 | Editor UI Polish | Planned | UI for new modes + keyframe editing | 1-4 days |

**Completed: 2/5 stories (S01, S02 done)**

---

## 4. Story Details

### S01 - Add Split-Screen Layout Modes ✓ COMPLETE

**Status:** DONE - Rework completed, QA verified

#### Corrected Requirements

**What split-screen IS:**
```
┌─────────────────────────────────────────────────────────┐
│                    │                                    │
│   Styled Background│         Camera Feed               │
│   (60% width)      │         (40% width)               │
│   + Text overlays  │         Cropped/zoomed            │
│                    │                                    │
└─────────────────────────────────────────────────────────┘
```

**What split-screen is NOT:**
- ❌ Display + Camera side-by-side
- ❌ PiP camera still visible
- ❌ Screen recording feed shown

#### Specifications

| Aspect | Value |
|--------|-------|
| **Camera position** | Always RIGHT side (40% width) |
| **Text/background** | Always LEFT side (60% width) |
| **Background color** | Dark purple/off-black (`#1a1a2e` or similar) |
| **Camera crop** | Top 10%, bottom 10% cropped (removes black bars) |
| **Camera horizontal** | Center crop (equal left/right) to fill area |
| **PiP in split-screen** | DISABLED (no duplicate camera) |
| **Screen feed** | NOT shown in split-screen |

#### Camera Cropping Detail
```
Original camera feed (with black bars):
┌─────────────────────────────┐
│▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│ ← 10% black (crop)
│                             │
│         Subject             │
│      (center frame)         │
│                             │
│▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│ ← 10% black (crop)
└─────────────────────────────┘

After crop (zoom to fill 40% width area):
┌─────────────────────────────┐
│                             │
│         Subject             │
│      (center frame)         │
│                             │
└─────────────────────────────┘
```

#### S01 Rework Plan (Simplified)

**Analysis found the rework is simpler than expected:**
- Camera cropping already implemented (aspect-ratio based)
- Background config already exists (`BackgroundSource::Color`)
- Only 2 core changes needed

**Phase 1: Skip display layer in split-screen** (REQUIRED)
- File: `crates/rendering/src/lib.rs:2159`
- Add `&& !uniforms.scene.is_split_screen()` to display render condition
- Background already renders full-screen, just skip display overlay

**Phase 2: Disable PiP in split-screen** (REQUIRED)
- File: `crates/rendering/src/scene.rs:412`
- Modify `regular_camera_transition_opacity()` to return 0.0 in split-screen
- Centralizes mode logic in scene helpers

**Phase 3: Test camera cropping** (VERIFY)
- Current aspect-ratio cropping may already handle black bars
- If not sufficient, add fixed 10% vertical crop to `lib.rs:1767-1775`

**Phase 4: Background color** (SKIP)
- Existing `BackgroundConfiguration.source` supports `Color` variant
- No new config needed - users can set background color in editor

**Files to Modify:**
- `crates/rendering/src/lib.rs` - Skip display in split-screen (~1 line)
- `crates/rendering/src/scene.rs` - Disable PiP in split-screen (~3 lines)

#### Previous Implementation (Keep)
- SceneMode enum variants (SplitScreenLeft, SplitScreenRight) ✓
- InterpolatedScene helpers ✓
- UI in SceneTrack.tsx and ConfigSidebar.tsx ✓
- Unit tests for scene.rs ✓

---

### S02 - Text Keyframe Animation System ✓ COMPLETE

**Status:** DONE (verified via unit tests and clippy)

**Implementation Summary:**
- [x] TypeScript types in `apps/desktop/src/routes/editor/text.ts`
- [x] Rust keyframe structs in `crates/project/src/configuration.rs`
  - `TextScalarKeyframe`, `TextVectorKeyframe`, `TextKeyframes`
  - `keyframes` field on `TextSegment` with `#[serde(default)]`
- [x] Rust interpolation in `crates/rendering/src/text.rs`
  - `interpolate_text_vector()` for position animation
  - `interpolate_text_scalar()` for opacity animation
  - `prepare_texts()` uses keyframe interpolation
- [x] Unit tests for interpolation functions

**Files Modified:**
- `crates/project/src/configuration.rs` - Keyframe structs + TextSegment.keyframes field
- `crates/rendering/src/text.rs` - Interpolation functions + prepare_texts integration + tests

---

## 5. S02 Rust Implementation Plan

### Step 1: Add Keyframe Structs to configuration.rs

**Location:** Before `TextSegment` struct (~line 697)

**Code to Add:**
```rust
#[derive(Type, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TextScalarKeyframe {
    pub time: f64,
    pub value: f64,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TextVectorKeyframe {
    pub time: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TextKeyframes {
    #[serde(default)]
    pub position: Vec<TextVectorKeyframe>,
    #[serde(default)]
    pub opacity: Vec<TextScalarKeyframe>,
}
```

### Step 2: Add keyframes field to TextSegment

**Location:** `TextSegment` struct (~line 721, after `fade_duration`)

**Code to Add:**
```rust
#[serde(default)]
pub keyframes: TextKeyframes,
```

### Step 3: Add Interpolation Functions to text.rs

**Location:** After imports, before `prepare_texts` function

**Reference:** Follow pattern from `crates/rendering/src/mask.rs` lines 5-64

**Code to Add:**
```rust
use cap_project::{TextScalarKeyframe, TextVectorKeyframe};

fn interpolate_text_vector(base: XY<f64>, keys: &[TextVectorKeyframe], time: f64) -> XY<f64> {
    if keys.is_empty() {
        return base;
    }

    let mut sorted = keys.to_vec();
    sorted.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));

    if time <= sorted[0].time {
        return XY::new(sorted[0].x, sorted[0].y);
    }

    for i in 0..sorted.len() - 1 {
        let prev = &sorted[i];
        let next = &sorted[i + 1];

        if time >= prev.time && time <= next.time {
            let span = (next.time - prev.time).max(1e-6);
            let t = ((time - prev.time) / span).clamp(0.0, 1.0);
            return XY::new(
                prev.x + (next.x - prev.x) * t,
                prev.y + (next.y - prev.y) * t,
            );
        }
    }

    let last = sorted.last().unwrap();
    XY::new(last.x, last.y)
}

fn interpolate_text_scalar(base: f64, keys: &[TextScalarKeyframe], time: f64) -> f64 {
    if keys.is_empty() {
        return base;
    }

    let mut sorted = keys.to_vec();
    sorted.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));

    if time <= sorted[0].time {
        return sorted[0].value;
    }

    for i in 0..sorted.len() - 1 {
        let prev = &sorted[i];
        let next = &sorted[i + 1];

        if time >= prev.time && time <= next.time {
            let span = (next.time - prev.time).max(1e-6);
            let t = ((time - prev.time) / span).clamp(0.0, 1.0);
            return (prev.value + (next.value - prev.value) * t).clamp(0.0, 1.0);
        }
    }

    sorted.last().unwrap().value.clamp(0.0, 1.0)
}
```

### Step 4: Modify prepare_texts Function

**Location:** `prepare_texts` function in text.rs, around line 55-86

**Changes:**
1. Calculate `relative_time` from segment start
2. Use `interpolate_text_vector` for center position if keyframes exist
3. Use `interpolate_text_scalar` for opacity multiplier
4. Apply keyframe opacity as multiplier with existing fade logic

**Code Changes:**
```rust
// After: let size = XY::new(...)
let relative_time = (frame_time - segment.start).max(0.0);

// Replace: let center = XY::new(segment.center.x.clamp(...), segment.center.y.clamp(...))
let base_center = XY::new(
    segment.center.x.clamp(0.0, 1.0),
    segment.center.y.clamp(0.0, 1.0),
);
let center = interpolate_text_vector(base_center, &segment.keyframes.position, relative_time);
let center = XY::new(center.x.clamp(0.0, 1.0), center.y.clamp(0.0, 1.0));

// Before opacity calculation, get keyframe opacity
let keyframe_opacity = interpolate_text_scalar(1.0, &segment.keyframes.opacity, relative_time);

// Modify opacity calculation to include keyframe_opacity
let opacity = if fade_duration > 0.0 {
    let time_since_start = (frame_time - segment.start).max(0.0);
    let time_until_end = (segment.end - frame_time).max(0.0);
    let fade_in = (time_since_start / fade_duration).min(1.0);
    let fade_out = (time_until_end / fade_duration).min(1.0);
    (fade_in * fade_out * keyframe_opacity) as f32
} else {
    keyframe_opacity as f32
};
```

### Step 5: Add Unit Tests for text.rs

**Location:** End of text.rs file

**Tests to Add:**
- `test_interpolate_text_scalar_empty` - Returns base value
- `test_interpolate_text_scalar_single` - Returns single keyframe value
- `test_interpolate_text_scalar_interpolation` - Linear interpolation works
- `test_interpolate_text_vector_empty` - Returns base position
- `test_interpolate_text_vector_interpolation` - Position interpolation works

---

## 6. Testing Strategy & Evaluation

### Reference Documents
- **Strategy:** [testing-strategy.md](./testing-strategy.md) - 82 test cases across 5 categories
- **Evaluation:** [test-evaluation.md](./test-evaluation.md) - Gap analysis and recommendations

### Current Test Status

| Category | Strategy | Existing | Coverage |
|----------|----------|----------|----------|
| Scalar Interpolation | 23 | 7 | ~30% |
| Vector Interpolation | 16 | 4 | ~25% |
| Color Parsing | 14 | 3 | ~21% |
| Text Preparation | 17 | 9 | ~53% |
| Fade Calculation | 12 | 3 | ~25% |
| **Total** | **82** | **26** | **~35%** |

### Critical Gaps (Must Fix)

| Gap | Risk | Test to Add |
|-----|------|-------------|
| Boundary timing | Text may not appear at exact start/end | `test_text_visible_at_boundaries` |
| Identical keyframe times | Division by zero | `test_identical_keyframe_times` |
| Overlapping fade regions | Incorrect opacity when fade_in + fade_out > duration | `test_overlapping_fade_regions` |
| Horizontal-only position | Only diagonal tested | `test_position_horizontal_only` |
| Color edge cases | Limited coverage | `test_parse_color_black`, `_white`, `_empty` |

### Test Improvement Plan (Subagent Execution)

**File:** `crates/rendering/src/text.rs`
**Target:** Reach 70%+ coverage (add ~15 high-priority tests)

#### Phase 1: Critical Edge Cases (5 tests)
```
1. test_text_visible_at_exact_start_time
   - segment.start = 5.0, query at 5.0 → should return 1 text

2. test_text_visible_at_exact_end_time
   - segment.end = 10.0, query at 10.0 → should return 1 text

3. test_identical_keyframe_times
   - Two keyframes at time=1.0 → should not panic/divide-by-zero

4. test_overlapping_fade_regions
   - segment 0-2s with fade_duration=1.5 → opacity should be valid

5. test_position_horizontal_only
   - Keyframes: (0,0.5) → (1,0.5) → verify Y stays constant
```

#### Phase 2: Interpolation Completeness (5 tests)
```
6. test_interpolate_scalar_decreasing
   - Keyframes 1.0 → 0.0 → verify correct interpolation

7. test_interpolate_scalar_quarter_points
   - Verify t=0.25 and t=0.75 interpolate correctly

8. test_position_vertical_only
   - Keyframes: (0.5,0) → (0.5,1) → verify X stays constant

9. test_zigzag_position_path
   - 3+ keyframes with direction changes

10. test_non_uniform_keyframe_spacing
    - Keyframes at t=0, 0.1, 0.9, 1.0
```

#### Phase 3: Color & Edge Cases (5 tests)
```
11. test_parse_color_black
    - "#000000" → [0.0, 0.0, 0.0, 1.0]

12. test_parse_color_white_uppercase
    - "#FFFFFF" → [1.0, 1.0, 1.0, 1.0]

13. test_parse_color_empty_string
    - "" → default white [1.0, 1.0, 1.0, 1.0]

14. test_multiple_hidden_indices
    - Hide indices [0, 2] from 3 segments → only index 1 visible

15. test_keyframe_opacity_during_fade_out
    - Verify multiplication: keyframe_opacity * fade_out
```

#### Verification Commands
```bash
cargo test -p cap-rendering text::tests
cargo clippy -p cap-rendering -- -D warnings
```

### Manual Testing Checklist

**S01 (Split-Screen):**
- [ ] SplitScreenLeft: camera left 40%, display right 60%
- [ ] SplitScreenRight: camera right 40%, display left 60%
- [ ] Transitions animate smoothly (300ms)
- [ ] No-camera recordings show full-width display

**S02 (Text Keyframes):**
- [ ] Old projects without keyframes load correctly
- [ ] Opacity keyframes animate text visibility
- [ ] Position keyframes animate text movement
- [ ] Keyframes combine with fadeDuration

---

## 7. Remaining Stories (Planned)

### S03 - Staggered Bullet Point Rendering
**Complexity:** Low (documentation only)

No code changes needed. Text segments + keyframes already support this pattern:
```json
{
  "textSegments": [
    { "start": 1.0, "end": 10.0, "content": "Point 1", "keyframes": { "opacity": [{"time": 0, "value": 0}, {"time": 0.5, "value": 1}] }},
    { "start": 2.0, "end": 10.0, "content": "Point 2", "keyframes": { "opacity": [{"time": 0, "value": 0}, {"time": 0.5, "value": 1}] }},
    { "start": 3.0, "end": 10.0, "content": "Point 3", "keyframes": { "opacity": [{"time": 0, "value": 0}, {"time": 0.5, "value": 1}] }}
  ]
}
```
**Deliverable:** Example JSON + documentation for LLM prompt engineering.

### S04 - JSON Timeline Import
**Complexity:** Medium

**Requirements to define:**
1. JSON format LLM will produce (subset of ProjectConfiguration)
2. Merge vs replace behavior for existing segments
3. Validation rules (time ranges, required fields)
4. Error handling UX

**Implementation:**
- Tauri command: `import_timeline_json(path: String, mode: MergeMode)`
- Reuse existing `ProjectConfiguration` serde - just deserialize and merge
- Add `#[serde(default = "fn")]` for any new fields (learned from S01)

### S05 - Editor UI Polish
**Complexity:** Variable (scope TBD)

**Potential features:**
- [ ] Import button in editor toolbar
- [ ] Camera crop sliders in settings (cropTop/cropBottom now exist)
- [ ] Split-screen background color picker
- [ ] Text segment visual editor

**Recommendation:** Start minimal - just add import button. Other UI can follow.

---

## 8. Technical Considerations

### Backwards Compatibility
- `#[serde(default)]` on `TextSegment.keyframes` ensures old projects load
- TypeScript types already have keyframes field
- No migration needed

### Performance
- Keyframe sorting happens per-frame per-segment (acceptable for small counts)
- Linear search through keyframes is O(n) but efficient for typical <10 keyframes

### Type Generation
- After Rust changes, rebuild desktop to regenerate `tauri.ts` types
- TypeScript `text.ts` types should match auto-generated types

### Learnings from S01/S02 Implementation

**1. Validate requirements with QA early**
- S01 initially built display+camera; actual need was camera+background
- Quick manual test after implementation catches architectural mismatches

**2. Leverage existing infrastructure**
- Background already rendered full-screen (no changes needed)
- Camera crop patterns existed (just needed config exposure)
- Check what's already there before building new

**3. Subagent plan review reduces rework**
- Review agent reduced S01 rework from 4 phases → 2 phases
- Run plan review before implementation on S04/S05

**4. Serde defaults need explicit functions**
- `#[serde(default)]` uses type default (0.0 for f32)
- Use `#[serde(default = "Type::fn")]` for custom defaults
- Critical for existing configs that lack new fields

**5. Debug logging accelerates diagnosis**
- `tracing::debug!` immediately revealed config wasn't being read
- Add early when building features that touch config

---

## 9. Files Summary

### S01 Files (DONE)
| File | Status |
|------|--------|
| `crates/project/src/configuration.rs` | ✓ SceneMode enum + camera crop config |
| `crates/rendering/src/scene.rs` | ✓ Helpers + PiP disable in split-screen |
| `crates/rendering/src/lib.rs` | ✓ Split-screen render + camera crop (all modes) |
| `apps/desktop/.../SceneTrack.tsx` | ✓ Icons/labels |
| `apps/desktop/.../ConfigSidebar.tsx` | ✓ Mode selector |

### S02 Files (DONE)
| File | Status |
|------|--------|
| `apps/desktop/.../text.ts` | ✓ TypeScript types |
| `crates/project/src/configuration.rs` | ✓ Rust keyframe structs |
| `crates/rendering/src/text.rs` | ✓ Interpolation + integration + tests |

---

## 10. Next Steps

~~1. **Implement S02 Rust** following the plan in Section 5~~ ✓ DONE
~~2. **Add unit tests** for text.rs interpolation~~ ✓ DONE
~~3. **Verify compilation** with `cargo check -p cap-project -p cap-rendering`~~ ✓ DONE
~~4. **Run all tests** with `cargo test -p cap-rendering`~~ ✓ DONE
~~5. **Code review** of S02 implementation~~ ✓ DONE
~~6. **Update this document** with completion status~~ ✓ DONE

**Remaining:**
1. **Manual testing** of text keyframes via project-config.json
2. **Begin S03** - Staggered Bullet Point Rendering (documentation/pattern)
3. **Begin S04** - JSON Timeline Import (Tauri command)
