# Testing Strategy: Text Keyframe Animation System

## Overview

This document outlines a comprehensive testing strategy for Cap's text keyframe animation system. The system enables text overlays on video with keyframe-based animation for position and opacity, fade effects, and support for multiple overlapping text segments.

## 1. Unit Test Categories

### 1.1 Scalar Interpolation (Opacity)

Tests for the function that interpolates single numeric values (opacity) between keyframes over time.

### 1.2 Vector Interpolation (Position)

Tests for the function that interpolates 2D coordinates (x, y) between keyframes over time.

### 1.3 Color Parsing

Tests for converting hex color strings to RGBA float arrays.

### 1.4 Text Preparation

Tests for the function that transforms text segment configurations into renderable text objects with computed properties.

### 1.5 Fade Effect Calculation

Tests for fade in/out effect calculations and their combination with keyframe opacity.

---

## 2. Test Cases for Each Category

### 2.1 Scalar Interpolation (Opacity)

#### 2.1.1 Basic Interpolation

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `interpolate_midpoint_between_two_keyframes` | Keyframes at t=0 (v=0.0) and t=1.0 (v=1.0), query t=0.5 | Returns 0.5 |
| `interpolate_quarter_point` | Keyframes at t=0 (v=0.0) and t=1.0 (v=1.0), query t=0.25 | Returns 0.25 |
| `interpolate_three_quarters_point` | Keyframes at t=0 (v=0.0) and t=1.0 (v=1.0), query t=0.75 | Returns 0.75 |
| `interpolate_decreasing_values` | Keyframes at t=0 (v=1.0) and t=1.0 (v=0.0), query t=0.5 | Returns 0.5 |
| `interpolate_partial_range` | Keyframes at t=0 (v=0.2) and t=1.0 (v=0.8), query t=0.5 | Returns 0.5 |

#### 2.1.2 Exact Keyframe Hits

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `returns_exact_value_at_first_keyframe` | Keyframes at t=0, t=1.0, query t=0 | Returns first keyframe value exactly |
| `returns_exact_value_at_last_keyframe` | Keyframes at t=0, t=1.0, query t=1.0 | Returns last keyframe value exactly |
| `returns_exact_value_at_middle_keyframe` | Keyframes at t=0, t=0.5, t=1.0, query t=0.5 | Returns middle keyframe value exactly |

#### 2.1.3 Multiple Keyframes

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `interpolate_between_first_two_of_three` | Keyframes at t=0, t=0.5, t=1.0, query t=0.25 | Interpolates between first and second keyframe |
| `interpolate_between_last_two_of_three` | Keyframes at t=0, t=0.5, t=1.0, query t=0.75 | Interpolates between second and third keyframe |
| `interpolate_with_many_keyframes` | 10+ keyframes, query various times | Returns correct interpolated values |
| `handles_non_uniform_keyframe_spacing` | Keyframes at t=0, t=0.1, t=0.9, t=1.0 | Correctly identifies which segment to interpolate |

#### 2.1.4 Edge Cases

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `single_keyframe_returns_that_value` | Single keyframe at t=0.5 (v=0.7), query any time | Returns 0.7 |
| `query_before_first_keyframe` | Keyframes start at t=0.5, query t=0.2 | Returns first keyframe value (clamp) |
| `query_after_last_keyframe` | Keyframes end at t=0.5, query t=0.8 | Returns last keyframe value (clamp) |
| `empty_keyframes_returns_default` | Empty keyframe array | Returns default value (likely 1.0 for opacity) |
| `identical_keyframe_times` | Two keyframes at same time t=0.5 | Does not crash, returns reasonable value |
| `very_close_keyframe_times` | Keyframes at t=0.5 and t=0.5000001 | No division by zero, returns stable value |

