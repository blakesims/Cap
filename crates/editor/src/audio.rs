use cap_audio::{
    AudioData, AudioRendererTrack, FromSampleBytes, StereoMode, cast_f32_slice_to_bytes,
};
use cap_media::MediaError;
use cap_media_info::AudioInfo;
use cap_project::{AudioConfiguration, ClipOffsets, ProjectConfiguration, TimelineConfiguration};
use ffmpeg::{
    ChannelLayout, Dictionary, format as avformat, frame::Audio as FFAudio, software::resampling,
};
#[cfg(not(target_os = "windows"))]
use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, Producer},
};
use std::sync::Arc;
use tracing::info;

pub struct PredecodedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: usize,
}

pub struct AudioRenderer {
    data: Vec<AudioSegment>,
    cursor: AudioRendererCursor,
    // sum of `frame.samples()` that have elapsed
    // this * channel count = cursor
    elapsed_samples: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct AudioRendererCursor {
    clip_index: u32,
    timescale: f64,
    // excludes channels
    samples: usize,
}

#[derive(Clone)]
pub struct AudioSegment {
    pub tracks: Vec<AudioSegmentTrack>,
}

// yeah this is cursed oh well
#[derive(Clone)]
pub struct AudioSegmentTrack {
    data: Arc<AudioData>,
    get_gain: fn(&AudioConfiguration) -> f32,
    get_stereo_mode: fn(&AudioConfiguration) -> StereoMode,
    get_offset: fn(&ClipOffsets) -> f32,
}

impl AudioSegmentTrack {
    pub fn new(
        data: Arc<AudioData>,
        get_gain: fn(&AudioConfiguration) -> f32,
        get_stereo_mode: fn(&AudioConfiguration) -> StereoMode,
        get_offset: fn(&ClipOffsets) -> f32,
    ) -> Self {
        Self {
            data,
            get_gain,
            get_stereo_mode,
            get_offset,
        }
    }

    pub fn data(&self) -> &Arc<AudioData> {
        &self.data
    }

    pub fn gain(&self, config: &AudioConfiguration) -> f32 {
        (self.get_gain)(config)
    }

    pub fn stereo_mode(&self, config: &AudioConfiguration) -> StereoMode {
        (self.get_stereo_mode)(config)
    }

    pub fn offset(&self, offsets: &ClipOffsets) -> f32 {
        (self.get_offset)(offsets)
    }
}

impl AudioRenderer {
    pub const SAMPLE_FORMAT: avformat::Sample = AudioData::SAMPLE_FORMAT;
    pub const SAMPLE_RATE: u32 = AudioData::SAMPLE_RATE;
    pub const CHANNELS: u16 = 2;

    pub fn info() -> AudioInfo {
        AudioInfo::new(Self::SAMPLE_FORMAT, Self::SAMPLE_RATE, Self::CHANNELS).unwrap()
    }

    pub fn new(data: Vec<AudioSegment>) -> Self {
        Self {
            data,
            cursor: AudioRendererCursor {
                clip_index: 0,
                samples: 0,
                timescale: 1.0,
            },
            elapsed_samples: 0,
        }
    }

    pub fn set_playhead(&mut self, playhead: f64, project: &ProjectConfiguration) {
        self.elapsed_samples = self.playhead_to_samples(playhead);

        self.cursor = match project.get_segment_time(playhead) {
            Some((segment_time, segment)) => AudioRendererCursor {
                clip_index: segment.recording_clip,
                timescale: segment.timescale,
                samples: self.playhead_to_samples(segment_time),
            },
            None => AudioRendererCursor {
                clip_index: 0,
                timescale: 1.0,
                samples: self.elapsed_samples,
            },
        };
    }

