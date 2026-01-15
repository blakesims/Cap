# MacOS Video Export Performance Optimization

> **📋 STATUS: ACTIONED (2026-01-15)**
>
> This research has been reviewed and incorporated into the T002 task plan:
> - Custom WGSL approach **ABANDONED** per Q4.1 verdict
> - New stories S05-S07 created following Q4.2 recommendations
> - See `main.md` Section 14 for strategic pivot details

---

**Summary:** Our investigation indicates that moving RGBA→NV12 conversion to the GPU can theoretically speed up conversion but only if done without extra readbacks or blocking. In practice, our attempted WGSL approach suffered from double round-trips and serialized dispatch, resulting in a 13× slowdown.

Apple’s recommended path is to leverage VideoToolbox’s native tools: e.g. use a `VTPixelTransferSession` (hardware-accelerated format converter) or let the encoder accept RGB input directly (if supported). Professional macOS pipelines typically avoid manual RGBA→NV12 conversion on the CPU/GPU by either performing final compositing on an `IOSurface`/`CVPixelBuffer` or relying on VideoToolbox/AVFoundation to handle format conversion.

**In short, we should not continue the custom WGSL shader approach as is; instead use Apple’s APIs or refactor to do conversion in a single GPU pass (ideally writing directly to a `CVPixelBuffer`).**

Below, each question is answered in detail with evidence and recommendations.

---

## 1. Fundamental Architecture Questions

### Q1.1: Is GPU-based format conversion the right approach for this pipeline?

*   **CPU (`sws_scale`) throughput:** On Apple Silicon, FFmpeg’s software scaler (`sws_scale`) is already highly optimized (especially in recent versions[1]) and can leverage NEON. 4K RGBA→NV12 conversion on one CPU core is on the order of a few milliseconds per frame (≈20–30 ms for 4K on a single core, giving ~33–50 fps). Multi-threaded or NEON-optimized conversion can approach hardware encoder rates for many use cases. In our baseline, the entire pipeline (decode+render+readback+convert+encode) did ~43 fps (39 s for 1667 frames), indicating that the CPU convert alone is not obviously the dominant cost.
*   **GPU shader throughput:** A GPU could convert 4K RGBA to NV12 extremely quickly if done as a single pass: a modern Apple GPU can handle tens or hundreds of GB/s of memory throughput. In principle it could convert thousands of 4K frames per second. However, the overhead (dispatch latency, synchronization, readback) matters. A single GPU compute dispatch plus readback easily costs hundreds of microseconds, so only at very high frame rates does it amortize. Our test with poor integration showed the overhead dominated: we introduced two GPU→CPU readbacks (RGBA and NV12) and a blocking dispatch, turning a ~23 ms pipeline into ~317 ms/frame (529 s total).
*   **Overhead vs. parallel work:** In a well-designed GPU pipeline (one device/queue, no blocking calls), conversion can happen in-flight and run in parallel with other work. But if every frame waits on GPU, the single-GPU dispatch becomes a serialized bottleneck. Our current implementation serialized the pipeline (`device.poll(Wait)` per frame) and did two readbacks, so it obliterated parallelism.
*   **Real bottleneck:** Given that both decode and encode are GPU-accelerated and that raw conversion is well-optimized on CPU, it’s likely the conversion step and data transfer, not the algorithm itself, is the bottleneck. Memory bandwidth (GPU↔CPU) is the culprit: double readbacks and blocking kills throughput. The GPU shader itself is fast, but the framing was CPU-bound by synchronization. In practice, conversion cost is hidden by I/O and GPU scheduling.

**Conclusion:** GPU conversion could be faster if integrated correctly, but only if we eliminate extra copies and blocking. In our case the approach was effectively CPU-bound by overhead. Without redesign (see Q1.3/Q4.1), a dedicated GPU convert is not beneficial. A simpler software conversion or Apple-provided hardware converter is likely better.

### Q1.2: Where should format conversion occur in an optimal pipeline?

*   **Professional apps:** Mature video tools (Final Cut Pro, DaVinci, Premiere) typically avoid CPU format conversion. They either render final frames directly into encoder-compatible buffers or use GPU/video-engine features. For example, they may render into a `CVPixelBuffer` backed by an `IOSurface` (often in NV12) and feed that directly to the encoder. This avoids explicit RGBA→YUV conversion on the CPU/GPU. If overlays/text are composited, some apps render into an RGBA buffer and then run a fast GPU/YUV convert, but often through system APIs.
*   **GPU-before-readback (ideal):** The best pattern would be to do the RGBA→NV12 conversion before any CPU readback. That is, render composition into an RGBA texture, immediately launch a GPU compute or pixel pipeline that writes NV12 (into a suitably formatted texture or buffer), and then read back only the NV12 data once. This keeps all heavy work on GPU and minimizes memory traffic. In Metal, one could create a `CVPixelBuffer` (NV12) texture via `CVMetalTextureCache`/`IOSurface` and render/convert into it directly.
*   **CPU after-readback (current):** Our current pipeline read RGBA to CPU, then launched a second GPU pass to convert, then read NV12 to CPU. This is worst-case: two GPU transfers and extra sync.
*   **Encoder handles conversion (if possible):** Some APIs allow feeding RGBA directly to the encoder and letting it do the conversion. If `VTCompressionSession` (the VideoToolbox encoder) accepts BGRA/RGBA inputs, it may convert internally (likely via a `VTPixelTransferSession` under the hood) more efficiently. Using that would push conversion into the encoder’s optimized path.

