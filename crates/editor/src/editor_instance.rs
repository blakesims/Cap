use crate::audio::{ClipAudioCache, PredecodedAudio};
use crate::editor;
use crate::playback::{self, PlaybackHandle, PlaybackStartError};
use arc_swap::ArcSwap;
use cap_audio::AudioData;
use cap_media_info::AudioInfo;
use cap_project::StudioRecordingMeta;
use cap_project::{
    CursorEvents, ProjectConfiguration, RecordingMeta, RecordingMetaInner, TimelineConfiguration,
    TimelineSegment, XY,
};
use cap_rendering::{
    ProjectRecordingsMeta, ProjectUniforms, RecordingSegmentDecoders, RenderVideoConstants,
    RenderedFrame, SegmentVideoPaths, Video, ZoomFocusInterpolator, get_duration,
    spring_mass_damper::SpringMassDamperSimulationConfig,
};
use cpal::traits::{DeviceTrait, HostTrait};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{Mutex, watch};
use tokio_util::sync::CancellationToken;
use tracing::warn;

fn get_video_duration_fallback(path: &Path) -> Option<f64> {
    tracing::debug!("get_video_duration_fallback called for: {:?}", path);
    let input = match ffmpeg::format::input(path) {
        Ok(i) => i,
        Err(e) => {
            tracing::warn!("get_video_duration_fallback: failed to open input: {}", e);
            return None;
        }
    };

    let container_duration = input.duration();
    tracing::debug!(
        "get_video_duration_fallback: container_duration (raw i64) = {}",
        container_duration
    );
    if container_duration > 0 {
        let secs = container_duration as f64 / 1_000_000.0;
        tracing::debug!(
            "get_video_duration_fallback: returning container duration {} seconds",
            secs
        );
        return Some(secs);
    }

    let stream = input.streams().best(ffmpeg::media::Type::Video)?;
    let stream_duration = stream.duration();
    let time_base = stream.time_base();
    tracing::debug!(
        "get_video_duration_fallback: stream_duration = {}, time_base = {}/{}",
        stream_duration,
        time_base.numerator(),
        time_base.denominator()
    );
    if stream_duration > 0 && time_base.denominator() > 0 {
        let secs =
            stream_duration as f64 * time_base.numerator() as f64 / time_base.denominator() as f64;
        tracing::debug!(
            "get_video_duration_fallback: returning stream duration {} seconds",
            secs
        );
        Some(secs)
    } else {
        tracing::warn!("get_video_duration_fallback: no valid duration found");
        None
    }
}