#### 2.1.5 Boundary Values

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `opacity_at_zero_boundary` | Keyframes with v=0.0 | Returns exactly 0.0 |
| `opacity_at_one_boundary` | Keyframes with v=1.0 | Returns exactly 1.0 |
| `opacity_never_exceeds_bounds` | Verify across all interpolation scenarios | Result always in [0.0, 1.0] |
| `very_small_time_values` | Keyframes at t=0.0001, t=0.0002 | Handles precision correctly |
| `very_large_time_values` | Keyframes at t=10000.0, t=10001.0 | Handles large values correctly |

---

### 2.2 Vector Interpolation (Position)

#### 2.2.1 Basic Interpolation

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `interpolate_position_midpoint` | Keyframes (0,0) at t=0, (100,100) at t=1, query t=0.5 | Returns (50, 50) |
| `interpolate_position_horizontal_only` | Keyframes (0,50) at t=0, (100,50) at t=1, query t=0.5 | Returns (50, 50) |
| `interpolate_position_vertical_only` | Keyframes (50,0) at t=0, (50,100) at t=1, query t=0.5 | Returns (50, 50) |
| `interpolate_position_diagonal_movement` | Various diagonal paths | Returns correct interpolated position |
| `interpolate_position_negative_coords` | Keyframes with negative x or y | Handles negative values correctly |

#### 2.2.2 Multiple Position Keyframes

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `zigzag_path_interpolation` | Position moving right, then left, then right | Follows correct path segment |
| `stationary_then_moving` | Same position for first two keyframes, different third | Stays still then moves |
| `moving_then_stationary` | Different positions, then same for last two | Moves then stays still |

#### 2.2.3 Edge Cases

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `single_position_keyframe` | One keyframe at (50, 50) | Returns (50, 50) for any time |
| `empty_position_keyframes` | No keyframes | Returns default position |
| `query_position_before_keyframes` | Query time before first keyframe | Returns first position (clamp) |
| `query_position_after_keyframes` | Query time after last keyframe | Returns last position (clamp) |

#### 2.2.4 Coordinate System Edge Cases

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `position_at_zero_zero` | Keyframes including (0, 0) | Handles origin correctly |
| `position_at_negative_coords` | Keyframes with x=-50, y=-50 | No issues with negative positions |
| `position_very_large_coords` | Keyframes at (10000, 10000) | Handles large coordinates |
| `position_fractional_coords` | Keyframes at (50.5, 75.25) | Preserves decimal precision |

---

### 2.3 Color Parsing

#### 2.3.1 Standard Formats

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `parse_six_digit_hex_black` | "#000000" | Returns [0.0, 0.0, 0.0, 1.0] |
| `parse_six_digit_hex_white` | "#FFFFFF" | Returns [1.0, 1.0, 1.0, 1.0] |
| `parse_six_digit_hex_red` | "#FF0000" | Returns [1.0, 0.0, 0.0, 1.0] |
| `parse_six_digit_hex_green` | "#00FF00" | Returns [0.0, 1.0, 0.0, 1.0] |
| `parse_six_digit_hex_blue` | "#0000FF" | Returns [0.0, 0.0, 1.0, 1.0] |
| `parse_mixed_color` | "#8B5CF6" | Returns correct RGB values |

#### 2.3.2 Case Insensitivity

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `parse_lowercase_hex` | "#ff0000" | Returns same as "#FF0000" |
| `parse_mixed_case_hex` | "#fF00Ff" | Parses correctly |

#### 2.3.3 Alternative Formats

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `parse_three_digit_hex` | "#F00" | Returns [1.0, 0.0, 0.0, 1.0] (if supported) |
| `parse_eight_digit_hex_with_alpha` | "#FF0000FF" | Returns [1.0, 0.0, 0.0, 1.0] (if supported) |
| `parse_without_hash_prefix` | "FF0000" | Handles gracefully (parse or error) |

#### 2.3.4 Edge Cases and Invalid Input

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `empty_string_color` | "" | Returns default color or error |
| `invalid_hex_characters` | "#GGGGGG" | Returns default color or error |
| `too_short_hex` | "#FFF0" | Returns default color or error |
| `too_long_hex` | "#FFFFFFFFFF" | Returns default color or error |
| `null_or_none_color` | null/None input | Returns default color |
| `whitespace_in_color` | " #FF0000 " | Trims whitespace or errors gracefully |