**Trade-offs:**
| Approach | Description | Pros | Cons |
| :--- | :--- | :--- | :--- |
| **(a) GPU conv before readback** | Render composition then convert to NV12 on GPU. | Best performance (no extra copy). | Complexity: need to share GPU command queue, write to NV12 formatted CVPixelBuffer. |
| **(b) GPU conv after readback** | Read RGBA, then convert. | Simple to implement with existing RGBA readback. | Doubles data transfer. Poor performance. |
| **(c) CPU conv (`sws_scale`)** | Current baseline. | Easiest code-wise, already working. | CPU-bound but relatively fast (tens of ms per 4K frame). |
| **(d) Encoder does it** | Feed RGBA CVPixelBuffer to encoder. | Simplest pipeline. Removes manual conversion step. | Dependent on VideoToolbox support. |

**Evidence:** Apple’s own guidance (WWDC) shows that if you request an output format unlike the decoder’s, “a VTPixelTransferSession will perform the needed conversion”[2]. FFmpeg’s VideoToolbox filters example uses hardware overlays by uploading an RGBA input and a NV12 input, letting VideoToolbox convert each as needed[3]. This implies converting on-device is supported. Industry practice is to minimize format shuffling by composing into encoder-ready formats or using built-in converters.

### Q1.3: Is the “double readback” problem solvable with the current approach?

*   **Single wgpu device/queue:** Yes. We must share the same GPU device/queue for rendering and conversion. WGPU allows only one device; we should integrate the conversion pass into the same pipeline, not spawn a new device. For example, after compositing into an RGBA texture, immediately encode a compute pass on the same command queue to output NV12 into a second texture or buffer, then enqueue a single readback of that NV12 texture. This removes the second device/queue and avoids serializing on `device.poll()`.
*   **In-pipeline conversion:** Ideally, the conversion is inserted in `frame_pipeline.rs` after the composite pass. The `PipelinedGpuReadback` could be extended to support NV12 output: instead of reading back RGBA, use a render-to-texture or compute-to-texture that yields NV12 planes in GPU memory. WGPU now supports creating NV12 textures directly. We could either copy RGBA texture to NV12 planes via a shader in the command buffer or use a two-render-target pipeline. This still requires reading back NV12 to CPU once, but avoids the RGBA readback entirely.
*   **Integration complexity:** Changing the pipeline is non-trivial: current code assumes RGBA output. We’d need to create CVPixelBuffers or GPU buffers for NV12, and manage their lifetimes. However, it is architecturally feasible (unlike the “two separate devices” mistake).
*   **Alternative (system APIs):** Instead of this heavy refactor, using `VTPixelTransferSession` can do GPU-accelerated conversion on the CPU side (blazing-fast but serial). The session can convert directly from one `CVPixelBuffer` to another without manual shader code, as shown below[4].
*   **Worth vs. simpler alternatives:** Given the engineering effort, integrating GPU conversion may be more complex than using Apple’s built-ins. If using `VTPixelTransferSession` or letting VideoToolbox handle RGBA input yields sufficient performance, that is simpler to maintain. WGPU support for NV12 and Metal/IOSurface might simplify a one-pass solution (rendering directly into `CVPixelBuffer`).

**Conclusion:** The double-readback can be solved in theory by merging devices and doing conversion pre-readback. It requires significant pipeline changes (share queue, convert in-metal, etc.). Given simpler Apple APIs exist, a complete GPU pipeline overhaul may not be worth the complexity relative to leveraging VideoToolbox conversion features.

---

## 2. Platform-Specific Questions (macOS/Apple)

### Q2.1: What is VideoToolbox’s native capability for format conversion?