    fn adjust_cursor(&mut self, timeline: &TimelineConfiguration) {
        let playhead = self.elapsed_samples_to_playhead();

        // ! Basically, to allow for some slop in the float -> usize and back conversions,
        // this will only seek if there is a significant change in actual vs expected next sample
        // (corresponding to a trim or split point). Currently this change is at least 0.2 seconds
        // - not sure we offer that much precision in the editor even!
        let new_cursor = match timeline.get_segment_time(playhead) {
            Some((segment_time, segment)) => AudioRendererCursor {
                clip_index: segment.recording_clip,
                timescale: segment.timescale,
                samples: self.playhead_to_samples(segment_time),
            },
            None => AudioRendererCursor {
                clip_index: 0,
                timescale: 1.0,
                samples: 0,
            },
        };

        let cursor_diff = new_cursor.samples as isize - self.cursor.samples as isize;
        let frame_samples = (AudioData::SAMPLE_RATE as usize) / 30;
        if new_cursor.clip_index != self.cursor.clip_index
            || cursor_diff.unsigned_abs() > frame_samples
        {
            self.cursor = new_cursor;
        }
    }

    fn playhead_to_samples(&self, playhead: f64) -> usize {
        (playhead * AudioData::SAMPLE_RATE as f64) as usize
    }

    pub fn elapsed_samples_to_playhead(&self) -> f64 {
        self.elapsed_samples as f64 / AudioData::SAMPLE_RATE as f64
    }

    pub fn render_frame(
        &mut self,
        requested_samples: usize,
        project: &ProjectConfiguration,
    ) -> Option<FFAudio> {
        self.render_frame_raw(requested_samples, project)
            .map(move |(samples, data)| {
                let mut raw_frame =
                    FFAudio::new(AudioData::SAMPLE_FORMAT, samples, ChannelLayout::STEREO);
                raw_frame.set_rate(AudioData::SAMPLE_RATE);
                raw_frame.data_mut(0)[0..data.len() * f32::BYTE_SIZE]
                    .copy_from_slice(unsafe { cast_f32_slice_to_bytes(&data) });

                raw_frame
            })
    }

    pub fn render_frame_raw(
        &mut self,
        samples: usize,
        project: &ProjectConfiguration,
    ) -> Option<(usize, Vec<f32>)> {
        if let Some(timeline) = &project.timeline {
            self.adjust_cursor(timeline);
        }
        let channels: usize = 2;

        if self.cursor.timescale != 1.0 {
            self.elapsed_samples += samples;
            return None;
        };

        let tracks = &self.data[self.cursor.clip_index as usize].tracks;

        if tracks.is_empty() {
            self.elapsed_samples += samples;
            return None;
        }

        let start = self.cursor;

        let offsets = project
            .clips
            .iter()
            .find(|c| c.index == start.clip_index)
            .map(|c| c.offsets)
            .unwrap_or_default();

        let max_samples = tracks
            .iter()
            .map(|t| {
                let track_offset_samples = (t.offset(&offsets) * Self::SAMPLE_RATE as f32) as isize;
                let available = t.data().sample_count() as isize - track_offset_samples;
                available.max(0) as usize
            })
            .max()
            .unwrap();

        if self.cursor.samples >= max_samples {
            self.elapsed_samples += samples;
            return None;
        }

        let samples = samples.min(max_samples - self.cursor.samples);

        let mut ret = vec![0.0; samples * 2];

        let track_datas = tracks
            .iter()
            .map(|t| AudioRendererTrack {
                data: t.data().as_ref(),
                gain: if project.audio.mute {
                    f32::NEG_INFINITY
                } else {
                    let g = t.gain(&project.audio);
                    if g < -30.0 { f32::NEG_INFINITY } else { g }
                },
                stereo_mode: t.stereo_mode(&project.audio),
                offset: (t.offset(&offsets) * Self::SAMPLE_RATE as f32) as isize,
            })
            .collect::<Vec<_>>();

        let actual_sample_count =
            cap_audio::render_audio(&track_datas, start.samples, samples, 0, &mut ret);

        self.elapsed_samples += actual_sample_count;
        self.cursor.samples += actual_sample_count;

        if actual_sample_count * channels < ret.len() {
            ret.resize(actual_sample_count * channels, 0.0);
        };

        Some((actual_sample_count, ret))
    }
}

#[cfg(not(target_os = "windows"))]
pub struct AudioPlaybackBuffer<T: FromSampleBytes> {
    frame_buffer: AudioRenderer,
    resampler: AudioResampler,
    resampled_buffer: HeapRb<T>,
}

#[cfg(not(target_os = "windows"))]
impl<T: FromSampleBytes> AudioPlaybackBuffer<T> {
    pub const PLAYBACK_SAMPLES_COUNT: u32 = 512;

