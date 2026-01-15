# S01-S03 Review Guide

## Purpose
This document guides a comprehensive code review of stories S01-S03 for task T002 (Export Performance Optimization). The goal is to identify issues, validate correctness, and capture learnings for S04-S05.

## Files Changed in S01-S03

### S01 - Buffer Sizes (commits: abdbd18d, 27199300)
| File | Change Type | Purpose |
|------|-------------|---------|
| `crates/export/Cargo.toml` | Modified | Added sysinfo dependency |
| `crates/export/src/buffer_config.rs` | New | RAM detection and adaptive buffer sizing |
| `crates/export/src/mp4.rs` | Modified | Use buffer config, log buffer sizes |
| `crates/export/src/gif.rs` | Modified | Use buffer config for GIF export |
| `crates/export/src/lib.rs` | Modified | Export buffer_config module |

### S02 - Format Audit (commit: a9b5b271)
| File | Change Type | Purpose |
|------|-------------|---------|
| `stories/S02-format-audit.md` | New | Documentation of format flow |

### S03 - RGBA→NV12 Converter (commit: 3c43c0d5)
| File | Change Type | Purpose |
|------|-------------|---------|
| `crates/gpu-converters/src/rgba_nv12/mod.rs` | New | Rust GPU converter implementation |
| `crates/gpu-converters/src/rgba_nv12/shader.wgsl` | New | WGSL compute shader |
| `crates/gpu-converters/src/lib.rs` | Modified | Export new converter |

## Review Checklist

### 1. CLAUDE.md Compliance

**Rust Clippy Rules (all must pass):**
- [ ] No `dbg!()` macros
- [ ] No `let _ = async_fn()` patterns
- [ ] Use `.saturating_sub()` for durations
- [ ] Merge nested `if` statements
- [ ] No `.clone()` on Copy types
- [ ] Use function refs directly (not `|x| foo(x)`)
- [ ] Accept `&[T]` not `&Vec<T>` in params
- [ ] Use `.is_empty()` not `.len() == 0`
- [ ] No `let x = ();` patterns
- [ ] Use `.unwrap_or()` not `.unwrap_or_else(|| val)` for simple values
- [ ] Use iterators not index loops
- [ ] Use `.clamp()` not manual min/max

**Code Style:**
- [ ] NO code comments (strict rule)
- [ ] Proper error handling
- [ ] Appropriate logging levels

### 2. Correctness Review

**S01 - Buffer Config:**
- [ ] RAM detection uses sysinfo correctly
- [ ] Buffer thresholds are reasonable (8/16/32/64 based on RAM)
- [ ] No panics possible (unwrap, expect, etc.)
- [ ] Logging is appropriate and useful

**S03 - RGBA→NV12 Shader:**
- [ ] BT.709 color matrix coefficients are correct
- [ ] Y plane indexing: `y * width + x` for each pixel
- [ ] UV plane indexing: `(y/2) * width + x` for even coords only
- [ ] Even dimension validation enforced
- [ ] Buffer sizes calculated correctly
- [ ] Shader bounds checking present

### 3. Memory Safety

- [ ] No buffer overflows possible
- [ ] No uninitialized memory access
- [ ] GPU buffer sizes match expected data sizes
- [ ] Y plane: `width * height` bytes
- [ ] UV plane: `width * (height/2)` bytes

### 4. Performance Considerations

- [ ] No unnecessary allocations in hot paths
- [ ] GPU workgroup size appropriate (8x8)
- [ ] Dispatch dimensions correct
- [ ] No synchronous waits where async would work

### 5. Error Handling

- [ ] All error paths have meaningful messages
- [ ] Errors propagate correctly
- [ ] No silent failures
- [ ] Validation errors are descriptive

### 6. Integration Readiness (for S04)

Check if the code is ready to be integrated:
- [ ] `RGBAToNV12` can be instantiated from rendering crate
- [ ] `ExportBufferConfig` is accessible where needed
- [ ] No circular dependencies introduced
- [ ] Public API is clean and documented through types

## Specific Verification Tasks

### Task 1: Verify BT.709 Matrix
Compare shader coefficients against standard BT.709:
```
Y  = 16 + 65.481*R + 128.553*G + 24.966*B
Cb = 128 - 37.797*R - 74.203*G + 112.0*B
Cr = 128 + 112.0*R - 93.786*G - 18.214*B
```

### Task 2: Verify Buffer Indexing
For a 1920x1080 frame:
- Y plane should be 2,073,600 bytes
- UV plane should be 1,036,800 bytes
- Max Y index: 2,073,599
- Max UV index: 1,036,799

### Task 3: Check GPU Resource Cleanup
- Are textures/buffers properly dropped?
- Is GPU memory released after convert()?

### Task 4: Verify sysinfo Usage
- Is memory queried efficiently (single call)?
- Is the value correctly converted to GB?

## Output Requirements

Create a document at:
`/home/blake/repos/cap-repo-fork/Cap/tasks/active/T002-export-performance/review-findings-s01-s03.md`

Include:
1. **Issues Found**: Critical/High/Medium/Low with file locations
2. **Verified Items**: Things confirmed correct
3. **Recommendations for S04**: Integration considerations
4. **Learnings**: General observations that improve future work
5. **Performance Notes**: Any performance concerns for S05 benchmarking
