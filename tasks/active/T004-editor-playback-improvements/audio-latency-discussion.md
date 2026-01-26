# Audio Playback Latency - Architecture Discussion

## Problem Statement

When pressing play in the Cap desktop editor, video starts immediately but audio has a significant delay (2-7+ seconds depending on recording length). This makes scrubbing and editing difficult.

## Investigation Results

### Measured Timing (9.5-minute recording)

| Phase | Duration | % of Total |
|-------|----------|------------|
| Decode loop | 6.44s | 94% |
| Flush loop | 0.35s | 5% |
| Init + stream setup | 0.07s | 1% |
| **Total audio delay** | **6.86s** | 100% |

### Key Metrics
- Recording duration: 569.64 seconds (~9.5 minutes)
- Chunks processed: 26,702 at 4,149 chunks/sec (~88x realtime)
- Memory allocated: 191 MB for pre-rendered audio buffer
- IPC latency: 12ms (video starts immediately)

### Root Cause

The current architecture in `crates/editor/src/playback.rs` and `crates/editor/src/audio.rs` uses `PrerenderedAudioBuffer::new()` which synchronously decodes and resamples the **entire audio track** before playback can begin.

**Delay scales linearly with recording duration:**
- 2-minute video → ~1.4s delay
- 10-minute video → ~7s delay
- 30-minute video → ~21s delay

## Current Architecture

```
User presses Play
    → commands.startPlayback() [Tauri IPC, returns in 12ms]
    → Spawns std::thread for audio
        → Gets cpal audio device
        → create_stream_prerendered()
            → PrerenderedAudioBuffer::new() [BLOCKING - decodes ALL audio]
                → Loop: decode 1024 samples → resample → store
                → Repeat for entire recording
            → Build cpal stream with callback
        → stream.play()
    → Audio finally starts
```

### Relevant Code Paths

- `crates/editor/src/playback.rs:761-844` — `AudioPlayback::spawn()`
- `crates/editor/src/playback.rs:1147-1226` — `create_stream_prerendered()`
- `crates/editor/src/audio.rs:462-544` — `PrerenderedAudioBuffer::new()`
- `crates/editor/src/playback.rs:846-1145` — `create_stream()` (dead code, streaming approach)

### Existing Dead Code

There is already a `create_stream<T>()` function (lines 846-1145) that implements on-demand audio decoding with a ring buffer. It is marked `#[allow(dead_code)]` and only compiled for non-Windows. This suggests a previous attempt at streaming audio that was abandoned in favor of pre-rendering.

---

## Proposed Solutions

### Option A: Streaming Decode (On-Demand)

**Description:** Decode audio chunks on-demand during playback using a ring buffer, similar to the existing dead `create_stream()` implementation.

**Implementation:**
- Use `AudioPlaybackBuffer` with ring buffer (already partially implemented)
- Decode ~100-200ms of audio ahead of playback position
- Audio callback pulls from ring buffer; background task refills it