pub struct EditorInstance {
    pub project_path: PathBuf,
    pub recordings: Arc<ProjectRecordingsMeta>,
    pub renderer: Arc<editor::RendererHandle>,
    pub render_constants: Arc<RenderVideoConstants>,
    playback_active: watch::Sender<bool>,
    playback_active_rx: watch::Receiver<bool>,
    pub state: Arc<Mutex<EditorState>>,
    on_state_change: Box<dyn Fn(&EditorState) + Send + Sync + 'static>,
    pub preview_tx: watch::Sender<Option<PreviewFrameInstruction>>,
    pub project_config: (
        watch::Sender<ProjectConfiguration>,
        watch::Receiver<ProjectConfiguration>,
    ),
    pub segment_medias: Arc<Vec<SegmentMedia>>,
    meta: RecordingMeta,
    pub export_preview_active: AtomicBool,
    pub export_active: AtomicBool,
    pub audio_predecode_buffer: Arc<ArcSwap<Option<PredecodedAudio>>>,
    pub clip_audio_cache: Arc<ArcSwap<ClipAudioCache>>,
    audio_decode_cancel: CancellationToken,
    audio_decode_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl EditorInstance {
    pub async fn new(
        project_path: PathBuf,
        on_state_change: impl Fn(&EditorState) + Send + Sync + 'static,
        frame_cb: Box<dyn FnMut(RenderedFrame) + Send>,
    ) -> Result<Arc<Self>, String> {
        if !project_path.exists() {
            return Err(format!("Video path {} not found!", project_path.display()));
        }

        let recording_meta = cap_project::RecordingMeta::load_for_project(&project_path)
            .map_err(|e| format!("Failed to load recording meta: {e}"))?;

        let RecordingMetaInner::Studio(meta) = &recording_meta.inner else {
            return Err("Cannot edit non-studio recordings".to_string());
        };

        let segment_count = match meta.as_ref() {
            StudioRecordingMeta::SingleSegment { .. } => 1,
            StudioRecordingMeta::MultipleSegments { inner } => inner.segments.len(),
        };

        if segment_count == 0 {
            return Err(
                "Recording has no segments. It may need to be recovered first.".to_string(),
            );
        }

        let mut project = recording_meta.project_config();

        if project.timeline.is_none() {
            warn!("Project config has no timeline, creating one from recording segments");
            let timeline_segments = match meta.as_ref() {
                StudioRecordingMeta::SingleSegment { segment } => {
                    let display_path = recording_meta.path(&segment.display.path);
                    let duration = match Video::new(&display_path, 0.0) {
                        Ok(v) => v.duration,
                        Err(e) => {
                            warn!(
                                "Failed to load video for duration calculation: {} (path: {}), trying fallback",
                                e,
                                display_path.display()
                            );
                            match get_video_duration_fallback(&display_path) {
                                Some(d) => d,
                                None => {
                                    warn!("Fallback also failed, using default duration 5.0s");
                                    5.0
                                }
                            }
                        }
                    };
                    vec![TimelineSegment {
                        recording_clip: 0,
                        start: 0.0,
                        end: duration,
                        timescale: 1.0,
                    }]
                }
                StudioRecordingMeta::MultipleSegments { inner } => inner
                    .segments
                    .iter()
                    .enumerate()
                    .filter_map(|(i, segment)| {
                        let display_path = recording_meta.path(&segment.display.path);
                        tracing::debug!("Attempting to get duration for segment {}: {:?}", i, display_path);
                        let duration = match Video::new(&display_path, 0.0) {
                            Ok(v) => {
                                tracing::debug!("Video::new succeeded, duration: {}", v.duration);
                                v.duration
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to load video for duration calculation: {} (path: {}), trying fallback",
                                    e,
                                    display_path.display()
                                );
                                match get_video_duration_fallback(&display_path) {
                                    Some(d) => {
                                        tracing::debug!("Fallback succeeded, duration: {}", d);
                                        d
                                    }
                                    None => {
                                        warn!("Fallback also failed, using default duration 5.0s");
                                        5.0
                                    }
                                }
                            }
                        };
                        tracing::debug!("Final duration for segment {}: {}", i, duration);
                        if duration <= 0.0 {
                            return None;
                        }
                        Some(TimelineSegment {
                            recording_clip: i as u32,
                            start: 0.0,
                            end: duration,
                            timescale: 1.0,
                        })
                    })
                    .collect(),
            };

            if !timeline_segments.is_empty() {
                project.timeline = Some(TimelineConfiguration {
                    segments: timeline_segments,
                    zoom_segments: Vec::new(),
                    scene_segments: Vec::new(),
                    mask_segments: Vec::new(),
                    text_segments: Vec::new(),
                });

                if let Err(e) = project.write(&recording_meta.project_path) {
                    warn!("Failed to save auto-generated timeline: {}", e);
                }
            }
        }

        if project.clips.is_empty() {
            let calibration_store = load_calibration_store(&recording_meta.project_path);

            match meta.as_ref() {
                StudioRecordingMeta::MultipleSegments { inner } => {
                    project.clips = inner
                        .segments
                        .iter()
                        .enumerate()
                        .map(|(i, segment)| {
                            let calibration_offset = get_calibration_offset(
                                segment.camera_device_id(),
                                segment.mic_device_id(),
                                &calibration_store,
                            );
                            cap_project::ClipConfiguration {
                                index: i as u32,
                                offsets: segment
                                    .calculate_audio_offsets_with_calibration(calibration_offset),
                            }
                        })
                        .collect();
                }
                StudioRecordingMeta::SingleSegment { .. } => {
                    project.clips = vec![cap_project::ClipConfiguration {
                        index: 0,
                        offsets: cap_project::ClipOffsets::default(),
                    }];
                }
            }

            if let Err(e) = project.write(&recording_meta.project_path) {
                warn!("Failed to save auto-generated clip offsets: {}", e);
            }
        }

        let recordings = Arc::new(ProjectRecordingsMeta::new(
            &recording_meta.project_path,
            meta.as_ref(),
        )?);

        let segments = create_segments(&recording_meta, meta.as_ref(), false).await?;

        let render_constants = Arc::new(
            RenderVideoConstants::new(
                &recordings.segments,
                recording_meta.clone(),
                (**meta).clone(),
            )
            .await
            .map_err(|e| format!("Failed to create render constants: {e}"))?,
        );

        let renderer = Arc::new(editor::Renderer::spawn(
            render_constants.clone(),
            frame_cb,
            &recording_meta,
            meta,
        )?);

        let (preview_tx, preview_rx) = watch::channel(None);
        let (playback_active_tx, playback_active_rx) = watch::channel(false);

        let this = Arc::new(Self {
            project_path,
            recordings,
            renderer,
            render_constants,
            state: Arc::new(Mutex::new(EditorState {
                playhead_position: 0,
                playback_task: None,
                preview_task: None,
            })),
            on_state_change: Box::new(on_state_change),
            preview_tx,
            project_config: watch::channel(project),
            segment_medias: Arc::new(segments),
            meta: recording_meta,
            playback_active: playback_active_tx,
            playback_active_rx,
            export_preview_active: AtomicBool::new(false),
            export_active: AtomicBool::new(false),
            audio_predecode_buffer: Arc::new(ArcSwap::new(Arc::new(None))),
            clip_audio_cache: Arc::new(ArcSwap::from_pointee(ClipAudioCache::new(0, 2))),
            audio_decode_cancel: CancellationToken::new(),
            audio_decode_task: Mutex::new(None),
        });

        this.state.lock().await.preview_task =
            Some(this.clone().spawn_preview_renderer(preview_rx));

        this.spawn_audio_predecode().await;

        Ok(this)
    }