    pub const WIRELESS_PLAYBACK_SAMPLES_COUNT: u32 = 1024;

    const PROCESSING_SAMPLES_COUNT: u32 = 1024;

    pub fn new(data: Vec<AudioSegment>, output_info: AudioInfo) -> Self {
        // Clamp output info for FFmpeg compatibility (max 8 channels)
        let output_info = output_info.for_ffmpeg_output();

        info!(
            sample_rate = output_info.sample_rate,
            channels = output_info.channels,
            sample_format = ?output_info.sample_format,
            "Audio playback output configuration"
        );

        let resampler = AudioResampler::new(output_info).unwrap();

        let capacity = (output_info.sample_rate as usize)
            * output_info.channels
            * output_info.sample_format.bytes();
        let resampled_buffer = HeapRb::new(capacity);

        let frame_buffer = AudioRenderer::new(data);

        Self {
            frame_buffer,
            resampler,
            resampled_buffer,
        }
    }

    pub fn set_playhead(&mut self, playhead: f64, project: &ProjectConfiguration) {
        self.resampler.reset();
        self.resampled_buffer.clear();
        self.frame_buffer.set_playhead(playhead, project);
    }

    #[allow(dead_code)]
    pub fn current_playhead(&self) -> f64 {
        self.frame_buffer.elapsed_samples_to_playhead()
    }

    pub fn current_audible_playhead(
        &self,
        device_sample_rate: u32,
        device_latency_secs: f64,
    ) -> f64 {
        let generated_secs = self.frame_buffer.elapsed_samples_to_playhead();
        let channels = self.resampler.output.channels;
        let buffered_elements = self.resampled_buffer.occupied_len();
        let buffered_frames = buffered_elements / channels;
        let buffered_secs = buffered_frames as f64 / device_sample_rate as f64;
        let audible = generated_secs - buffered_secs - device_latency_secs.max(0.0);
        if audible.is_sign_negative() {
            0.0
        } else {
            audible
        }
    }

    pub fn buffer_reaching_limit(&self) -> bool {
        self.resampled_buffer.vacant_len()
            <= 2 * (Self::PROCESSING_SAMPLES_COUNT as usize) * self.resampler.output.channels
    }

    fn render_chunk(&mut self, project: &ProjectConfiguration) -> bool {
        if self.buffer_reaching_limit() {
            return false;
        }

        let bytes_per_sample = self.resampler.output.sample_size();

        let next_frame = self
            .frame_buffer
            .render_frame(Self::PROCESSING_SAMPLES_COUNT as usize, project);

        let maybe_rendered = match next_frame {
            Some(frame) => Some(self.resampler.queue_and_process_frame(&frame)),
            None => self.resampler.flush_frame(),
        };

        let Some(rendered) = maybe_rendered else {
            return false;
        };

        if rendered.is_empty() {
            return false;
        }

        let mut typed_data = vec![T::EQUILIBRIUM; rendered.len() / bytes_per_sample];

        for (src, dest) in std::iter::zip(rendered.chunks(bytes_per_sample), &mut typed_data) {
            *dest = T::from_bytes(src);
        }
        self.resampled_buffer.push_slice(&typed_data);
        true
    }

