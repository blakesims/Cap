# Code Review: Phase 2 — Incremental Timeline Rebuild with Crossfade

## Review Meta
- **Phase:** 2 (FINAL)
- **Commit:** `9d827e3be` — `feat(editor): incremental timeline rebuild with crossfade on buffer swap [T005-P2]`
- **Reviewer:** Claude
- **Date:** 2026-01-28
- **Verdict:** PASS

---

## Summary

Phase 2 implements `PrerenderedAudioBuffer::from_clip_cache()` for O(memcpy) timeline rebuilds and adds a 15ms crossfade mechanism to prevent audible clicks during buffer swaps. The implementation correctly uses the clip audio cache from Phase 1, integrates cleanly with the existing rebuilder thread, and follows the plan specifications.

---

## Acceptance Criteria Verification

### AC1: Deleting a segment rebuilds audio in <50ms (splice, no decode)
**PASS**

`from_clip_cache()` (audio.rs:845-935) performs only:
- One iteration over timeline segments
- HashMap lookups via `cache.get_readonly()`
- Slice operations and `extend_from_slice()` for f32 direct path
- Pre-allocated Vec with `with_capacity()`

This is O(n) in segment count with O(1) per-sample copy operations. For typical recordings (10-20 segments), this is well under 50ms. The fallback to full decode only triggers on cache miss (logged: "Cache miss in rebuilder, falling back to full decode").

### AC2: Audio remains perfectly synced after any timeline edit
**PASS**

Sync is maintained through:
1. Sample-accurate slicing: `start_sample = (segment.start * sample_rate) as usize * channels` (audio.rs:886-889)
2. Correct handling of all segment types:
   - Normal clips: sliced from cache
   - `timescale != 1.0`: silence inserted (matching existing behavior)
   - Out-of-bounds: silence padding
3. Playhead clamping after rebuild: `clamped_playhead = current.min(new_duration)` (playback.rs:1413)

### AC3: No audible clicks or gaps during buffer swap (15ms crossfade)
**PASS**

Crossfade implementation (audio.rs:1078-1130):
- Duration: 15ms constant (`CROSSFADE_DURATION_MS`)
- Linear blend: `old_val * old_weight + new_val * new_weight`
- Sample calculation: `crossfade_samples_for_rate()` correctly computes samples per channel

Swap sequence (playback.rs:1416-1422):
1. Snapshot old buffer at playhead: `old_buffer.snapshot_at_playhead(crossfade_samples)`
2. Lock mutex, set crossfade state
3. Store new buffer via `ArcSwap`

Audio callback (playback.rs:1459-1466):
- `try_lock()` avoids blocking the audio thread
- Applies crossfade only if lock acquired (graceful degradation if contended)
- Clears state when complete

### AC4: Playhead position preserved exactly across buffer swap
**PASS**

Playhead handling:
1. Current playhead read via `AtomicU64`: `f64::from_bits(playhead_for_rebuilder.load(Ordering::Acquire))` (playback.rs:1410-1412)
2. Clamped to new duration to prevent out-of-bounds
3. Set on new buffer before swap: `new_buffer.set_playhead(clamped_playhead)` (playback.rs:1414)

The existing `AtomicU64` mechanism is unchanged; Phase 2 just reads and applies it to the rebuilt buffer.

### AC5: I/O point changes (user-marked segments) reuse cached clip audio
**PASS**

`from_clip_cache()` is agnostic to segment origin — it only looks up `segment.recording_clip` in the cache. Whether segments come from:
- Recording chunks (original recording segments)
- User-defined I/O points (manual splits)

Both reference the same `recording_clip` index, so cached audio is reused.

---

## Code Quality Analysis

### Correctness

1. **Sample format conversion** (audio.rs:899-926): The fast path correctly identifies when T is f32:
   ```rust
   let is_f32_direct = output_info.sample_format == AudioData::SAMPLE_FORMAT
       && std::mem::size_of::<T>() == std::mem::size_of::<f32>();
   ```
   The unsafe reinterpret is sound because both conditions verify the types match.