    pub fn meta(&self) -> &RecordingMeta {
        &self.meta
    }

    async fn spawn_audio_predecode(&self) {
        use crate::segments::get_audio_segments;
        use tracing::{info, warn};

        let segment_medias = self.segment_medias.clone();
        let project_config = self.project_config.1.borrow().clone();
        let audio_buffer = self.audio_predecode_buffer.clone();
        let clip_cache_handle = self.clip_audio_cache.clone();
        let cancel_token = self.audio_decode_cancel.clone();

        let duration = if let Some(timeline) = &project_config.timeline {
            timeline.duration()
        } else {
            return;
        };

        let timeline_segment_count = project_config
            .timeline
            .as_ref()
            .map(|t| t.segments.len())
            .unwrap_or(0);

        info!(
            duration_secs = duration,
            timeline_segments = timeline_segment_count,
            "Starting background audio pre-decode task"
        );

        let handle = tokio::task::spawn_blocking(move || {
            let start_time = std::time::Instant::now();
            info!("Audio pre-decode task started on blocking thread");

            if cancel_token.is_cancelled() {
                info!("Audio pre-decode cancelled before start");
                return;
            }

            let segments = get_audio_segments(&segment_medias);
            if segments.is_empty() || segments[0].tracks.is_empty() {
                info!("No audio segments for pre-decode");
                return;
            }

            info!(audio_segment_count = segments.len(), "Found audio segments for pre-decode");

            let host = cpal::default_host();
            let device = match host.default_output_device() {
                Some(d) => d,
                None => {
                    warn!("No default output device for pre-decode");
                    return;
                }
            };
            let supported_config = match device.default_output_config() {
                Ok(sc) => sc,
                Err(e) => {
                    warn!("Failed to get output config for pre-decode: {}", e);
                    return;
                }
            };

            let mut output_info = AudioInfo::from_stream_config(&supported_config);
            output_info.sample_format = output_info.sample_format.packed();
            output_info = output_info.for_ffmpeg_output();

            if cancel_token.is_cancelled() {
                return;
            }

            info!(
                sample_rate = output_info.sample_rate,
                channels = output_info.channels,
                duration_secs = duration,
                "Beginning clip audio cache population"
            );

            let cache_start = std::time::Instant::now();
            let mut cache = ClipAudioCache::new(output_info.sample_rate, output_info.channels);
            crate::audio::populate_clip_cache(&segments, &project_config, output_info, &mut cache);
            let cache_elapsed = cache_start.elapsed();

            info!(
                cache_ms = cache_elapsed.as_millis(),
                clips_cached = cache.len(),
                cache_bytes = cache.total_bytes(),
                "Clip audio cache populated"
            );

            clip_cache_handle.store(Arc::new(cache));

            if cancel_token.is_cancelled() {
                return;
            }

            info!(
                duration_secs = duration,
                "Beginning full timeline audio render for pre-decode"
            );

            let render_start = std::time::Instant::now();
            let buffer = crate::audio::PrerenderedAudioBuffer::<f32>::new(
                segments,
                &project_config,
                output_info,
                duration,
            );
            let render_elapsed = render_start.elapsed();

            if cancel_token.is_cancelled() {
                info!("Audio pre-decode cancelled after render");
                return;
            }

            let timeline_hash = project_config
                .timeline
                .as_ref()
                .map(|t| crate::audio::compute_timeline_hash(t))
                .unwrap_or(0);

            let samples = buffer.into_samples();
            let sample_count = samples.len();

            let predecoded = PredecodedAudio {
                samples,
                sample_rate: output_info.sample_rate,
                channels: output_info.channels,
                timeline_hash,
            };

            audio_buffer.store(Arc::new(Some(predecoded)));

            let total_elapsed = start_time.elapsed();
            info!(
                render_ms = render_elapsed.as_millis(),
                total_ms = total_elapsed.as_millis(),
                sample_count = sample_count,
                timeline_hash = timeline_hash,
                "Audio pre-decode COMPLETED and stored"
            );
        });

        *self.audio_decode_task.lock().await = Some(handle);
    }