    pub fn prefill(&mut self, project: &ProjectConfiguration, min_samples: usize) {
        if min_samples == 0 {
            return;
        }

        let capacity = self.resampled_buffer.capacity().get();
        let target = min_samples.min(capacity);

        while self.resampled_buffer.occupied_len() < target {
            if !self.render_chunk(project) {
                break;
            }
        }
    }

    pub fn fill(
        &mut self,
        playback_buffer: &mut [T],
        project: &ProjectConfiguration,
        min_headroom_samples: usize,
    ) {
        let filled = self.resampled_buffer.pop_slice(playback_buffer);
        playback_buffer[filled..].fill(T::EQUILIBRIUM);

        self.prefill(project, min_headroom_samples);
    }
}

pub struct AudioResampler {
    pub context: resampling::Context,
    pub output_frame: FFAudio,
    delay: Option<resampling::Delay>,
    output: AudioInfo,
}

impl AudioResampler {
    pub fn new(output_info: AudioInfo) -> Result<Self, MediaError> {
        // Clamp output info for FFmpeg compatibility (max 8 channels)
        let output_info = output_info.for_ffmpeg_output();

        let mut options = Dictionary::new();
        options.set("filter_size", "32");
        options.set("cutoff", "0.97");

        let context = resampling::Context::get_with(
            AudioData::SAMPLE_FORMAT,
            ChannelLayout::STEREO,
            AudioData::SAMPLE_RATE,
            output_info.sample_format,
            output_info.channel_layout(),
            output_info.sample_rate,
            options,
        )?;

        info!(
            input_rate = AudioData::SAMPLE_RATE,
            output_rate = output_info.sample_rate,
            output_format = ?output_info.sample_format,
            "Audio resampler created with high-quality settings (filter_size=32)"
        );

        Ok(Self {
            output: output_info,
            context,
            output_frame: FFAudio::empty(),
            delay: None,
        })
    }

    #[cfg(not(target_os = "windows"))]
    pub fn reset(&mut self) {
        *self = Self::new(self.output).unwrap();
    }

    fn current_frame_data(&self) -> &[u8] {
        let end = self.output_frame.samples() * self.output.channels * self.output.sample_size();
        &self.output_frame.data(0)[0..end]
    }

    pub fn queue_and_process_frame<'a>(&'a mut self, frame: &FFAudio) -> &'a [u8] {
        self.delay = self.context.run(frame, &mut self.output_frame).unwrap();

        // Teeechnically this doesn't work for planar output
        self.current_frame_data()
    }

    pub fn flush_frame(&mut self) -> Option<&[u8]> {
        self.delay?;

        self.delay = self.context.flush(&mut self.output_frame).unwrap();

        Some(self.current_frame_data())
    }
}

pub struct PrerenderedAudioBuffer<T: FromSampleBytes> {
    samples: Vec<T>,
    read_position: std::sync::atomic::AtomicUsize,
    sample_rate: u32,
    channels: usize,
}