---

### 2.4 Text Preparation

#### 2.4.1 Basic Text Rendering

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `prepare_single_visible_text` | One text segment, time within its bounds | Returns prepared text with correct properties |
| `prepare_text_with_default_values` | Minimal text config (only required fields) | Uses sensible defaults for all optional properties |
| `prepare_text_respects_font_size` | Text with explicit font size | Output uses specified font size |
| `prepare_text_respects_position` | Text with explicit x, y position | Output positioned correctly |

#### 2.4.2 Visibility and Timing

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `text_not_visible_before_start` | Query time before segment start | Text not in output |
| `text_not_visible_after_end` | Query time after segment end | Text not in output |
| `text_visible_at_start_boundary` | Query time exactly at start | Text is visible |
| `text_visible_at_end_boundary` | Query time exactly at end | Text is visible |
| `text_visible_throughout_duration` | Query various times within bounds | Text always visible |

#### 2.4.3 Multiple Text Segments

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `multiple_non_overlapping_texts` | Three texts at different time ranges | Only correct text visible at each time |
| `multiple_overlapping_texts` | Two texts visible at same time | Both texts in output |
| `all_texts_visible_simultaneously` | All texts share same time range | All returned with correct z-order |
| `texts_in_sequence` | Text A then B then C with no overlap | Correct text for each time query |

#### 2.4.4 Hidden Indices

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `hidden_index_excludes_text` | Text at index 0, hidden_indices contains 0 | Text not in output |
| `multiple_hidden_indices` | Several texts, multiple hidden | Only non-hidden visible |
| `empty_hidden_indices` | No hidden indices | All matching texts visible |
| `hidden_index_out_of_range` | hidden_indices contains index 999 | No crash, ignored gracefully |

#### 2.4.5 Output Size Handling

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `position_scaled_to_output_size` | Position as percentage, various output sizes | Absolute position scales correctly |
| `text_bounds_within_output` | Large text near edge | Bounds calculated correctly |
| `zero_output_size` | Output size (0, 0) | Handles gracefully (no crash) |
| `very_large_output_size` | Output size (4096, 2160) | Handles large dimensions |

---

### 2.5 Fade Effect Calculation

#### 2.5.1 Fade In

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `fade_in_at_start` | Fade duration 0.5s, query t=0 | Opacity 0.0 |
| `fade_in_midpoint` | Fade duration 0.5s, query t=0.25 | Opacity 0.5 |
| `fade_in_complete` | Fade duration 0.5s, query t=0.5 | Opacity 1.0 |
| `fade_in_after_complete` | Fade duration 0.5s, query t=1.0 | Opacity 1.0 |
| `no_fade_in` | Fade duration 0, query t=0 | Opacity 1.0 immediately |

#### 2.5.2 Fade Out

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `fade_out_before_start` | Segment ends at 10s, fade 0.5s, query t=9.0 | Opacity 1.0 |
| `fade_out_midpoint` | Segment ends at 10s, fade 0.5s, query t=9.75 | Opacity 0.5 |
| `fade_out_complete` | Segment ends at 10s, fade 0.5s, query t=10.0 | Opacity 0.0 |
| `no_fade_out` | Fade out duration 0 | Opacity stays at 1.0 until end |

#### 2.5.3 Combined Fade In and Out

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `fade_in_and_out_short_segment` | 1s segment with 0.3s fade in/out | Correct opacity progression |
| `overlapping_fade_regions` | Fade in + out durations exceed segment length | Handles gracefully |
| `fade_with_keyframe_opacity` | Fade + keyframe that sets opacity to 0.5 | Values multiply correctly |

#### 2.5.4 Fade with Keyframe Opacity Interaction

