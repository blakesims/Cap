# S01 - Increase Channel Buffer Sizes with Safety Mechanisms

## Overview
- **Complexity:** Low
- **Estimated Time:** ~2-3 hours
- **Lines Added:** ~80-100
- **Lines Modified:** ~10-20
- **Files Changed:** 3-4

## Objective
Increase export channel buffer sizes adaptively based on available system RAM, with timeout mechanisms to prevent deadlocks and ensure safe operation on low-memory (8GB) Macs.

## Background/Context

### Current State
The export pipeline uses small fixed-size channel buffers:
- `crates/export/src/mp4.rs:62` - 8-frame async channel for rendered frames
- `crates/export/src/mp4.rs:63` - 8-frame sync channel for encoder input
- `crates/export/src/gif.rs:44` - 4-frame async channel for rendered frames

These small buffers create artificial stalls when the encoder temporarily blocks, as the renderer cannot continue producing frames.

### Memory Impact
Frame sizes are significant due to RGBA format (4 bytes/pixel):
- 1080p: ~8.3 MB per frame
- 4K: ~33 MB per frame

Current buffer memory usage (4K):
- MP4: 8 frames x 33MB = ~264MB
- GIF: 4 frames x 33MB = ~132MB

### Why Change is Needed
1. Small buffers cause renderer to block waiting for encoder
2. Increasing buffer sizes allows pipeline to absorb temporary encoder delays
3. Must be done safely to avoid memory exhaustion on low-RAM systems

## Acceptance Criteria

- [ ] MP4 export channel increased adaptively: 32 frames (RAM >= 16GB) or 16 frames (RAM < 16GB)
- [ ] GIF export channel increased to 16 frames
- [ ] Buffer sizing uses runtime RAM detection
- [ ] Timeout mechanism on send operations (5-second timeout)
- [ ] Graceful error handling when timeouts occur
- [ ] Memory usage validated on 8GB Mac (simulated by setting low threshold)
- [ ] No regression in export functionality
- [ ] Export error messages are user-friendly

## Technical Design

### 1. RAM Detection Approach

Use the existing `sysinfo` crate pattern from `crates/recording/src/diagnostics.rs`:

```rust
fn get_total_memory_gb() -> f64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0)
}
```

### 2. Buffer Size Thresholds

| Total RAM | Rendered Frame Buffer | Encoder Input Buffer |
|-----------|----------------------|---------------------|
| >= 32GB   | 64                   | 32                  |
| >= 16GB   | 32                   | 16                  |
| >= 8GB    | 16                   | 8                   |
| < 8GB     | 8 (current)          | 4                   |

Rationale:
- 32+ frames at 4K uses ~1GB for rendered frames
- 16 frames at 4K uses ~500MB
- 8 frames at 4K uses ~250MB
- Conservative thresholds ensure ~50% of "export budget" RAM remains free

### 3. Timeout Implementation

**IMPORTANT**: `tokio::sync::mpsc::Sender` does NOT have `send_timeout()`. Use `tokio::time::timeout()` wrapper instead (this matches existing patterns in the codebase).

For `tokio::sync::mpsc::channel` (async sender) - wrap with `tokio::time::timeout()`:
```rust
use tokio::time::{timeout, Duration};

match timeout(Duration::from_secs(5), sender.send((frame, frame_number))).await {
    Ok(Ok(())) => {}
    Ok(Err(_)) => {
        return Err("Export cancelled - channel closed".into());
    }
    Err(_) => {
        return Err("Export stalled - encoder not consuming frames".into());
    }
}
```

For `std::sync::mpsc::sync_channel` (sync sender) - use `send_timeout()`:
```rust
use std::sync::mpsc::SendTimeoutError;

match frame_tx.send_timeout(mp4_input, Duration::from_secs(5)) {
    Ok(()) => {}
    Err(SendTimeoutError::Timeout(_)) => {
        return Err("Export stalled - encoder not consuming frames".into());
    }
    Err(SendTimeoutError::Disconnected(_)) => {
        return Err("Export cancelled - encoder disconnected".into());
    }
}
```

### 4. Error Handling Strategy

1. **Timeout on frame send**: Return user-friendly error suggesting possible encoder hang
2. **Low memory detection**: Log warning but proceed with reduced buffer sizes
3. **Channel closed unexpectedly**: Propagate error with context about what failed
4. **Memory refresh failure**: Use conservative defaults (16 frames)

### 5. Configuration Structure

Create a buffer configuration helper:

```rust
pub struct ExportBufferConfig {
    pub rendered_frame_buffer: usize,
    pub encoder_input_buffer: usize,
    pub send_timeout: Duration,
}

impl ExportBufferConfig {
    pub fn for_current_system() -> Self {
        let total_ram_gb = get_total_memory_gb();

        let (rendered, encoder) = if total_ram_gb >= 32.0 {
            (64, 32)
        } else if total_ram_gb >= 16.0 {
            (32, 16)
        } else if total_ram_gb >= 8.0 {
            (16, 8)
        } else {
            (8, 4)
        };

        Self {
            rendered_frame_buffer: rendered,
            encoder_input_buffer: encoder,
            send_timeout: Duration::from_secs(5),
        }
    }
}
```

## Implementation Steps