impl<T: FromSampleBytes> PrerenderedAudioBuffer<T> {
    pub fn from_predecoded(predecoded: &PredecodedAudio, output_info: AudioInfo) -> Self {
        let output_info = output_info.for_ffmpeg_output();

        let rates_match = predecoded.sample_rate == output_info.sample_rate
            && predecoded.channels == output_info.channels;

        let samples = if rates_match {
            if output_info.sample_format == AudioData::SAMPLE_FORMAT
                && std::mem::size_of::<T>() == std::mem::size_of::<f32>()
            {
                let mut samples: Vec<T> = Vec::with_capacity(predecoded.samples.len());
                unsafe {
                    let src = predecoded.samples.as_ptr() as *const T;
                    let dst = samples.as_mut_ptr();
                    std::ptr::copy_nonoverlapping(src, dst, predecoded.samples.len());
                    samples.set_len(predecoded.samples.len());
                }
                samples
            } else {
                let bytes_per_sample = output_info.sample_size();
                predecoded
                    .samples
                    .iter()
                    .flat_map(|&f| match output_info.sample_format {
                        ffmpeg::format::Sample::I16(_) => {
                            let i = (f * 32767.0).clamp(-32768.0, 32767.0) as i16;
                            i.to_ne_bytes().to_vec()
                        }
                        ffmpeg::format::Sample::I32(_) => {
                            let i = (f * 2147483647.0).clamp(-2147483648.0, 2147483647.0) as i32;
                            i.to_ne_bytes().to_vec()
                        }
                        ffmpeg::format::Sample::F64(_) => (f as f64).to_ne_bytes().to_vec(),
                        _ => f.to_ne_bytes().to_vec(),
                    })
                    .collect::<Vec<u8>>()
                    .chunks(bytes_per_sample)
                    .map(|chunk| T::from_bytes(chunk))
                    .collect()
            }
        } else {
            let input_info = AudioInfo::new(
                AudioData::SAMPLE_FORMAT,
                predecoded.sample_rate,
                predecoded.channels as u16,
            )
            .unwrap();

            let mut options = Dictionary::new();
            options.set("filter_size", "32");
            options.set("cutoff", "0.97");

            let mut context = resampling::Context::get_with(
                input_info.sample_format,
                input_info.channel_layout(),
                input_info.sample_rate,
                output_info.sample_format,
                output_info.channel_layout(),
                output_info.sample_rate,
                options,
            )
            .unwrap();

            let bytes_per_sample = output_info.sample_size();
            let mut samples = Vec::new();
            let chunk_size = 4096usize;
            let mut output_frame = FFAudio::empty();

            for chunk_start in
                (0..predecoded.samples.len()).step_by(chunk_size * predecoded.channels)
            {
                let chunk_end =
                    (chunk_start + chunk_size * predecoded.channels).min(predecoded.samples.len());
                let chunk_samples = (chunk_end - chunk_start) / predecoded.channels;

                if chunk_samples == 0 {
                    break;
                }

                let mut frame = FFAudio::new(
                    AudioData::SAMPLE_FORMAT,
                    chunk_samples,
                    ChannelLayout::STEREO,
                );
                frame.set_rate(predecoded.sample_rate);

                let frame_data = &predecoded.samples[chunk_start..chunk_end];
                frame.data_mut(0)[0..frame_data.len() * std::mem::size_of::<f32>()]
                    .copy_from_slice(unsafe { cast_f32_slice_to_bytes(frame_data) });

                if let Ok(Some(_)) = context.run(&frame, &mut output_frame) {
                    let out_data = output_frame.data(0);
                    let out_samples = output_frame.samples() * output_info.channels;
                    let out_bytes = out_samples * bytes_per_sample;
                    for chunk in out_data[..out_bytes].chunks(bytes_per_sample) {
                        samples.push(T::from_bytes(chunk));
                    }
                }
            }

            while let Ok(Some(_)) = context.flush(&mut output_frame) {
                let out_data = output_frame.data(0);
                let out_samples = output_frame.samples() * output_info.channels;
                if out_samples == 0 {
                    break;
                }
                let out_bytes = out_samples * bytes_per_sample;
                for chunk in out_data[..out_bytes].chunks(bytes_per_sample) {
                    samples.push(T::from_bytes(chunk));
                }
            }

            samples
        };

        Self {
            samples,
            read_position: std::sync::atomic::AtomicUsize::new(0),
            sample_rate: output_info.sample_rate,
            channels: output_info.channels,
        }
    }