| Test Name | Input Conditions | Expected Behavior |
|-----------|------------------|-------------------|
| `keyframe_opacity_during_fade_in` | Keyframe opacity 0.5, during fade in | Result is fade_factor * keyframe_opacity |
| `keyframe_opacity_during_fade_out` | Keyframe opacity 0.8, during fade out | Result is fade_factor * keyframe_opacity |
| `keyframe_zero_opacity_with_fade` | Keyframe opacity 0.0, any fade state | Result is 0.0 |

---

## 3. Integration Test Scenarios

### 3.1 Full Text Animation Lifecycle

**Scenario**: Text appears, animates position, fades out

1. Create text segment starting at t=2.0, ending at t=8.0
2. Configure fade in duration 0.5s, fade out duration 1.0s
3. Add position keyframes: (100, 100) at t=2.0, (500, 300) at t=5.0, (800, 100) at t=8.0
4. Add opacity keyframes: 1.0 at t=2.0, 0.5 at t=5.0, 1.0 at t=8.0
5. Query at multiple times and verify:
   - t=1.9: Text not visible
   - t=2.0: Text at (100, 100), opacity affected by fade in start
   - t=2.5: Text partway between positions, full fade in complete
   - t=5.0: Text at (500, 300), opacity 0.5
   - t=7.0: Text between (500, 300) and (800, 100), fade out starting
   - t=8.0: Text at (800, 100), opacity near 0 due to fade out
   - t=8.1: Text not visible

### 3.2 Multiple Overlapping Animated Texts

**Scenario**: Two text overlays with different animations running simultaneously

1. Text A: t=0-10s, moves left to right, constant opacity
2. Text B: t=5-15s, moves top to bottom, pulsing opacity via keyframes
3. Query at t=7.5 and verify both texts present with correct positions/opacity

### 3.3 Rapid Keyframe Changes

**Scenario**: Many keyframes in quick succession

1. Create text with 20 keyframes over 1 second
2. Query at 30fps (33ms intervals)
3. Verify smooth interpolation without jumps or artifacts

### 3.4 Editor Preview vs Export Consistency

**Scenario**: Ensure same calculations during preview and export

1. Create complex text animation
2. Generate frames at preview frame rate
3. Generate frames at export frame rate
4. Verify position/opacity values match at equivalent times

---

## 4. Edge Cases and Boundary Conditions

### 4.1 Empty and Null Inputs

| Condition | Expected Behavior |
|-----------|-------------------|
| Empty text segments array | Returns empty prepared texts |
| Text segment with empty content string | Handles gracefully (skip or render empty) |
| Null/undefined optional properties | Uses default values |
| Empty keyframe arrays | Uses default position/opacity |

### 4.2 Single Element Inputs

| Condition | Expected Behavior |
|-----------|-------------------|
| Single text segment | Works correctly |
| Single keyframe for position | Returns that position for all times |
| Single keyframe for opacity | Returns that opacity for all times |

### 4.3 Boundary Values

| Condition | Expected Behavior |
|-----------|-------------------|
| Opacity exactly 0.0 | Renders as fully transparent |
| Opacity exactly 1.0 | Renders as fully opaque |
| Position at (0, 0) | Correctly positioned at origin |
| Time exactly 0.0 | Handled correctly |
| Segment with zero duration | Handles gracefully (never visible or brief flash) |

### 4.4 Out-of-Order Data

| Condition | Expected Behavior |
|-----------|-------------------|
| Keyframes not sorted by time | Either sorts internally or documents requirement |
| Text segments not sorted by start time | Works regardless of order |

### 4.5 Overlapping Time Ranges

| Condition | Expected Behavior |
|-----------|-------------------|
| Multiple texts at exact same time range | All rendered with consistent z-order |
| Keyframes at identical timestamps | Does not crash, picks one or averages |

### 4.6 Division by Zero Scenarios

| Condition | Expected Behavior |
|-----------|-------------------|
| Two keyframes at same time | No division by zero in interpolation |
| Zero-duration fade | No division by zero |
| Zero-width or zero-height output | Handles gracefully |

### 4.7 Invalid/Malformed Inputs