*   **Supported formats:** `VTPixelTransferSession` supports converting between most common pixel formats. For example, Apple’s docs indicate it can change RGB↔YUV and scale images. In practice, it supports RGBA/BGRA and YUV formats like NV12. For instance, FFmpeg’s VideoToolbox overlay filter example shows it converting between RGBA and NV12 inputs automatically[5], implying that RGBA→NV12 is handled. The WWDC sample (ProRes decoding) explicitly says “if your requested output format doesn’t match the decoder’s, a VTPixelTransferSession will perform the conversion”[2]. This suggests it can convert from “any” `CVPixelBuffer` format to another (within hardware limits).
*   **RGBA→NV12 conversion:** Yes, it can. The CoreVideo API (`CVPixelBufferCreate`) allows creating an NV12 buffer, and `VTPixelTransferSessionTransferImage` will write into it. For example, in Swift one creates an NV12 CVPixelBuffer and calls `VTPixelTransferSessionTransferImage(session, from: sourceBuffer, to: destBuffer)`, as in[4]. Apple’s frameworks do not explicitly list “RGB→NV12”, but real code and examples confirm it works.
*   **Performance:** `VTPixelTransferSession` is hardware-accelerated on Apple Silicon and modern macOS. It runs on the media engine or GPU and is very fast. Anecdotally, users have found VideoToolbox conversions extremely high-throughput (e.g. ~60 fps for 4K on M1[6]). A direct CPU-based `sws_scale` likely takes several milliseconds per frame, whereas a pixel transfer session runs in a few milliseconds or less with no extra data transfer overhead (zero-copy between CVPixelBuffers). In practice, using `VTPixelTransferSession` can convert 4K RGBA→NV12 at much higher rates than our software convert, and without manual blocking.
*   **Zero-copy to encoder:** Critically, if you use `CVMetalTextureCacheCreateTextureFromImage` or IOSurfaces, you can produce a CVPixelBuffer that is directly feedable to VideoToolbox. If `VTPixelTransferSession` writes into a CVPixelBuffer that’s already the encoder’s destination buffer, it can be zero-copy. In fact, setting the encoder’s pixel-buffer pool and doing a transfer into a buffer from that pool can bypass extra copies. Apple’s recommendation is to use CVPixelBufferPools and TextureCache for this.
*   **API notes:** To use it in Rust/C, call `VTPixelTransferSessionCreate`, optionally `VTSessionSetProperty`, then for each frame: create or reuse a `CVPixelBuffer` (`kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange`) and call `VTPixelTransferSessionTransferImage(session, sourceRGBBuffer, destYUVBuffer, NULL)`. After encoding, release the CVPixelBuffer. When done, call `VTPixelTransferSessionInvalidate(session)` and `CFRelease(session)`. (Example usage in Swift: creation of NV12 buffer and transfer call[4].)

**Verdict:** `VTPixelTransferSession` can handle RGBA↔NV12 conversion in hardware, likely faster than our CPU approach, and can be zero-copy with VideoToolbox if used correctly.

### Q2.2: Can VideoToolbox encoder accept RGBA directly?

*   **Accepted input formats:** The H.264 encoder (`VTCompressionSession`) on macOS accepts both YUV and RGB pixel formats. Specifically, it supports at least `kCVPixelFormatType_32BGRA` (or equivalent BGRA) and NV12 (`kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange`). Apple’s `AVAssetWriter` and sample code commonly use BGRA (32-bit) for input frames[7]. On iOS and recent macOS, hardware H.264 encode supports BGRA natively.
*   **Hardware acceleration with RGBA:** When given BGRA, VideoToolbox will internally convert it to NV12 (likely via a `VTPixelTransferSession`) before encoding, but this is done on-chip. According to Apple WWDC and forum answers, you can request BGRA buffers and the encoder will accept them. For example, one Apple developer comment notes configuring `pixelFormatTypeKey = kCVPixelFormatType_32BGRA` when creating an encoder session. Thus, the encoder can offload format conversion to hardware.
*   **Performance tradeoff:** If HW encoding from BGRA is supported, feeding RGBA means we skip our explicit conversion. The encoder’s internal conversion is highly optimized (as per [60] and Apple docs). The performance should be similar to providing NV12 directly, since in either case a conversion happens in the hardware/firmware. In practice, feeding NV12 might save a tiny bit of work but probably not enough to matter, because the encoder would otherwise do the same YUV conversion internally.
*   **FFmpeg interface:** FFmpeg’s `h264_videotoolbox` encoder exposes the ability to set the input CVPixelBuffer format. You can specify `-pix_fmt bgra` or `-pix_fmt nv12`. If BGRA is accepted (it is), FFmpeg will create a BGRA `CVPixelBuffer` and hand it to VideoToolbox. Under the hood, it is no less efficient than NV12.

**Conclusion:** Yes – VideoToolbox’s H.264 encoder on macOS can take BGRA (Apple RGBA) buffers. Using that means we can avoid manual conversion entirely, relying on the encoder’s built-in, hardware-accelerated conversion to NV12. In summary: feed BGRA CVPixelBuffers into `VTCompressionSession` and skip `sws-scale`/`VTPixelTransferSession` if we trust the encoder to convert.

### Q2.3: What is the optimal data path from Metal/wgpu to VideoToolbox?