    pub async fn dispose(&self) {
        self.audio_decode_cancel.cancel();

        if let Some(task) = self.audio_decode_task.lock().await.take() {
            let _ = task.await;
        }

        let mut state = self.state.lock().await;

        if let Some(handle) = state.playback_task.take() {
            handle.stop();
        }

        if let Some(task) = state.preview_task.take() {
            task.abort();
            if let Err(e) = task.await {
                tracing::warn!("preview task abort await failed: {e}");
            }
        }

        self.renderer.stop().await;

        tokio::task::yield_now().await;

        drop(state);
    }

    pub async fn modify_and_emit_state(&self, modify: impl Fn(&mut EditorState)) {
        let mut state = self.state.lock().await;
        modify(&mut state);
        (self.on_state_change)(&state);
    }

    pub async fn start_playback(self: &Arc<Self>, fps: u32, resolution_base: XY<u32>) {
        let (mut handle, prev) = {
            let mut state = self.state.lock().await;

            let start_frame_number = state.playhead_position;

            let playback_handle = match (playback::Playback {
                segment_medias: self.segment_medias.clone(),
                renderer: self.renderer.clone(),
                render_constants: self.render_constants.clone(),
                start_frame_number,
                project: self.project_config.0.subscribe(),
                predecoded_audio: self.audio_predecode_buffer.clone(),
                clip_audio_cache: self.clip_audio_cache.clone(),
            })
            .start(fps, resolution_base)
            .await
            {
                Ok(handle) => handle,
                Err(PlaybackStartError::InvalidFps) => {
                    warn!(fps, "Skipping playback start due to invalid FPS");
                    return;
                }
            };

            if let Err(e) = self.playback_active.send(true) {
                tracing::warn!(%e, "failed to send playback_active=true");
            }

            let prev = state.playback_task.replace(playback_handle.clone());

            (playback_handle, prev)
        };

        let this = self.clone();
        tokio::spawn(async move {
            loop {
                let event = *handle.receive_event().await;

                match event {
                    playback::PlaybackEvent::Start => {}
                    playback::PlaybackEvent::Frame(frame_number) => {
                        this.modify_and_emit_state(|state| {
                            state.playhead_position = frame_number;
                        })
                        .await;
                    }
                    playback::PlaybackEvent::Stop => {
                        if let Err(e) = this.playback_active.send(false) {
                            tracing::warn!(%e, "failed to send playback_active=false");
                        }
                        return;
                    }
                }
            }
        });

        if let Some(prev) = prev {
            prev.stop();
        }
    }