| Condition | Expected Behavior |
|-----------|-------------------|
| Negative time values in keyframes | Handles gracefully |
| Opacity values outside [0, 1] | Clamps or rejects |
| NaN or Infinity in numeric fields | Does not propagate, uses default or errors |
| Invalid font name | Falls back to default font |
| Negative font size | Uses minimum size or errors |

---

## 5. Test Quality Criteria

### 5.1 Coverage Requirements

- **Line Coverage**: Minimum 90% for interpolation functions
- **Branch Coverage**: Minimum 85% for all conditional logic
- **Edge Case Coverage**: 100% of documented edge cases must have explicit tests

### 5.2 Test Independence

- Each test must be runnable in isolation
- Tests must not depend on execution order
- Tests must clean up any state they create

### 5.3 Test Clarity

- Test names must clearly describe the scenario being tested
- Test failures must produce actionable error messages
- Complex assertions must include explanatory comments (in test description, not code)

### 5.4 Performance Criteria

- Interpolation functions must complete in < 1ms for typical inputs
- Text preparation for 10 segments must complete in < 10ms
- Test suite must complete in < 30 seconds

### 5.5 Determinism

- All tests must be deterministic (same input = same output)
- No reliance on system time for business logic tests
- Random values only for fuzz testing with fixed seeds

### 5.6 Test Organization

```
tests/
  unit/
    interpolation/
      scalar_interpolation_tests
      vector_interpolation_tests
    color_parsing_tests
    text_preparation_tests
    fade_calculation_tests
  integration/
    animation_lifecycle_tests
    multi_text_animation_tests
  property/
    interpolation_property_tests
```

### 5.7 Test Documentation

Each test file should include:
- Brief description of the component under test
- List of assumptions being tested
- Known limitations or excluded scenarios

### 5.8 Regression Testing

- Any bug fix must include a regression test
- Regression tests must reference the issue/bug they prevent
- Regression tests should be minimal (test the specific fix)

---

## 6. Testing Tools and Infrastructure

### 6.1 Recommended Test Types

| Type | Purpose | When to Use |
|------|---------|-------------|
| Unit Tests | Test individual functions in isolation | All pure functions |
| Integration Tests | Test component interactions | Animation pipelines |
| Property-Based Tests | Find edge cases automatically | Interpolation functions |
| Snapshot Tests | Detect unintended changes | Prepared text output |
| Benchmark Tests | Ensure performance | Interpolation hot paths |

### 6.2 Property-Based Testing Opportunities

The interpolation system is ideal for property-based testing:

1. **Monotonicity**: For monotonic keyframe values, interpolated values should be monotonic
2. **Boundary Clamping**: Output always within valid range for opacity
3. **Continuity**: Small changes in input time produce small changes in output
4. **Idempotency**: Querying at keyframe time returns exact keyframe value

### 6.3 Test Data Generation

Maintain test fixtures for:
- Standard text configurations
- Complex keyframe sequences
- Edge case scenarios
- Performance benchmark datasets

---

## 7. Risk-Based Test Prioritization

### 7.1 High Priority (Must Test First)

1. Keyframe interpolation accuracy
2. Fade effect calculations
3. Visibility timing boundaries
4. Opacity/position combination with fades

### 7.2 Medium Priority

1. Multiple text segment handling
2. Hidden index filtering
3. Output size scaling
4. Color parsing

### 7.3 Lower Priority

1. Error handling for invalid inputs
2. Performance edge cases
3. Unusual coordinate values

---

## 8. Test Maintenance Guidelines

### 8.1 When to Update Tests

- New feature added: Add corresponding tests
- Bug fixed: Add regression test
- Behavior changed intentionally: Update affected tests
- Test flaky: Fix root cause, do not disable

### 8.2 Test Review Checklist

- [ ] Tests cover happy path
- [ ] Tests cover documented edge cases
- [ ] Tests have clear names
- [ ] Tests are independent
- [ ] Tests run quickly
- [ ] Test failures are actionable

### 8.3 Continuous Integration

- All tests must pass before merge
- New tests must not reduce coverage
- Performance tests should alert on regression