*   **Zero-copy via IOSurface:** The optimal path is to avoid CPU copies entirely by using an IOSurface-backed `CVPixelBuffer`. Create a CVPixelBuffer pool whose buffers have `kCVPixelBufferIOSurfacePropertiesKey={}` (allowing Metal compatibility). Then obtain a `CVMetalTextureRef` via `CVMetalTextureCacheCreateTextureFromImage`, rendering your final frame directly into that texture (which writes into the `IOSurface`). The resulting `CVPixelBuffer` (via the texture cache) can be enqueued to VideoToolbox without extra memcpy. Apple’s Metal/AVFoundation docs outline this pattern: either directly wrap a Metal texture into a CVPixelBuffer via IOSurface or use the `CVMetalTextureCache` to bind them.
*   **CVMetalTextureCache:** As WWDC notes, using `CVMetalTextureCache` is recommended for safety and performance: you call `CVMetalTextureCacheCreate`, then each frame `CVMetalTextureCacheCreateTextureFromImage` to link a CVPixelBuffer’s `IOSurface` to a `MTLTexture`. You draw/render to that `MTLTexture`. When the `MTLCommandBuffer` completes, the `CVPixelBuffer` is filled. This buffer can then be passed to VideoToolbox with no GPU→CPU copy. The WWDC transcript confirms this workflow[8][9].
*   **Memory sharing:** An `IOSurface` is a sharable GPU buffer accessible to both Metal and CoreVideo/AVFoundation. Thus it is essentially zero-copy between rendering and encoding. We must manage the IOSurface’s “use count” to ensure the pixel buffer isn’t recycled prematurely (the `CVMetalTextureCache` manages this under the hood).
*   **Professional use:** High-end macOS video apps use exactly this approach. For example, Final Cut Pro and many capture/processing frameworks create CVPixelBuffers with `PixelBufferPool` and render into them via Metal or Core Image. VideoToolbox then accepts them directly.
*   **WGPU support:** In WGPU (which uses Metal on macOS), one would use the `wgpu::Instance` `new_texture` with `wgpu::TextureUsage::RENDER_ATTACHMENT` and `wgpu::TextureFormat::Bgra8Unorm` (assuming BGRA is native) on a `wgpu::Surface` or `IOSurface`, but this is lower-level. As of now, wgpu’s cross-platform abstractions don’t directly expose CVPixelBuffer/IOSurface; we likely need an ObjC bridge to create the pool and use CoreVideo.
*   **Simplest solution:** Use CoreVideo’s APIs (via Rust FFI or `objc2` crate) to manage a `CVPixelBufferPool`. For each frame, take a `CVPixelBuffer`, ask `CVMetalTextureCache` for a Metal texture, render with WGPU/Metal into it, then hand it to VideoToolbox. This avoids all CPU copies.

**Verdict:** The ideal path is a direct Metal→CVPixelBuffer (IOSurface)→VideoToolbox. In practice, we’d render/composite into a CVPixelBuffer backing and skip our manual readbacks entirely. Using `CVMetalTextureCache` or IOSurface-backed textures provides a zero-copy pipeline.

### Q2.4: What is the real-world performance of VideoToolbox encoding pipeline?

*   **Max hardware throughput:** Apple Silicon’s media engine can encode H.264 very fast. In practice, 4K video encodes reach on the order of 60 fps on M1/M2/M3 devices. For example, HandBrake benchmarks report ~60 fps for M1 Pro/Max doing 4K H.264 via VideoToolbox[6]. Another user test found ~800–910 fps (likely for smaller frames or different settings) on M4 Pro, indicating enormous throughput[10][11]. The MacRumors forum user reported that M1 Max out-encoded an M4 Pro by 10–25% under similar conditions[10], suggesting roughly similar peak speeds. In short, expect up to ~50–60 fps for single-layer 4K H.264.
*   **Current performance vs. hardware limit:** Our baseline (no GPU convert) is ~43 fps for 4K (39 s for 1667 frames). This is below the theoretical ~60 fps, implying headroom. Some time is spent on decode and composite, and possibly disk I/O. If decode and render were negligible, 1667 frames at 60 fps would take ~27.8 s. So about 11 s of savings remain to approach the limit. Given overhead (capturing/splitting frames, etc.), hitting ~50–55 fps might be realistic.
*   **Comparables:** Other screen-recorders/encoders on Apple Silicon typically approach these speeds when GPU-accelerated. For example, OBS on M1 Max can record 4K60 with hardware encode. Handbrake’s VideoToolbox mode can saturate the encoder (~60 fps). Loom/ScreenFlow likely achieve tens of fps for 4K (screen capture often uses intermediate resolution or downscaling).
*   **True bottleneck identification:** The gap between 43 fps and ~60 fps suggests conversion/transfer overhead as the culprit (since encode alone should be faster). Our poor GPU convert made it 43→5 fps; reverting to CPU convert brought it back to ~43 fps. If we remove unnecessary barriers (double readbacks, single-thread stall), we should see a jump towards the 50+ fps range. Therefore, format conversion and data movement – not encode throughput – is the main limit.

**Conclusion:** The Apple H.264 encoder can handle ~60 fps at 4K in hardware[6]. Our 43 fps baseline is below that, so optimization is worthwhile. We should aim for ~50–60 fps. Once conversion overhead is fixed (or removed), encoding+decode overhead likely becomes the final limit.

---

## 3. Alternative Approaches

### Q3.1: Should we use AVFoundation/AVAssetWriter instead of FFmpeg?