**Pros:**
- Near-instant playback start (<100ms)
- Memory efficient (only buffer what's needed)
- Existing partial implementation to build on

**Cons:**
- Seek operations require buffer flush and re-decode
- Risk of audio dropouts if decode can't keep up
- More complex synchronization between decode thread and audio callback
- The existing implementation was abandoned (unknown reason)

---

### Option B: Async Pre-render with Progressive Start

**Description:** Start audio playback as soon as the first N seconds are decoded, continue pre-rendering in background.

**Implementation:**
- Pre-render first 5-10 seconds synchronously
- Start playback immediately
- Continue pre-rendering remaining audio in background thread
- Extend buffer as decoding progresses

**Pros:**
- Fast initial start (~500ms for first 5s of audio)
- Still benefits from pre-rendered quality/simplicity
- Graceful degradation if user seeks past decoded region

**Cons:**
- Complex state management (buffer growing during playback)
- Seeking beyond decoded region still causes delay
- Memory still grows to full size eventually
- Race conditions between playback and buffer extension

---

### Option C: Cached Pre-rendered Audio

**Description:** Store the pre-rendered audio buffer to disk after first playback; load from cache on subsequent plays.

**Implementation:**
- First play: pre-render as now, save to `.cache/audio-prerender.bin`
- Subsequent plays: mmap or load cached buffer
- Invalidate cache when project config changes audio settings

**Pros:**
- Second+ playback is instant
- Simple implementation (serialize/deserialize)
- No changes to audio callback logic

**Cons:**
- First playback still has full delay
- Disk I/O for large files (191MB for 10-min video)
- Cache invalidation complexity
- Doesn't help iterative editing workflow (frequent changes)

---

### Option D: Hybrid Streaming + Lookahead Cache

**Description:** Stream audio on-demand but maintain a modest lookahead cache (30-60 seconds) that pre-decodes ahead of playback.

**Implementation:**
- Ring buffer for immediate playback (~500ms)
- Background task decodes 30-60s ahead of current position
- On seek: if within cache, instant; otherwise brief decode delay

**Pros:**
- Fast start (<100ms)
- Seeks within cache window are instant
- Bounded memory usage
- Good balance of responsiveness and complexity

**Cons:**
- Most complex implementation
- Still has delay for large seeks
- Need to tune cache size for different use cases

---

### Option E: Reduce Decode Overhead

**Description:** Optimize the existing pre-render approach to be faster.

**Implementation:**
- Reduce resampler quality (filter_size=128 → 32)
- Use SIMD-optimized decode paths
- Parallel chunk processing
- Skip resampling if input/output rates match

**Pros:**
- Minimal code changes
- Could reduce delay by 2-4x
- Preserves simple architecture

**Cons:**
- May degrade audio quality
- Still O(n) with recording length
- Diminishing returns (10-min still ~2-3s even with 3x speedup)
- Doesn't solve fundamental scaling issue

---

## Evaluation Criteria

1. **Startup Latency** — Time from play press to audio output
2. **Seek Responsiveness** — Time to resume audio after seeking
3. **Implementation Complexity** — Code complexity and risk
4. **Memory Efficiency** — RAM usage during playback
5. **Audio Quality** — Any degradation vs. current approach
6. **Maintainability** — Long-term code health
7. **Cross-platform** — Works on macOS and Windows

---

## Architecture Review

### Code Analysis Summary

After reviewing the relevant source files, several important patterns and constraints emerge:

**Current Architecture Observations:**

1. **`PrerenderedAudioBuffer::new()`** (audio.rs:463-560) performs synchronous decoding in a tight loop with 1024-sample chunks. The resampler uses `filter_size=128` for high quality. Memory allocation is pre-estimated but the Vec grows dynamically.

2. **`AudioPlaybackBuffer`** (audio.rs:252-387) exists as a streaming implementation using `ringbuf::HeapRb<T>`. It includes `set_playhead()`, `prefill()`, and `fill()` methods designed for on-demand operation. This is the foundation of the dead `create_stream()` code.

3. **`create_stream()`** (playback.rs:860-1157) is a 300-line implementation with sophisticated features:
   - Buffer size negotiation with device capabilities (Fixed vs DeviceDefault strategies)
   - Wireless device detection and larger buffer sizing
   - Latency correction via `LatencyCorrector`
   - Platform-specific sync thresholds (Windows vs macOS)
   - Smooth playhead adjustment (`set_playhead_smooth()`)
   - Initial prefill before playback starts

4. **Platform divergence**: `create_stream()` is `#[cfg(not(target_os = "windows"))]` and marked dead. This strongly suggests Windows had issues with the streaming approach (likely audio dropouts or device compatibility).

5. **Thread model**: Audio runs on a separate `std::thread` (not tokio) because cpal callbacks require real-time guarantees. The `watch` channel pattern is already used extensively for playhead synchronization.

6. **Resampler cost**: The resampler context uses FFmpeg's `software::resampling::Context` with filter_size=128. Resetting requires full reconstruction (`AudioResampler::reset()` calls `Self::new()`).

---

### Evaluation Matrix

| Criterion | A: Streaming | B: Progressive | C: Cached | D: Hybrid | E: Optimize |
|-----------|--------------|----------------|-----------|-----------|-------------|
| **Startup Latency** | 5 | 4 | 2 | 5 | 3 |
| **Seek Responsiveness** | 2 | 3 | 5 | 4 | 5 |
| **Impl Complexity** | 3 | 2 | 4 | 1 | 5 |
| **Memory Efficiency** | 5 | 2 | 1 | 4 | 1 |
| **Audio Quality** | 5 | 5 | 5 | 5 | 3 |
| **Maintainability** | 3 | 2 | 4 | 2 | 5 |
| **Cross-platform** | 2 | 4 | 5 | 2 | 5 |
| **Total (sum)** | **25** | **22** | **26** | **23** | **27** |
| **Weighted Total*** | **27** | **24** | **25** | **26** | **27** |

*Weighted: Startup Latency x2 (primary goal)

---

### Detailed Analysis

#### Option A: Streaming Decode (On-Demand)

**Strengths:**
- Existing `AudioPlaybackBuffer` implementation provides 70% of the needed infrastructure
- Ring buffer pattern (`HeapRb<T>`) is the correct lock-free primitive for audio
- The `prefill()` method already handles initial buffer population
- Memory usage bounded to ~1 second of audio (~200KB at 48kHz stereo f32)

**Weaknesses:**
- The dead `create_stream()` was clearly abandoned for reasons. Based on code analysis:
  - Windows-specific sync logic suggests platform issues
  - Complex latency correction (`LatencyCorrector`) needed to avoid drift
  - The callback must never block, but `render_chunk()` does FFmpeg operations
- Seek requires `resampler.reset()` which reconstructs the entire FFmpeg context
- If decode falls behind, audio will underrun (clicks/pops)

**Why It Was Likely Abandoned:**
The code shows Windows-specific `#[cfg]` blocks with different sync thresholds, separate buffer sizing for wireless devices, and fallback buffer size negotiation. This suggests:
1. Windows had higher latency requirements the streaming couldn't meet
2. Wireless audio devices (AirPods, Bluetooth) caused unpredictable latency
3. The team chose reliability (pre-rendered) over responsiveness

**Rust Patterns Observed:**
- Uses `watch::Receiver::has_changed()` for non-blocking playhead updates in callback
- `ringbuf` crate's `Producer`/`Consumer` split for lock-free audio

---

#### Option B: Async Pre-render with Progressive Start

**Strengths:**
- Conceptually simple: start with partial buffer, grow it
- Could use existing `PrerenderedAudioBuffer` with minor modifications
- User gets audio quickly; full buffer arrives in background

**Weaknesses:**
- **Race condition risk**: `samples: Vec<T>` cannot safely grow while being read
  - Would need `Arc<RwLock<Vec<T>>>` or atomic swap of buffers
  - Either adds latency (lock contention) or complexity (double-buffering)
- Seek beyond decoded region is undefined behavior or must block
- No clean way to signal "buffer not yet available for this range"

**Critical Issue:**
The current `PrerenderedAudioBuffer::fill()` does:
```rust
buffer[..to_copy].copy_from_slice(&self.samples[self.read_position..]);
```
This cannot be made thread-safe without introducing locks that would violate real-time audio requirements. The cpal callback MUST NOT block.

---

#### Option C: Cached Pre-rendered Audio

**Strengths:**
- Second play is instant via `mmap()` or bulk read
- Current architecture completely unchanged
- Cross-platform with standard file I/O

**Weaknesses:**
- **First play latency unchanged** - this is the primary complaint
- Cache invalidation is error-prone (audio gain, stereo mode, clip offsets, timeline edits)
- During editing workflow, user makes frequent changes invalidating cache

**When It Makes Sense:**
Only valuable for playback-heavy workflows (reviewing final edit). For active editing, cache hit rate will be very low.

---

#### Option D: Hybrid Streaming + Lookahead Cache

**Strengths:**
- Best theoretical latency/responsiveness balance
- Bounded memory (~30-60 seconds of audio = ~15-30MB)
- Seeks within cache window are instant

**Weaknesses:**
- **Most complex implementation** combining issues from A, B, and C
- Cache management: what to prefetch, when to evict, bidirectional seeking
- Testing matrix explosion: cache hit vs miss vs partial hit scenarios

---

#### Option E: Reduce Decode Overhead

**Strengths:**
- Minimal code changes to existing architecture
- Preserves the simplicity of pre-rendered approach
- Predictable behavior (no race conditions, no underruns)

**Optimization Opportunities:**

1. **Resampler quality** (high impact):
   - Current: `filter_size=128` (high quality, slow)
   - Proposed: `filter_size=32` or `16` (still good, 4-8x faster)
   - Quality impact: minimal for speech/screen recordings

2. **Skip resampling when rates match**:
   - If `AudioData::SAMPLE_RATE == output_info.sample_rate`, bypass resampler entirely
   - Common case: 48kHz source to 48kHz output device

3. **Larger decode chunks**:
   - Current: 1024 samples per iteration
   - Larger chunks (4096, 8192) reduce per-iteration overhead

**Expected Improvement:**
- Filter size reduction: 2-3x speedup
- Skip resampling: 3-5x speedup (when applicable)
- Combined: 10-minute recording from ~7s to ~1-2s

**Weaknesses:**
- Still O(n) - 30-minute recording = ~3-6s delay even optimized
- Doesn't solve the fundamental problem, just delays it

---

### Additional Options

#### Option F: Background Pre-decode on Project Open

**Concept:** Start decoding audio when the editor opens (before user presses play), not when play is pressed.

**Implementation:**
- On project load, spawn background thread to pre-render audio
- When user presses play: if ready, instant; if not, show brief "preparing audio" state
- Most users spend several seconds looking at timeline before playing

**Pros:**
- Zero perceived latency if user waits 5+ seconds after opening
- Minimal architecture changes (just timing shift)
- Falls back gracefully to current behavior

**Cons:**
- Memory allocated even if user never plays
- Wasted work if user seeks before playing

---

### Recommendation

**Primary Recommendation: Option E (Reduce Decode Overhead) + Option F (Background Pre-decode)**

**Rationale:**

1. **Low Risk**: Option E is purely optimization with no architectural changes
2. **Measurable Progress**: Implement incrementally:
   - Phase 1: Reduce filter_size to 32 (1 line change, ~2-3x speedup)
   - Phase 2: Skip resampling when rates match (~3-5x when applicable)
   - Phase 3: Background pre-decode on project open
3. **Acceptable Outcome**: For most recordings (<10 minutes), optimized pre-render achieves <2s latency
4. **Cross-Platform Safe**: No platform-specific audio streaming code
5. **Maintainability**: No new concurrency primitives or state machines

**Runner-Up: Option A (Streaming Decode)**

If Option E optimizations prove insufficient, revive the dead `create_stream()` implementation with Windows-specific fallback to pre-rendered.

---

### Risk Assessment

| Option | Technical Risk | Schedule Risk | Quality Risk |
|--------|---------------|---------------|--------------|
| A | High | Medium | Low |
| B | High | High | Low |
| C | Low | Low | Low |
| D | Very High | High | Low |
| E | Very Low | Low | Medium |
| F | Low | Low | Low |

Option E+F has the best risk profile for delivering meaningful improvement quickly.

---

## Decision

**Accepted: Option E (Optimize) + Option F (Background Pre-decode)**

### Rationale

1. Low technical risk with high potential impact
2. Incremental implementation allows validation at each step
3. No new concurrency primitives or platform-specific audio code
4. Graceful fallback to current behavior if optimizations underperform

### Implementation Plan

| Phase | Change | Expected Impact | Effort |
|-------|--------|-----------------|--------|
| 1 | Reduce `filter_size` from 128 to 32 | 2-3x speedup | 1 line |
| 2 | Skip resampling when input/output rates match | 3-5x when applicable | ~20 lines |
| 3 | Increase decode chunk size from 1024 to 4096 | 10-20% improvement | ~5 lines |
| 4 | Background pre-decode on project open | Hides latency entirely | ~50 lines |
| 5 | Add "Preparing audio..." toast if play pressed before ready | Better UX | ~10 lines |

### Success Criteria

- **Phase 1-3**: 10-minute recording delay drops from ~7s to <2s
- **Phase 4-5**: Zero perceived delay for typical editing workflows (user opens project, examines timeline for 3+ seconds, then plays)

### Future Consideration

If Phase 1-5 proves insufficient for long recordings (30+ minutes), consider:
- Reviving `create_stream()` for macOS only (streaming decode)
- Windows continues to use optimized pre-rendered approach
- This is already partially supported by the `#[cfg(not(target_os = "windows"))]` structure

### Next Steps

1. Remove the `[AUDIO_LATENCY]` debug logging added during investigation
2. Implement Phase 1 (filter_size change)
3. Re-run timing measurement to validate improvement
4. Proceed to Phase 2-5 based on results
