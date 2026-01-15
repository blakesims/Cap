# Manual Testing Guide - S01-S03

## What to Test

### S01 - Adaptive Buffer Sizes
The only testable feature from S01-S03 is the adaptive buffer sizing. S03's GPU converter is not yet integrated (that's S04).

### What S03 Does NOT Do Yet
The `RGBAToNV12` GPU converter exists but is **not connected** to the export pipeline. That integration happens in S04. So you won't see performance improvements yet.

## Test Environment

**Requirements:**
- macOS with Apple Silicon or Intel
- Cap desktop app built and running
- A recording to export (any length works, longer = more observable)

## Test Procedure

### Step 1: Build and Run Desktop App

```bash
cd ~/repos/cap/cap-repo  # or your cap directory
pnpm install
pnpm run dev:desktop
```

### Step 2: Create or Open a Recording

Either:
- Record a new screen capture (any duration, 10-60 seconds is fine)
- Open an existing recording

### Step 3: Export as MP4

1. Go to Editor view
2. Click Export (or use export button)
3. Select MP4 format
4. Start export

### Step 4: Check Logs for Buffer Configuration

Look in the terminal for log output like:
```
INFO cap_export::buffer_config: Using medium buffer sizes for mid-memory system
    total_ram_gb: "16.0"
    rendered_buffer: 32
    encoder_buffer: 16
```

**Expected values based on RAM:**
| Your Mac RAM | rendered_buffer | encoder_buffer |
|--------------|-----------------|----------------|
| 32GB+ | 64 | 32 |
| 16GB-31GB | 32 | 16 |
| 8GB-15GB | 16 | 8 |
| <8GB | 8 | 4 |

### Step 5: Export as GIF (Optional)

1. Select GIF format
2. Export
3. Verify logs show buffer config (uses same `rendered_frame_buffer`)

## What to Verify

### Pass Criteria
- [ ] Export completes successfully (MP4)
- [ ] Export completes successfully (GIF)
- [ ] Logs show correct buffer sizes for your RAM
- [ ] No crashes or errors during export
- [ ] Output file plays correctly

### Bonus Observations
- Note export time (we'll compare in S05)
- Note any stalls or pauses during export
- Check Activity Monitor for memory usage during export

## Performance Baseline (for S05)

If you want to help with S05 benchmarking, record these metrics:

| Metric | Value |
|--------|-------|
| Mac Model | |
| RAM | GB |
| Recording Resolution | |
| Recording Duration | sec |
| Export Time (MP4) | sec |
| Peak Memory Usage | MB |

## Known Limitations

1. **No GPU conversion yet**: S03's converter isn't integrated. Export still uses CPU-based RGBA→NV12 conversion.

2. **No timeout on send**: The sync channel send is blocking (not timed out). If encoder stalls completely, export will hang. The receive-side has timeout protection.

3. **Buffer sizes are conservative**: We erred on the side of not running out of memory. Performance gains from larger buffers may be modest (5-10% estimate).

## Troubleshooting

### Export hangs
- Check if encoder is consuming frames (Activity Monitor)
- Cancel and retry
- Report if reproducible

### Out of memory
- Shouldn't happen with current thresholds
- Report your RAM and recording specs if it occurs

### Wrong buffer sizes in logs
- Verify your actual RAM
- sysinfo should detect it correctly

## Reporting Issues

If you find issues, note:
1. Your Mac specs (model, RAM, macOS version)
2. Recording specs (resolution, duration, format)
3. Exact error message or behavior
4. Logs from terminal