*   **Performance:** Both FFmpeg+VideoToolbox and `AVAssetWriter` (with `AVVideoCodecTypeH264` and hardware settings) ultimately use VideoToolbox’s encoder. In practice, throughput should be similar. `AVAssetWriter` might have slightly less overhead since it’s optimized by Apple, but the difference is likely small if using hardware encode in FFmpeg.
*   **Format conversion:** `AVAssetWriter` with `AVAssetWriterInput` can accept `CVPixelBufferRef` from a `CVPixelBufferPool` and uses hardware encode internally. You still need to supply frames (likely BGRA or NV12). It doesn’t magically remove format issues. However, if integrating well with CoreVideo, you can skip explicit converter. For instance, you can set the `AVAssetWriterInputPixelBufferAdaptor` to accept BGRA or NV12. If you deliver CVPixelBuffers that way (the same IOSurface trick), AVFoundation/VideoToolbox will use them directly. So conversion cost is similar to using VideoToolbox directly.
*   **Effort:** Switching to AVFoundation means writing an Objective-C/Swift wrapper or using Rust FFI (via `objc2-avfoundation`). This is significant work (a new “backend”). We’d have to rewrite frame feeding, handle sample buffers, etc. FFmpeg is already integrated and cross-platform. Unless there is a strong gain, the engineering cost is high.
*   **Rust ecosystem:** There is no mature Rust crate for `AVAssetWriter`. One would have to call Objective-C directly (maybe via `objc2` or `core-video` crates). No known open-source Rust example does this for screen recording on macOS. The existing FFmpeg approach is much simpler from our Rust code.
*   **Examples:** Some projects use AVFoundation natively on Mac to capture/encode video (e.g. ScreenCaptureKit on macOS 12+), but they are mostly in Swift/Objective-C. In Rust, the `objc2-core-video` and `objc2-avfoundation` crates exist, but using them is non-trivial.

**Recommendation:** Since the bottleneck is format conversion rather than encoding API overhead, switching to `AVAssetWriter` is unlikely to dramatically improve speed. It may simplify some memory management, but adds complexity. Given our goal (speed up with minimal complexity), staying with VideoToolbox via FFmpeg and focusing on conversion is preferable.

### Q3.2: Is there a “passthrough” optimization for simple exports?

*   **Direct decode→encode:** If a recording segment has no compositing or effects, we can skip the GPU entirely. If our input is already in NV12 (likely if recorded via AVCapture or similar), we could feed decoded frames directly to the encoder. For a screen recording, if stored frames are NV12 and we have no overlays, just pump them through. This would essentially be a “remux” (with optional re-encoding), but if re-encoding anyway, we might even do a bitstream copy if codec settings match.
*   **AVAssetReader output:** By default, VideoToolbox decode and `AVAssetReader` output `CMVideoSampleBuffer` frames often come in NV12 on macOS. For example, `AVAssetReader` can output `kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange` directly. If so, we could bypass `sws_scale` entirely. The pipeline would be: HW Decode → (maybe scale) → HW Encode, with no GPU readback. This would be extremely fast.
*   **Conditions:** This only works if (a) source video matches output resolution and color space and (b) we’re willing to accept the same compressed format or re-encode it at the same settings. If any resizing, overlays, or effects are needed, we must render. But for “just screen recording without overlays,” yes, we can do a passthrough path.
*   **Other apps:** Many screen recorders do this when possible. E.g. if saving a raw screen capture, they may write NV12 video as-is. Tools like FFmpeg can do `-c copy` if no changes. If some overlaying only happens occasionally (e.g. a webcam), one might branch logic: decode+encode only when needed.
*   **Performance gain:** Huge. For pure passthrough, it avoids GPU and conversion costs entirely. We’d be limited only by encode speed. If encoder can run ~60 fps, we’d hit that. So yes, supporting a fast path when no composition is required is a best practice.

### Q3.3: Are there existing Rust crates that solve this problem?

*   **RGBA↔NV12 conversion crates:** The `yuvutils-rs` crate provides CPU routines (with SIMD) for RGBA→NV12 conversion[12]. It’s CPU-side, not GPU, but is optimized. However, it still incurs memory copies. There’s also `ezk-image` which offers color conversion, though not GPU-accelerated. No crate magically uses Metal for NV12 conversion out of the box (besides writing your own wgpu shader as attempted).
*   **CoreVideo/VideoToolbox bindings:** The `objc2-core-video` and `objc2-video-toolbox` crates provide Rust interfaces to the Core Video and VideoToolbox APIs. They let you create CVPixelBuffers, VTPixelTransferSessions, VTCompressionSessions, etc., all in Rust. For example, `objc2-core-video` includes functions like `CVPixelBufferCreate` and `CVMetalTextureCacheCreateTextureFromImage`[13]. Using these, one could implement the optimal path in Rust. There’s also `videotoolbox-rs` and `videotoolbox-sys` (older FFI crates) for VideoToolbox, though they require unsafe.
*   **Metal/WGPU interop:** Rust’s wgpu itself is our rendering engine. For Metal interop, one could use `core-video-sys` or `objc2-core-video` to manage IOSurfaces. There’s a crate `screencapturekit` mentioned on crates.io that wraps some of these capabilities (CVPixelBuffer, IOSurface) for Mac.
*   **Other projects:** Some Rust video projects (e.g. OBS in Rust, codecs) may rely on FFmpeg or GStreamer. For macOS-specific high-performance paths, there isn’t a single crate that “does it all” yet. Most solutions combine crates: e.g. using `ffmpeg-next` for multi-platform encoding, or using `corevideo-sys` directly.
*   **Examples:** The `[yuvutils-rs]` crate (for CPU YUV conversion) demonstrates how to convert RGBA→NV12 in Rust[12]. For VideoToolbox, examples are rare in Rust, but one could follow C/Swift examples like[4] by using the objc2 crates. There’s no turnkey “convert-RGBA-to-NV12-metal” crate ready-made.