    pub fn new(
        segments: Vec<AudioSegment>,
        project: &ProjectConfiguration,
        output_info: AudioInfo,
        duration_secs: f64,
    ) -> Self {
        // Clamp output info for FFmpeg compatibility (max 8 channels)
        // The resampler will produce audio with this channel count
        let output_info = output_info.for_ffmpeg_output();

        let mut renderer = AudioRenderer::new(segments);
        let mut resampler = AudioResampler::new(output_info).unwrap();

        let can_bypass_resampler = AudioData::SAMPLE_RATE == output_info.sample_rate
            && 2 == output_info.channels
            && output_info.sample_format == AudioData::SAMPLE_FORMAT;

        if can_bypass_resampler {
            info!("Resampler bypass enabled - rates and formats match");
        }

        let total_source_samples = (duration_secs * AudioData::SAMPLE_RATE as f64) as usize;
        let estimated_output_samples =
            (duration_secs * output_info.sample_rate as f64) as usize * output_info.channels;

        let mut samples: Vec<T> = Vec::with_capacity(estimated_output_samples + 10000);
        let bytes_per_sample = output_info.sample_size();
        let chunk_size = 4096usize;

        renderer.set_playhead(0.0, project);

        let mut rendered_source_samples = 0usize;
        let output_chunk_samples = (chunk_size as f64 * output_info.sample_rate as f64
            / AudioData::SAMPLE_RATE as f64) as usize
            * output_info.channels;

        while rendered_source_samples < total_source_samples {
            let frame_opt = renderer.render_frame(chunk_size, project);

            match frame_opt {
                Some(frame) => {
                    if can_bypass_resampler {
                        debug_assert_eq!(
                            bytes_per_sample,
                            std::mem::size_of::<f32>(),
                            "Bypass requires f32 sample format"
                        );
                        let frame_samples = frame.samples() * output_info.channels;
                        let frame_bytes = &frame.data(0)[0..frame_samples * bytes_per_sample];
                        for chunk in frame_bytes.chunks(bytes_per_sample) {
                            samples.push(T::from_bytes(chunk));
                        }
                    } else {
                        let resampled = resampler.queue_and_process_frame(&frame);
                        for chunk in resampled.chunks(bytes_per_sample) {
                            samples.push(T::from_bytes(chunk));
                        }
                    }
                }
                None => {
                    if !can_bypass_resampler && let Some(flushed) = resampler.flush_frame() {
                        for chunk in flushed.chunks(bytes_per_sample) {
                            samples.push(T::from_bytes(chunk));
                        }
                    }
                    for _ in 0..output_chunk_samples {
                        samples.push(T::EQUILIBRIUM);
                    }
                }
            }

            rendered_source_samples += chunk_size;
        }

        if !can_bypass_resampler {
            while let Some(flushed) = resampler.flush_frame() {
                if flushed.is_empty() {
                    break;
                }
                for chunk in flushed.chunks(bytes_per_sample) {
                    samples.push(T::from_bytes(chunk));
                }
            }
        }

        Self {
            samples,
            read_position: std::sync::atomic::AtomicUsize::new(0),
            sample_rate: output_info.sample_rate,
            channels: output_info.channels,
        }
    }

    pub fn set_playhead(&self, playhead_secs: f64) {
        use std::sync::atomic::Ordering;
        let sample_position = (playhead_secs * self.sample_rate as f64) as usize * self.channels;
        self.read_position
            .store(sample_position.min(self.samples.len()), Ordering::Release);
    }

    #[allow(dead_code)]
    pub fn current_playhead_secs(&self) -> f64 {
        use std::sync::atomic::Ordering;
        (self.read_position.load(Ordering::Acquire) / self.channels) as f64
            / self.sample_rate as f64
    }

    pub fn fill(&self, buffer: &mut [T]) {
        use std::sync::atomic::Ordering;
        let read_pos = self.read_position.load(Ordering::Acquire);
        let available = self.samples.len().saturating_sub(read_pos);
        let to_copy = buffer.len().min(available);

        if to_copy > 0 {
            buffer[..to_copy].copy_from_slice(&self.samples[read_pos..read_pos + to_copy]);
            self.read_position
                .store(read_pos + to_copy, Ordering::Release);
        }

        if to_copy < buffer.len() {
            buffer[to_copy..].fill(T::EQUILIBRIUM);
        }
    }

    pub fn into_samples(self) -> Vec<T> {
        self.samples
    }
}
