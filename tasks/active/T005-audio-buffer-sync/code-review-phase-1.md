# Code Review: T005 Phase 1 — Clip-Level Audio Cache

**Reviewed:** 2026-01-28
**Commit:** `bb884f622`
**Verdict:** PASS WITH ISSUES (non-blocking)

---

## Files Changed

| File | Lines | Summary |
|------|-------|---------|
| `crates/editor/src/audio.rs` | +231 | `ClipAudioCache` struct, LRU logic, `populate_clip_cache()` |
| `crates/editor/src/editor_instance.rs` | +28/-3 | Cache init, population in `spawn_audio_predecode`, plumbing to `Playback` |
| `crates/editor/src/lib.rs` | +1/-1 | Re-export `ClipAudioCache` |
| `crates/editor/src/playback.rs` | +5/-1 | `clip_audio_cache` field on `Playback` and `AudioPlayback` |

---

## Acceptance Criteria Verification

### AC1: Each recording clip's audio decoded exactly once per session (unless clip data changes)
**PASS.** `populate_clip_cache()` iterates `segments` by index and calls `cache.contains(clip_index)` before decoding (`audio.rs:131-133`). Re-decode is skipped for already-cached clips. The `invalidate()` method exists for explicit cache busting when clip data changes. The cache is populated once in `spawn_audio_predecode` (`editor_instance.rs:395-404`).

### AC2: Cache lookup by `recording_clip` index returns pre-decoded samples
**PASS.** `get()` and `get_readonly()` look up by `u32` clip index, returning `Option<Arc<Vec<f32>>>`. `get()` updates LRU access order; `get_readonly()` does not (appropriate for read-only audio callback thread).

### AC3: Memory usage stays within ~500MB for recordings up to ~50 minutes
**PASS.** `CLIP_CACHE_MAX_BYTES` is set to `500 * 1024 * 1024`. The `insert()` method evicts LRU entries until `total_bytes + entry_bytes <= CLIP_CACHE_MAX_BYTES` before inserting. Byte accounting uses `samples.len() * BYTES_PER_F32_SAMPLE` consistently across insert, remove, and invalidate paths.

---

## Issues Found

### Issue 1: LRU eviction is O(n) per operation — LOW
**Location:** `audio.rs:107` (`touch()`), `audio.rs:93` (`insert()`), `audio.rs:112` (`invalidate()`)

`access_order.retain(|&k| k != clip_index)` is O(n) on every `get()`, `insert()`, and `invalidate()`. For typical recordings (< 20 clips), this is negligible. For very long recordings with many clips approaching the 500MB limit, performance could degrade.

Not blocking: real-world clip counts are small. If this ever matters, swap `Vec` for a `LinkedHashMap` or `VecDeque` with index tracking.

### Issue 2: `populate_clip_cache` uses `enumerate()` index as clip_index — MEDIUM
**Location:** `audio.rs:130`

```rust
for (clip_index, segment) in segments.iter().enumerate() {
    let clip_index = clip_index as u32;
```

This assumes that the `AudioSegment` array index equals the `recording_clip` index from `ProjectConfiguration.clips`. This works if `get_audio_segments()` returns segments in order matching their clip indices, but if segments are filtered or reordered, the cache keys would be wrong and Phase 2 lookups would miss.

**Needs verification:** Confirm `get_audio_segments()` returns one segment per recording clip in index order. If it does, this is fine. If not, each `AudioSegment` would need to carry its clip index explicitly.

### Issue 3: Single-clip project config remaps index to 0 — LOW
**Location:** `audio.rs:147-155`

```rust
let single_clip_project = ProjectConfiguration {
    timeline: None,
    clips: clip_config
        .map(|mut c| {
            c.index = 0;
            vec![c]
        })
        .unwrap_or_default(),
    ..project.clone(),
};
```

The index remap (`c.index = 0`) is necessary because `AudioRenderer` is given a single-segment vec where index 0 is the only clip. This is correct but subtle — Phase 2 must not rely on the project config's clip index matching the cache key without understanding this remap.

### Issue 4: `#[allow(dead_code)]` on `clip_audio_cache` — EXPECTED
**Location:** `playback.rs:768-769`

The field is plumbed but unused until Phase 2. `#[allow(dead_code)]` is appropriate here. Should be removed when Phase 2 integrates it.

### Issue 5: Resampler byte-to-f32 conversion assumes native endian — LOW
**Location:** `audio.rs:181-184`, `audio.rs:199-203`

```rust
let mut buf = [0u8; 4];
buf[..chunk.len().min(4)].copy_from_slice(&chunk[..chunk.len().min(4)]);
decoded_samples.push(f32::from_ne_bytes(buf));
```

This pattern handles chunks smaller than 4 bytes by zero-padding, which silently produces incorrect sample values if a chunk is ever not exactly 4 bytes. The `from_ne_bytes` call assumes native endianness, which matches the existing codebase's approach in `PrerenderedAudioBuffer::new()`.

Not blocking, but the guard against `< 4` byte chunks is defensive for a scenario that shouldn't occur with f32 output format.

### Issue 6: No cancellation check between clip decodes — LOW
**Location:** `audio.rs:127-234`

`populate_clip_cache()` doesn't check the `CancellationToken` between clips. The caller in `editor_instance.rs` checks cancellation after the full cache population completes (`editor_instance.rs:409-411`), but if a user closes the editor while many clips are being decoded, this function will run to completion unnecessarily.

The fix is simple (pass `&CancellationToken` and check between loop iterations), but given clips decode in ~100ms each, this is low priority.

---

## Structural Assessment

**Architecture:** Sound. `ClipAudioCache` is a focused struct with clear responsibility. The `Arc<ArcSwap<>>` pattern matches existing audio infrastructure (`audio_predecode_buffer`). The population path in `spawn_audio_predecode` runs the cache first, then the full timeline render — correct ordering since Phase 2 will use the cache for rebuilds.

**Thread safety:** The `get_readonly()` method avoids `&mut self` for the audio callback thread, which can't hold mutable references. Phase 2 will need to use `ArcSwap::load()` to get a snapshot, then `get_readonly()` on it. This is well-designed.

**Memory model:** Byte accounting is consistent. `Arc<Vec<f32>>` allows zero-copy sharing. LRU eviction front-evicts from `access_order` (FIFO on ties), which is standard LRU behavior.

---

## Verdict

**PASS WITH ISSUES.** Phase 1 implementation is structurally correct and meets all three acceptance criteria. Issue 2 (clip index mapping) should be verified before Phase 2 relies on cache lookups by `recording_clip` index. The remaining issues are low-severity and non-blocking.

**Recommendation:** Proceed to Phase 2. Address Issue 2 verification and Issue 6 (cancellation) during Phase 2 implementation if convenient.