    fn spawn_preview_renderer(
        self: Arc<Self>,
        mut preview_rx: watch::Receiver<Option<(u32, u32, XY<u32>)>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut prefetch_cancel_token: Option<CancellationToken> = None;
            let mut has_rendered_first_frame = false;

            loop {
                preview_rx.changed().await.unwrap();
                has_rendered_first_frame = false;

                loop {
                    let Some((frame_number, fps, resolution_base)) =
                        *preview_rx.borrow_and_update()
                    else {
                        break;
                    };

                    if let Some(token) = prefetch_cancel_token.take() {
                        token.cancel();
                    }

                    if *self.playback_active_rx.borrow() {
                        break;
                    }

                    if self.export_active.load(Ordering::Acquire) {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        break;
                    }

                    let project = self.project_config.1.borrow().clone();

                    let Some((segment_time, segment)) =
                        project.get_segment_time(frame_number as f64 / fps as f64)
                    else {
                        warn!(
                            "Preview renderer: no segment found for frame {}",
                            frame_number
                        );
                        break;
                    };

                    let segment_medias = &self.segment_medias[segment.recording_clip as usize];
                    let clip_config = project
                        .clips
                        .iter()
                        .find(|v| v.index == segment.recording_clip);
                    let clip_offsets = clip_config.map(|v| v.offsets).unwrap_or_default();

                    let new_cancel_token = CancellationToken::new();
                    prefetch_cancel_token = Some(new_cancel_token.clone());

                    let playback_is_active = *self.playback_active_rx.borrow();
                    let export_preview_is_active =
                        self.export_preview_active.load(Ordering::Acquire);
                    let export_is_active = self.export_active.load(Ordering::Acquire);
                    if !playback_is_active && !export_preview_is_active && !export_is_active {
                        let prefetch_frames_count = 15u32;
                        let hide_camera = project.camera.hide;
                        let playback_rx = self.playback_active_rx.clone();
                        for offset in 1..=prefetch_frames_count {
                            let prefetch_frame = frame_number + offset;
                            if let Some((prefetch_segment_time, prefetch_segment)) =
                                project.get_segment_time(prefetch_frame as f64 / fps as f64)
                                && let Some(prefetch_segment_media) = self
                                    .segment_medias
                                    .get(prefetch_segment.recording_clip as usize)
                            {
                                let prefetch_clip_offsets = project
                                    .clips
                                    .iter()
                                    .find(|v| v.index == prefetch_segment.recording_clip)
                                    .map(|v| v.offsets)
                                    .unwrap_or_default();
                                let decoders = prefetch_segment_media.decoders.clone();
                                let cancel_token = new_cancel_token.clone();
                                let playback_rx = playback_rx.clone();
                                tokio::spawn(async move {
                                    if cancel_token.is_cancelled() || *playback_rx.borrow() {
                                        return;
                                    }
                                    let _ = decoders
                                        .get_frames(
                                            prefetch_segment_time as f32,
                                            !hide_camera,
                                            prefetch_clip_offsets,
                                        )
                                        .await;
                                });
                            }
                        }
                    }

                    let use_initial_timeout = !has_rendered_first_frame;

                    tokio::select! {
                        biased;

                        _ = preview_rx.changed() => {
                            continue;
                        }

                        segment_frames_opt = async {
                            if use_initial_timeout {
                                segment_medias.decoders.get_frames_initial(
                                    segment_time as f32,
                                    !project.camera.hide,
                                    clip_offsets,
                                ).await
                            } else {
                                segment_medias.decoders.get_frames(
                                    segment_time as f32,
                                    !project.camera.hide,
                                    clip_offsets,
                                ).await
                            }
                        } => {
                            if preview_rx.has_changed().unwrap_or(false) {
                                continue;
                            }

                            if let Some(segment_frames) = segment_frames_opt {
                                has_rendered_first_frame = true;

                                let total_duration = project
                                    .timeline
                                    .as_ref()
                                    .map(|t| t.duration())
                                    .unwrap_or(0.0);

                                let cursor_smoothing = (!project.cursor.raw).then_some(
                                    SpringMassDamperSimulationConfig {
                                        tension: project.cursor.tension,
                                        mass: project.cursor.mass,
                                        friction: project.cursor.friction,
                                    },
                                );

                                let zoom_focus_interpolator = ZoomFocusInterpolator::new(
                                    &segment_medias.cursor,
                                    cursor_smoothing,
                                    project.screen_movement_spring,
                                    total_duration,
                                );

                                let uniforms = ProjectUniforms::new(
                                    &self.render_constants,
                                    &project,
                                    frame_number,
                                    fps,
                                    resolution_base,
                                    &segment_medias.cursor,
                                    &segment_frames,
                                    total_duration,
                                    &zoom_focus_interpolator,
                                );
                                self.renderer
                                    .render_frame(segment_frames, uniforms, segment_medias.cursor.clone())
                                    .await;
                            } else {
                                warn!("Preview renderer: no frames returned for frame {}", frame_number);
                            }
                        }
                    }

                    break;
                }
            }
        })
    }

    fn get_studio_meta(&self) -> &StudioRecordingMeta {
        match &self.meta.inner {
            RecordingMetaInner::Studio(meta) => meta.as_ref(),
            _ => panic!("Not a studio recording"),
        }
    }

    pub fn get_total_frames(&self, fps: u32) -> u32 {
        let duration = get_duration(
            &self.recordings,
            &self.meta,
            self.get_studio_meta(),
            &self.project_config.1.borrow(),
        );

        (fps as f64 * duration).ceil() as u32
    }
}