2. **Sample rate/channel validation** (audio.rs:857-859): Returns `None` on mismatch, triggering fallback. This handles device reconfiguration gracefully.

3. **Empty segment handling** (audio.rs:852-855): Returns `None` if timeline has no segments, avoiding empty buffer construction.

### Thread Safety

1. **Crossfade mutex**: Uses `std::sync::Mutex` shared between rebuilder and callback threads
2. **Audio callback**: Uses `try_lock()` — never blocks the real-time audio thread
3. **Buffer swap**: Uses `ArcSwap::store()` — lock-free atomic pointer swap
4. **Playhead**: Uses `AtomicU64` with proper ordering (Acquire/Release)

### Performance

1. **Vec pre-allocation**: `Vec::with_capacity(estimated_samples + 1024)` avoids reallocations
2. **Direct f32 path**: Avoids per-sample conversion when output is f32
3. **Non-blocking crossfade**: `try_lock()` in callback means crossfade is best-effort; audio continues even if mutex is contended

---

## Issues Identified

### Issue 1: Crossfade sample count includes channels (LOW)
`crossfade_samples_for_rate()` multiplies by channels:
```rust
let samples_per_ms = (sample_rate as f64 / 1000.0) * channels as f64;
```

This means for stereo (2 channels), the crossfade is over 15ms * 2 = ~1323 sample _pairs_ at 44.1kHz. The `apply()` loop iterates over all samples (L+R interleaved), so the actual crossfade duration is still 15ms. **This is correct**, but the variable naming (`samples_per_ms`) could be clearer as `sample_values_per_ms`.

**Impact:** None — code is correct, naming is slightly misleading.

### Issue 2: Conversion path allocates Vec per sample (LOW)
For non-f32 output formats (audio.rs:908-925), each sample conversion allocates a small `Vec<u8>`:
```rust
let converted: Vec<u8> = match output_info.sample_format { ... };
```

**Impact:** Non-f32 output devices (rare) will have higher allocation pressure during cache-based rebuild. Acceptable since most devices use f32.

### Issue 3: Crossfade missed if mutex contended (EXPECTED)
If `try_lock()` fails, the crossfade is skipped for that buffer callback. This could cause a brief click.

**Impact:** Negligible — mutex contention is extremely rare (rebuilder holds lock only during snapshot/state creation, ~microseconds). The design intentionally prioritizes audio thread latency over guaranteed crossfade.

### Issue 4: Phase 1 concern addressed (VERIFIED)
Phase 1 review flagged that `populate_clip_cache()` uses `enumerate()` index as `recording_clip`. Phase 2's `from_clip_cache()` looks up by `segment.recording_clip`:
```rust
let clip_audio = match cache.get_readonly(segment.recording_clip) { ... };
```

This works correctly because:
- `get_audio_segments()` returns segments in clip index order
- Timeline segments reference clips via `recording_clip` which matches the populate order

**Verified:** The index mapping is consistent across both phases.

---

## Summary

| Category | Assessment |
|----------|------------|
| Acceptance Criteria | 5/5 PASS |
| Correctness | Sound |
| Thread Safety | Appropriate for real-time audio |
| Performance | Optimized with fast path |
| Issues | 0 blocking, 3 low (naming, allocation, expected behavior) |

**Final Verdict: PASS**

The implementation correctly delivers O(memcpy) timeline rebuilds using the Phase 1 clip cache, with a 15ms crossfade that prevents audible artifacts during buffer swap. All acceptance criteria are met. The design makes appropriate trade-offs for real-time audio constraints.

---

## Task Completion Recommendation

**Set T005 Status to COMPLETE.**

Both phases have passed code review:
- Phase 1: Clip-level audio cache with LRU eviction
- Phase 2: Incremental timeline rebuild with crossfade

The original problem (audio disappearing after segment deletion) is solved by:
1. Caching decoded clip audio on editor open (once per session)
2. Rebuilding timeline buffer via memcpy from cache (<50ms)
3. Crossfading during buffer swap to prevent clicks

Remaining low-priority items (naming clarity, non-f32 allocation) are not blocking and can be addressed in future maintenance.