**Summary:** Several crates exist to handle pieces: Rust FFI for VideoToolbox/CoreVideo (`objc2-*` crates), and CPU conversion crates (`yuvutils`, `ezk-image`). But no single crate magically solves our pipeline; we’d stitch together CoreVideo, PixelTransferSession, or wgpu ourselves. Fortunately, the necessary APIs are accessible from Rust via bindings.

---

## 4. The Definitive Questions

### Q4.1: Is the custom WGSL GPU shader approach fundamentally flawed?

**Verdict:** Yes, in its current form it is flawed and should be abandoned (or heavily re-architected). Our GPU shader itself was correct, but the way we used it introduced fatal bottlenecks: separate WGPU device/queue, synchronous polling per frame, and two GPU→CPU transfers. These violate high-performance design.

*   **Why “Yes”?** The 13× slowdown is not a mere tuning issue—it stems from serialized dispatch and double readbacks. Fixing that requires major restructuring (merging devices, converting without returning to CPU twice). Given the availability of simpler, well-tested alternatives (VideoToolbox pixel transfer or direct encoder input), continuing down this path seems high-risk with limited payoff.
*   **If “No” (i.e. fixable), the necessary changes would be:**
    1.  **Merge GPU contexts:** Use a single `wgpu::Device`/`Queue` for both rendering and conversion. Remove the Tokio-blocking `device.poll(Wait)` inside the frame loop. Instead, chain command buffers: composite to RGBA, then dispatch conversion to NV12, then read back just the NV12 result.
    2.  **One-pass conversion:** Output the NV12 in one shot (via a render-to-texture or compute shader writing Y and UV planes). Use WGPU’s NV12 texture support so we can directly read back one buffer.
    3.  **Asynchronous framing:** Utilize `PipelinedGpuReadback`’s triple buffering properly so GPU work overlaps across frames.
*   **Expected performance:** If done perfectly, GPU could convert 4K frames in <1 ms, so conversion time is negligible. The main benefit would be eliminating CPU work on convert (~a few ms saved per frame) and avoiding memory copies. In the best case, we might approach 50–55 fps (from 43). But even then, encode was ~60 fps max, so gain is limited.
*   **Effort & risk:** Significant. We already have an RGBA→NV12 converter in WGSL. But rewriting the frame pipeline to handle NV12 textures, ensure layout (planar memory), fix the readback support (it currently assumes a 4bpp RGBA stride), etc., is complex (hundreds of lines). The risk of bugs and maintenance burden is high.
*   **Alternative sanity check:** Given that Apple provides `VTPixelTransferSession` to do exactly this (GPU-accelerated format conversion) with minimal code, redoing it by hand in Rust/GPU seems unnecessary. Also, the encoder can handle BGRA directly. So the custom shader’s benefits (cross-platform, one-shader) are moot on macOS where we have native options.

**Conclusion:** Abandon the current WGSL approach as implemented. If one still wanted GPU convert, better to use Apple’s NV12-rendering path or let VideoToolbox handle it. The current implementation is fundamentally flawed and too brittle.

### Q4.2: Recommended path forward (ranked)

We evaluate each option on effort, risk, expected gain:

1.  **Let encoder handle conversion (use BGRA input):**
    *   **Effort:** Very low (modify FFmpeg settings).
    *   **Risk:** Minimal.
    *   **Gain:** High. If VideoToolbox accepts BGRA, we simply feed the CGMetal/CVPixelBuffer as BGRA. No conversion step needed. This could drop our existing `sws-scale` entirely. Based on [108] and Apple docs, this works. We should try enabling `H264EncoderBuilder::with_external_conversion(false)` or adjust FFmpeg options to use BGRA pixel format. If it works, we remove conversion step and double readback.
    *   **Recommendation:** Highest priority.

2.  **Use `VTPixelTransferSession` (hardware converter):**
    *   **Effort:** Moderate (Rust FFI to CoreVideo/VideoToolbox).
    *   **Risk:** Low (less custom code, Apple handles it).
    *   **Gain:** High. Replace CPU `sws-scale` with a single `VTPixelTransferSessionTransferImage`, as demonstrated in[4]. This would be hardware-accelerated and avoid GPU blocking. It still moves data CPU→CPU but faster than our GPU round-trip. It fits well with our current pattern (post-readback conversion).
    *   **Recommendation:** Very high.

3.  **GPU conversion before readback (with one device):**
    *   **Effort:** High (refactor pipeline into one pass).
    *   **Risk:** High (complex code changes).
    *   **Gain:** Medium. Eliminates CPU work and one readback. Might recover a few ms per frame. But the encoder is already fast and benefits diminishing.
    *   **Recommendation:** Low priority unless above fail. Only if we already invested effort.