impl Drop for EditorInstance {
    fn drop(&mut self) {}
}

type PreviewFrameInstruction = (u32, u32, XY<u32>);

pub struct EditorState {
    pub playhead_position: u32,
    pub playback_task: Option<PlaybackHandle>,
    pub preview_task: Option<tokio::task::JoinHandle<()>>,
}

pub struct SegmentMedia {
    pub audio: Option<Arc<AudioData>>,
    pub system_audio: Option<Arc<AudioData>>,
    pub cursor: Arc<CursorEvents>,
    pub decoders: RecordingSegmentDecoders,
}

pub async fn create_segments(
    recording_meta: &RecordingMeta,
    meta: &StudioRecordingMeta,
    force_ffmpeg: bool,
) -> Result<Vec<SegmentMedia>, String> {
    match &meta {
        cap_project::StudioRecordingMeta::SingleSegment { segment: s } => {
            let audio = s
                .audio
                .as_ref()
                .map(|audio_meta| {
                    AudioData::from_file(recording_meta.path(&audio_meta.path))
                        .map_err(|e| format!("SingleSegment Audio / {e}"))
                })
                .transpose()?
                .map(Arc::new);

            let cursor = Arc::new(
                s.cursor
                    .as_ref()
                    .map(|cursor_path| {
                        let full_path = recording_meta.path(cursor_path);
                        match CursorEvents::load_from_file(&full_path) {
                            Ok(events) => events,
                            Err(e) => {
                                warn!(
                                    "Failed to load cursor events from {}: {}",
                                    full_path.display(),
                                    e
                                );
                                CursorEvents::default()
                            }
                        }
                    })
                    .unwrap_or_default(),
            );

            let decoders = RecordingSegmentDecoders::new(
                recording_meta,
                meta,
                SegmentVideoPaths {
                    display: recording_meta.path(&s.display.path),
                    camera: s.camera.as_ref().map(|c| recording_meta.path(&c.path)),
                },
                0,
                force_ffmpeg,
            )
            .await
            .map_err(|e| format!("SingleSegment / {e}"))?;

            Ok(vec![SegmentMedia {
                audio,
                system_audio: None,
                cursor,
                decoders,
            }])
        }
        cap_project::StudioRecordingMeta::MultipleSegments { inner, .. } => {
            let mut segments = vec![];

            for (i, s) in inner.segments.iter().enumerate() {
                let audio = s
                    .mic
                    .as_ref()
                    .map(|audio| {
                        AudioData::from_file(recording_meta.path(&audio.path))
                            .map_err(|e| format!("MultipleSegments {i} Audio / {e}"))
                    })
                    .transpose()?
                    .map(Arc::new);

                let system_audio = s
                    .system_audio
                    .as_ref()
                    .map(|audio| {
                        AudioData::from_file(recording_meta.path(&audio.path))
                            .map_err(|e| format!("MultipleSegments {i} System Audio / {e}"))
                    })
                    .transpose()?
                    .map(Arc::new);

                let cursor = Arc::new(s.cursor_events(recording_meta));

                let decoders = RecordingSegmentDecoders::new(
                    recording_meta,
                    meta,
                    SegmentVideoPaths {
                        display: recording_meta.path(&s.display.path),
                        camera: s.camera.as_ref().map(|c| recording_meta.path(&c.path)),
                    },
                    i,
                    force_ffmpeg,
                )
                .await
                .map_err(|e| format!("MultipleSegments {i} / {e}"))?;

                segments.push(SegmentMedia {
                    audio,
                    system_audio,
                    cursor,
                    decoders,
                });
            }

            Ok(segments)
        }
    }
}

fn load_calibration_store(project_path: &std::path::Path) -> cap_audio::CalibrationStore {
    let calibration_dir = project_path
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| project_path.to_path_buf());

    cap_audio::CalibrationStore::load(&calibration_dir)
}

fn get_calibration_offset(
    camera_id: Option<&str>,
    mic_id: Option<&str>,
    store: &cap_audio::CalibrationStore,
) -> Option<f32> {
    match (camera_id, mic_id) {
        (Some(cam), Some(mic)) => store.get_offset(cam, mic).map(|o| o as f32),
        _ => None,
    }
}
