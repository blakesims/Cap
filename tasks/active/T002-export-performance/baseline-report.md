# T002-S05 Baseline Performance Report

**Date:** 2026-01-15
**Status:** COMPLETE - Bottleneck confirmed, S06 approved

## Test Configuration

| Parameter | Value |
|-----------|-------|
| Mac Model | (User's Mac - details TBD) |
| Resolution | 3840x2160 (4K) |
| Target FPS | 60 |
| Total Frames | 7743 |
| Recording Duration | ~2 min 9 sec |
| Format Conversion | RGBA → NV12 via sws_scale |

## Results Summary

| Metric | Value |
|--------|-------|
| **Total Export Time** | 200,780 ms (3 min 21 sec) |
| **Export FPS** | 38.6 fps |
| **sws_scale Total Time** | 130,150.6 ms |
| **sws_scale Avg Per Frame** | 16,808.8 µs (~16.8 ms) |
| **sws_scale % of Export** | **64.8%** |

## Analysis

### Bottleneck Confirmed

The `sws_scale` CPU-based format conversion consumes **64.8%** of total export time. This far exceeds our 20% threshold for proceeding with S06.

```
Export time breakdown (estimated):
├── sws_scale (CPU)      130.2s  (64.8%)  ← TARGET FOR S06
├── Encoding (HW)         ~40s   (~20%)
├── Decoding (HW)         ~15s   (~7.5%)
├── Rendering (GPU)       ~10s   (~5%)
└── I/O, overhead          ~6s   (~2.7%)
```

### Per-Frame Timing

Sample of per-frame conversion times (every 100 frames):

| Frame | Conversion Time |
|-------|-----------------|
| 0 | 24,612 µs (cold start) |
| 100 | 16,607 µs |
| 500 | 16,453 µs |
| 1000 | 16,216 µs |
| 3000 | 16,419 µs |
| 5000 | 16,437 µs |
| 7000 | 16,517 µs |
| 7700 | 16,987 µs |

Timing is very consistent after the first frame (~16.5ms avg), indicating steady-state CPU-bound work.

### Performance Ceiling Analysis

**Current:** 38.6 fps (limited by sws_scale)

**If sws_scale eliminated:**
- Time without sws_scale: 200,780 - 130,150 = 70,630 ms
- Theoretical FPS: 7743 / 70.63 = **109.6 fps**

**Realistic target with VTPixelTransferSession:**
- VTPixelTransfer adds ~0.5-1ms per frame (HW accelerated)
- Additional time: 7743 * 1ms = ~7.7s
- New total: 70,630 + 7,700 = 78,330 ms
- **Expected FPS: ~99 fps**

However, VideoToolbox encoder may cap at ~55-60 fps for 4K. Realistic target: **50-60 fps**.

## Decision

| Criteria | Threshold | Actual | Decision |
|----------|-----------|--------|----------|
| sws_scale % of export time | >20% | **64.8%** | ✅ PROCEED |
| Expected improvement | +15% | **+30-55%** | ✅ HIGH VALUE |

**APPROVED: Proceed to S06 (VTPixelTransferSession implementation)**

## Potential Improvement

| Scenario | Export FPS | Time for 7743 frames |
|----------|------------|---------------------|
| Current (sws_scale) | 38.6 fps | 3 min 21 sec |
| With VTPixelTransfer | ~55 fps | ~2 min 21 sec |
| Improvement | +42% | -60 sec |

## Log File Reference

Raw log: `cap-export-grepped.log` (same directory)

Key log entries:
```
[T002-S05] Export started: frames=7743 fps=60 resolution=3840x2160
[T002-S05] Format conversion: using sws_scale RGBA -> NV12 resolution=3840x2034
[T002-S05] sws_scale total conversion time: frames=7743 total_sws_ms=130150.6 avg_sws_us=16808.8
[T002-S05] Export complete: frames=7743 duration_ms=200780 fps=38.6
```

## Next Steps

1. **S06**: Implement VTPixelTransferSession for RGBA→NV12
   - Replace sws_scale with Apple's hardware converter
   - Target: 50+ fps export

2. **S07**: Benchmark with same test recording
   - Compare to this baseline
   - Document improvement percentage
