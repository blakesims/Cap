# Plan Review: T005 Audio Buffer Sync (Re-Review)

**Reviewed:** 2026-01-28
**Verdict:** APPROVED

---

## Executive Summary

The revised plan addresses all four issues raised in the initial review. The architecture is now correctly structured around clip-level caching with incremental timeline splicing, reduced to 2 focused phases, and the unnecessary complexity has been eliminated.

---

## Issue Resolution

### 1. Root Cause Misidentified → RESOLVED
The plan now includes a clear Root Cause Analysis section identifying the bottleneck at `playback.rs:1379-1384` where `PrerenderedAudioBuffer::new()` re-decodes the entire timeline on every change. The distinction between decode cost (expensive) and splice cost (cheap) is explicit.

### 2. Segment vs Clip Confusion → RESOLVED
The plan clearly distinguishes:
- **Recording clips** (`SegmentMedia`): Indexed by `recording_clip` (u32), decoded once per session
- **Timeline segments** (`TimelineConfiguration.segments`): User-defined slices referencing clips via `recording_clip`

Cache key is now `recording_clip` index (u32), not a composite of timeline positions. Cache survives all timeline edits since clips don't change when segments are added/removed/split. This is the correct architecture.

### 3. Progressive Loading Overengineered → RESOLVED
Progressive loading has been eliminated entirely. The rationale is sound: clip decode is ~100ms per clip (one-time), splice rebuild is <50ms. The priority queue complexity is gone.

### 4. Crossfade Duration Too Long → RESOLVED
Reduced from 50-100ms to 15ms. Within the recommended 10-20ms range. Implementation is well-specified: linear fade stored alongside the `ArcSwap`, blending last/first 15ms at the playhead position during buffer swap.

---

## Technical Assessment

### Phase 1: Clip-Level Audio Cache
- `ClipAudioCache` as `HashMap<u32, Arc<Vec<f32>>>` — appropriate data structure. `Arc` allows zero-copy sharing between rebuilder and playback threads.
- `Arc<ArcSwap<ClipAudioCache>>` for thread-safe sharing — consistent with existing `ArcSwap` patterns in the audio system.
- LRU eviction at ~500MB — reasonable for ~50 minutes of stereo f32 audio (~10MB/min). Handles long recordings without unbounded memory growth.
- Invalidation only on clip data change (not timeline edits) — correct, since timeline segment changes only affect splice points.

### Phase 2: Incremental Timeline Rebuild with Crossfade
- `from_clip_cache()` walks timeline segments, looks up clip cache, slices, concatenates — this is the O(memcpy) rebuild path that makes deletion near-instant.
- Fallback to full `::new()` on cache miss — safe degradation path for edge cases (e.g., new clip imported mid-session before cache populated).
- Timescale handling (insert silence for non-1.0) — matches existing `AudioRenderer` behavior, avoids scope creep.
- Crossfade at 15ms on buffer swap — prevents clicks without perceptible delay.
- Playhead preservation via existing `AtomicU64` — no new mechanism needed, just verification.

### Decision Matrix
All decisions are well-reasoned with clear rationale. No open questions remain.

---

## Verdict Details

| Criterion | Assessment |
|-----------|------------|
| Addresses user requirements | Yes — fast initial load (one-time decode), near-instant rebuild on deletion (splice from cache), segments stay synced |
| Technically feasible | Yes — builds on existing `ArcSwap` patterns, minimal new abstractions |
| Scope appropriate | Yes — 2 phases, no over-engineering |
| Dependencies clear | Yes — Phase 2 depends on Phase 1, no external dependencies |
| Acceptance criteria verifiable | Yes — measurable targets (<50ms rebuild, <500MB memory, no audible clicks) |
| Clip vs segment distinction | Correct throughout |

**Gate Decision:** APPROVED — Ready for execution.