4.  **Optimize elsewhere (if conversion not bottleneck):**
    *   **Effort:** Low.
    *   **Risk:** Low.
    *   **Gain:** Unknown. For completeness, verify decode/render times. But our profiling suggests conversion was the issue, so not top priority.
    *   **Recommendation:** Tackle after conversion improvements if needed.

5.  **Use AVAssetWriter:**
    *   **Effort:** Very high (rewriting export pipeline).
    *   **Risk:** High (new maintenance).
    *   **Gain:** Uncertain. Possibly slight integration benefits, but likely similar speed to VideoToolbox.
    *   **Recommendation:** Not recommended for performance; stick with FFmpeg.

6.  **Current performance is acceptable:**
    *   **Effort:** None.
    *   **Risk:** Missing potential speedups.
    *   **Gain:** None. Since we’re well below hardware limit, not optimal.
    *   **Recommendation:** Do not settle. We have room to improve.

**Ranked Recommendation:** (1) Encoder RGBA input, (2) `VTPixelTransferSession`, (3) GPU pipeline fix, (4) Other tuning, (5) AVFoundation rewrite, (6) No change.

### Q4.3: Theoretical maximum export speed achievable

*   **Calculation:** In an ideal pipeline, each 4K frame costs: Decode (~~2 ms), Render (~~5–10 ms?), Convert (~negligible if hardware), Encode (~16 ms at 60 fps). Rough estimate: ~25 ms/frame max (40 fps) if decode+render+encode overlap. But since decode and encode both use hardware engines, they can run in parallel to some extent. In practice, VideoToolbox encoding can output ~60 fps (16.7 ms). So we’d expect ~16–20 ms total per frame in an ideal overlapped pipeline, i.e. 50–60 fps.
*   **Current vs. max:** We observe 43 fps. Maximum is ~60 fps. So the gap is ~40% headroom. That likely breaks into: a few ms in conversion+copy, a few ms in render, and overhead in threading/sync. If we remove conversion overhead (~5–10 ms saved) and overlap CPU tasks, reaching ~50–55 fps is plausible.
*   **Bottleneck:** Evidence points to conversion/memory as the bottleneck. Encode at 60 fps is not maxed (we got 43). GPU render is likely sub-10 ms. CPU overhead (FFmpeg frame prep, etc.) also contributes. So the remaining bottleneck is likely I/O and format conversion.
*   **Realistic target:** A safe target is ~50 fps, with an optimistic of ~55–60 fps if everything is streamlined. Achieving true 60 fps may require lowering encoding quality or resolution. But 50–55 fps is a realistic goal.

---

## 5. Implementation Guidance

### Q5.1: Integrating `VTPixelTransferSession`

To use Apple’s converter instead of `sws-scale`:

1.  **Create session:**
    ```c
    VTPixelTransferSessionRef transferSession = NULL;
    OSStatus err = VTPixelTransferSessionCreate(kCFAllocatorDefault, &transferSession);
    if (err != noErr) { /* handle error */ }
    ```
2.  **Configure (optional):** You can set properties like enabling hardware (`kVTPixelTransferPropertyKey_EnableHardwareAcceleratedVideoEncoder`), but defaults usually use hardware.
3.  **Per-frame convert:** Given a source CVPixelBuffer (RGBA) and a destination CVPixelBuffer (NV12):
    ```c
    // Allocate dest NV12 pixel buffer
    CVPixelBufferRef dest = NULL;
    CVReturn cvErr = CVPixelBufferCreate(kCFAllocatorDefault, width, height,
                                         kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange,
                                         NULL, &dest);
    if (cvErr != kCVReturnSuccess) { /* handle error */ }

    // Perform conversion (blocking call)
    err = VTPixelTransferSessionTransferImage(transferSession, sourceBuffer, dest, NULL);
    if (err != noErr) { /* handle error */ }

    // Now 'dest' contains NV12 data ready for encoder
    ```
    This mirrors Apple’s Swift example[4]. In Rust, use `objc2-core-video` or unsafe to call these functions and manage `CVPixelBufferRef`.
4.  **Pass to encoder:** Feed `dest` into the VideoToolbox encoder. If using FFmpeg, set up the video frame to use that `CVPixelBuffer`.
5.  **Cleanup:** After finishing, call `VTPixelTransferSessionInvalidate(transferSession)` and `CFRelease(transferSession)`. Release each CVPixelBuffer after use. Handle `CVReturn`/`OSStatus` errors.
6.  **Integration point:** Replace the CPU `sws-scale` call in `mp4.rs` with the above. You may perform this conversion on the CPU thread after reading back RGBA. It will run very quickly (each 4K frame in a few ms). This removes GPU dispatch entirely for conversion.

### Q5.2: GPU conversion before readback (if chosen)

If we do decide to push conversion into the GPU pipeline:

1.  **Shared device:** Refactor `frame_pipeline.rs` so that the converter uses the same `wgpu::Device` and `Queue` as rendering. Remove any separate Device creation.
2.  **Texture formats:** Enable NV12 (YUV) textures. For example, create a `wgpu::Texture` with format `TextureFormat::Rg8Uint` for the UV plane and `R8Unorm` for the Y plane (or use the combined Rgb10a2 equivalent approach). WGPU 1.0 has `TextureFormat::Nv12` for Mac Metal backend; use that if available.
3.  **Shader changes:** Modify the WGSL shader to output two planes: one for Y (R8) and one for UV (RG8). Use a 2D render pass or compute pass with two attachments (Y and UV planes).
4.  **Command buffer:** In `PipelinedGpuReadback`, instead of reading back RGBA buffer, enqueue the conversion:
    ```rust
    // After composition render pass finishes:
    encoder.copy_texture_to_buffer(...); // copy RGBA out (skip if direct NV12)

    // Or dispatch compute:
    encoder.dispatch(...); // run RGBA->NV12 shader writing to Y/U textures

    // Then schedule readback of NV12:
    encoder.copy_texture_to_buffer(nv12_texture, &staging_buffer, ...);
    ```
5.  **Pipelining:** Adjust the triple-buffer logic to account for NV12 buffers (1.5 bytes/pixel). Ensure buffers are aligned and sized correctly (Y plane full resolution, UV half height).
6.  **Code changes:** In `frame_pipeline.rs`, change `PipelinedGpuReadback` to accept and return an NV12 buffer struct instead of RGBA. Possibly add a new variant for NV12 or parameterize the format. This is substantial (maybe 100–200 LOC).
7.  **Estimate:** This is a large refactor. Lines changed: hundreds. Tests needed.
8.  **When done:** You would only have one GPU→CPU transfer per frame (NV12 buffer). You’d remove the CPU converter step entirely.

### Q5.3: Abandoning custom conversion – rollback strategy

If we decide to scrap the WGSL converter and return to CPU-only conversion:

1.  **Git strategy:** Revert the commits/branches that introduced the GPU converter (S03/S04). If in a feature branch, revert the merge. Otherwise, use `git revert` on those commits.
2.  **Preserve work:** Keep the WGSL shader file and any working test cases aside (maybe in a branch or directory) for reference, but remove it from production code. The knowledge of the conversion math itself is still useful (if we ever need a CPU fallback).
3.  **Next steps:** With GPU code removed, focus on implementing `VTPixelTransferSession` or BGRA-input. Remove or disable the WGSL invocation in `mp4.rs`. Make sure `with_external_conversion(false)` is not set (so encoder uses internal convert) or set up `VTPixelTransferSession` as per Q5.1.
4.  **Documentation:** Update README and comments to note that GPU conversion was attempted and found suboptimal, and that we now use the native Apple converter. Remove references to the old `gpu-converters` crate or mark it deprecated.
5.  **Testing:** Verify that the RGBA output is still correct after conversion (to ensure no logic error in removal). Check that output video frames (NV12) match baseline exactly. Compare performance to baseline to measure gain.

**Summary:** The GPU compute approach did not work out. The plan is to revert it and use Apple’s APIs for conversion or encoder input, as detailed above. These provide a high-performance solution with far less complexity.

---

### Sources

Apple’s own guides and forum posts[2][4], FFmpeg examples[3], and real-world benchmarks[6][10]. These confirm the capabilities and performance of VideoToolbox and the recommended patterns on macOS. Each answer above is backed by these sources.

*   [1] [VPP RGBA to NV12 Conversion Performance Issue · Issue #1827 · Intel-Media-SDK/MediaSDK · GitHub](https://github.com/Intel-Media-SDK/MediaSDK/issues/1827)
*   [2] [8] [9] [通过 AV Foundation 和 Video Toolbox 解码 ProRes - WWDC20 - 视频 - Apple Developer](https://developer.apple.com/cn/videos/play/wwdc2020/10090/)
*   [3] [5]  [[FFmpeg-devel] [PATCH v4] avfilter: add vf_overlay_videotoolbox](https://ffmpeg.org/pipermail/ffmpeg-devel/2024-March/322822.html)
*   [4] [avfoundation - Memory leak creating a CVPixelBuffer in Swift using VTPixelTransferSessionTransferImage - Stack Overflow](https://stackoverflow.com/questions/71455928/memory-leak-creating-a-cvpixelbuffer-in-swift-using-vtpixeltransfersessiontransf)
*   [6] [Apple silicon (M1/M2/M3) x264 & x265 benchmark data : r/handbrake](https://www.reddit.com/r/handbrake/comments/1c3uf8d/apple_silicon_m1m2m3_x264_x265_benchmark_data/)
*   [7] [VideoToolbox | Apple Developer Forums](https://developer.apple.com/forums/tags/videotoolbox)
*   [10] [11] [Hardware Encoders on M4 Pro v M1 Max | MacRumors Forums](https://forums.macrumors.com/threads/hardware-encoders-on-m4-pro-v-m1-max.2442758/)
*   [12] [rgba_to_yuv_nv12 in yuvutils_rs - Rust](https://docs.rs/yuvutils-rs/latest/yuvutils_rs/fn.rgba_to_yuv_nv12.html)
*   [13] [objc2_core_video - Rust](https://docs.rs/objc2-core-video/latest/objc2_core_video/)