### Step 1: Add sysinfo dependency to cap-export
**File:** `crates/export/Cargo.toml`
- Add `sysinfo = "0.35"` to dependencies (matches recording crate version, not workspace 0.32)

### Step 2: Create buffer configuration module
**File:** `crates/export/src/buffer_config.rs` (new file)
- Implement `ExportBufferConfig` struct
- Implement `get_total_memory_gb()` function
- Add logging for selected buffer sizes

### Step 3: Update MP4 export buffer sizes
**File:** `crates/export/src/mp4.rs`
- Lines 62-63: Replace hardcoded `8` with config values
- Update frame send to use timeout
- Update logging to show buffer config

### Step 4: Update GIF export buffer sizes
**File:** `crates/export/src/gif.rs`
- Line 44: Replace hardcoded `4` with config value
- Update frame send to use timeout

### Step 5: Update lib.rs exports
**File:** `crates/export/src/lib.rs`
- Add `pub mod buffer_config;`
- Re-export `ExportBufferConfig` if needed externally

### Step 6: Add integration logging
- Log detected RAM at export start
- Log selected buffer configuration
- Log any timeout events with context

## Testing Strategy

### Manual Testing
1. **Normal export**: Run MP4 and GIF exports, verify completion
2. **Large file**: Export 4K 10-minute video, verify no OOM
3. **Log verification**: Check logs show correct buffer sizing
4. **Cancellation**: Cancel export mid-way, verify clean shutdown

### Simulated Low-Memory Testing
1. Temporarily modify threshold constants to trigger "low RAM" path
2. Verify reduced buffer sizes are used
3. Verify export still completes successfully

### Automated Testing (if applicable)
- Unit test for `ExportBufferConfig::for_current_system()` with mocked RAM values
- Verify correct buffer sizes for each RAM tier

## Risks and Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| OOM on 8GB Mac with 4K export | High | Conservative thresholds, use available vs total memory |
| Timeout too aggressive | Medium | 5s generous for normal operation, log on timeout |
| sysinfo crate overhead | Low | Single call at export start, cached |
| Platform differences | Low | sysinfo handles macOS/Windows/Linux |
| Buffer size regression | Medium | Keep old sizes as minimum fallback |

## Code Locations Reference

| File | Line(s) | Purpose |
|------|---------|---------|
| `crates/export/src/mp4.rs` | 62 | Async channel for rendered frames |
| `crates/export/src/mp4.rs` | 63 | Sync channel for encoder input |
| `crates/export/src/gif.rs` | 44 | Async channel for rendered frames |
| `crates/rendering/src/lib.rs` | ~465 | Renderer frame send (sender.send()) - consider timeout here too |
| `crates/rendering/src/frame_pipeline.rs` | 333-341 | RenderedFrame struct definition |
| `crates/recording/src/diagnostics.rs` | 60-76 | Existing sysinfo usage pattern |
| `crates/recording/Cargo.toml` | 59 | Uses sysinfo = "0.35" |
| `Cargo.toml` | 74 | Workspace sysinfo = "0.32" (outdated) |

## Checklist

- [ ] Add sysinfo dependency to cap-export Cargo.toml
- [ ] Create buffer_config.rs with ExportBufferConfig
- [ ] Update mp4.rs channel creation (lines 62-63)
- [ ] Add timeout to mp4.rs frame send
- [ ] Update gif.rs channel creation (line 44)
- [ ] Add timeout to gif.rs frame send
- [ ] Add mod and export to lib.rs
- [ ] Add export start logging showing RAM and buffer config
- [ ] Test on local machine
- [ ] Verify no Rust compilation errors
- [ ] Run existing export tests if available

## Review Findings (Applied)

The following issues were identified during plan review and corrected:

1. **Tokio API Fix**: `tokio::sync::mpsc::Sender` does NOT have `send_timeout()`. Updated to use `tokio::time::timeout()` wrapper (matching existing codebase patterns in mp4.rs:147-150).

2. **Error Type Fix**: Changed `std::sync::mpsc::RecvTimeoutError` to correct `SendTimeoutError` for send operations.

3. **sysinfo Version**: Recording crate uses 0.35, workspace defines 0.32. Plan updated to use 0.35 directly to avoid version conflicts.

4. **Additional Send Point**: Identified `crates/rendering/src/lib.rs:465` where renderer sends frames. Consider timeout protection here as well for complete coverage.

## Implementation Notes

### GIF vs MP4 Timeout Architecture
The timeout implementation differs between MP4 and GIF due to architectural differences:

- **MP4**: Has render_task that receives from renderer and sends to encoder via sync channel (`frame_tx`). Timeout is on `frame_tx.send_timeout()`.
- **GIF**: Encoder thread directly receives from renderer via `video_rx.blocking_recv()`. No intermediate sync channel.

The story's "timeout on send operations" is satisfied by MP4. For GIF, timeout protection would require:
1. Modifying cap_rendering (out of scope for S01)
2. Or redesigning GIF export to match MP4's architecture

**Decision**: GIF timeout deferred to S04 (frame pipeline integration) when cap_rendering is modified.

### Missing sysinfo Import (Fixed)
Code review identified missing `use sysinfo::System;` import in buffer_config.rs. Fixed during review.

### Pre-existing Comments
Code review flagged doc comments in gif.rs (lines 13-16) and regular comments (lines 56, 69). These are pre-existing and not introduced by S01 - not removed per scope constraint.
