
# 20. Media Pipeline

## 20.1 Overview

The media pipeline handles audio and video playback (`<audio>`, `<video>`), real-time audio processing (Web Audio API), media capture (getUserMedia), streaming extensions (MSE), and encrypted media (EME). It is one of the most complex subsystems in a browser engine, involving codec negotiation, hardware acceleration, real-time scheduling, and cross-process coordination.

```
┌─────────────────────────────────────────────────────────────────────┐
│ Renderer Process                                                    │
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────────────────┐ │
│  │ HTMLMediaElement│ │ MediaSource  │  │ Web Audio API             │ │
│  │ (<video>,     │  │ Extensions   │  │ (AudioContext,            │ │
│  │  <audio>)     │  │ (MSE)        │  │  AudioNode graph)         │ │
│  └──────┬───────┘  └──────┬───────┘  └─────────────┬─────────────┘ │
│         │                 │                         │               │
│         ▼                 ▼                         ▼               │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ MediaPlayer                                                  │   │
│  │  ├── MediaDemuxer (container parsing)                        │   │
│  │  ├── DecoderProxy (IPC to Decoder Process)                   │   │
│  │  ├── AudioRenderer (feeds audio device or Web Audio)         │   │
│  │  └── VideoRenderer (feeds compositor layer)                  │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                          │ IPC                                      │
├──────────────────────────┼──────────────────────────────────────────┤
│ Decoder Process          │ (sandboxed utility)                      │
│                          ▼                                          │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ DecoderHost                                                  │   │
│  │  ├── SoftwareDecoders (dav1d, libvpx, opus, vorbis, flac)   │   │
│  │  ├── PlatformDecoders (AVFoundation, MediaFoundation, VA-API)│   │
│  │  └── CdmProxy (EME → CDM plugin, future)                    │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ GPU Process                                                         │
│  ├── Hardware-decoded frames (zero-copy texture import)             │
│  └── Video frame → compositor layer (Ch. 15)                        │
└─────────────────────────────────────────────────────────────────────┘
```

## 20.2 Codec Strategy

### 20.2.1 Decoder Trait

All decoders implement a common trait, following the same pattern as ImageDecoder (Ch. 18, ADR #31) and HttpTransport (Ch. 10):

```rust
pub trait MediaDecoder: Send {
    /// Initialize the decoder with codec configuration.
    fn configure(&mut self, config: &DecoderConfig) -> Result<(), DecoderError>;

    /// Submit an encoded packet for decoding.
    fn decode(&mut self, packet: EncodedPacket) -> Result<(), DecoderError>;

    /// Retrieve decoded frames. May return zero or more frames per decode call.
    fn receive_frame(&mut self) -> Result<Option<DecodedFrame>, DecoderError>;

    /// Flush the decoder (end of stream or seek).
    fn flush(&mut self) -> Result<(), DecoderError>;

    /// Query decoder capabilities.
    fn capabilities(&self) -> DecoderCapabilities;
}

pub struct DecoderConfig {
    pub codec: Codec,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub extra_data: Option<Bytes>,  // codec-specific initialization data
}

pub enum DecodedFrame {
    Video(VideoFrame),
    Audio(AudioFrame),
}

pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub planes: Vec<Plane>,
    pub timestamp: Duration,
    pub duration: Duration,
    /// Hardware-decoded: opaque texture handle from GPU process
    pub hw_texture: Option<GpuTextureHandle>,
}

pub struct AudioFrame {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u32,
    pub timestamp: Duration,
    pub duration: Duration,
}
```

### 20.2.2 Core / Compat Codec Classification

**Video codecs:**

| Codec | Classification | Decoder | Notes |
| --- | --- | --- | --- |
| VP8 | Core | libvpx (Rust bindings) | WebM standard, royalty-free |
| VP9 | Core | libvpx (Rust bindings) | YouTube primary codec |
| AV1 | Core | dav1d (Rust bindings) | Next-gen royalty-free, Ch. 18 shares dav1d for AVIF |
| H.264/AVC | Core | Platform decoder | Patent-encumbered; delegate to OS (AVFoundation, MediaFoundation, VA-API) |
| H.265/HEVC | Core | Platform decoder | Patent-encumbered; platform-only |
| MPEG-1 | Compat | Software (optional) | Legacy |
| WMV | Not supported | — | Proprietary, negligible web usage |

**Audio codecs:**

| Codec | Classification | Decoder | Notes |
| --- | --- | --- | --- |
| Opus | Core | opus (Rust bindings) | WebRTC mandatory, best general-purpose |
| Vorbis | Core | lewton (pure Rust) or libvorbis | Ogg container standard |
| AAC | Core | Platform decoder | Patent-encumbered; delegate to OS |
| FLAC | Core | claxon (pure Rust) or libflac | Lossless |
| MP3 | Core | minimp3 (Rust bindings) | Ubiquitous, patents expired (2017) |
| WMA | Not supported | — | Proprietary |

**Container formats:**

| Container | Classification | Notes |
| --- | --- | --- |
| MP4 / fMP4 | Core | H.264/H.265/AV1/AAC container |
| WebM | Core | VP8/VP9/AV1/Opus/Vorbis container |
| Ogg | Core | Vorbis/Opus/FLAC container |
| WAV | Core | PCM audio |
| MPEG-TS | Compat | HLS segments |
| AVI | Compat | Legacy |
| FLV | Not supported | Flash legacy |

### 20.2.3 Decoder Selection Strategy

```rust
pub enum DecoderStrategy {
    /// Pure software decoders (Rust/C libraries). Maximum portability.
    SoftwareOnly,
    /// Prefer platform hardware decoders, fallback to software.
    PlatformPreferred,
    /// Per-codec selection (default).
    Mixed(HashMap<Codec, DecoderPreference>),
}

pub enum DecoderPreference {
    Software,
    Platform,
    /// Try platform first, fallback to software
    PlatformWithFallback,
}
```

Default strategy (`Mixed`):
- VP8/VP9: Software (libvpx). Platform HW decode available on some GPUs; use if detected.
- AV1: Software (dav1d). Platform HW decode on newer GPUs (Intel Gen12+, Apple M3+); use if detected.
- H.264/H.265: Platform only. No software decoder bundled (patent licensing).
- Opus/Vorbis/FLAC/MP3: Always software (lightweight, no patent issues, CPU cost negligible).
- AAC: Platform only.

### 20.2.4 Platform Decoder Adapters

```rust
/// macOS / iOS
pub struct AvFoundationDecoder {
    // VTDecompressionSession for video
    // AudioConverter for audio
}

/// Windows
pub struct MediaFoundationDecoder {
    // IMFTransform for video and audio
}

/// Linux
pub struct VaApiDecoder {
    // VA-API for hardware-accelerated video decode
    // Requires GPU Process coordination for texture import
}

/// Fallback for Linux audio
pub struct PulseAudioDecoder {
    // GStreamer pipeline for AAC on Linux where no VA-API audio exists
}
```

Each adapter implements the `MediaDecoder` trait and is selected based on the runtime platform and available hardware.

## 20.3 Media Demuxer

The demuxer parses container formats and extracts encoded packets:

```rust
pub trait MediaDemuxer: Send {
    /// Open a media source and read container headers.
    fn open(&mut self, source: MediaSource) -> Result<MediaInfo, DemuxError>;

    /// Read the next encoded packet.
    fn read_packet(&mut self) -> Result<Option<EncodedPacket>, DemuxError>;

    /// Seek to a timestamp. Returns the actual seek position.
    fn seek(&mut self, timestamp: Duration) -> Result<Duration, DemuxError>;
}

pub struct MediaInfo {
    pub duration: Option<Duration>,
    pub tracks: Vec<TrackInfo>,
    pub is_seekable: bool,
}

pub struct TrackInfo {
    pub id: TrackId,
    pub kind: TrackKind,
    pub codec: Codec,
    pub config: DecoderConfig,
    pub language: Option<String>,
    pub label: Option<String>,
}

pub enum TrackKind {
    Video,
    Audio,
    Subtitle,
}

pub enum MediaSource {
    /// URL-based: data arrives via fetch (Ch. 10) as ByteStream (Ch. 21)
    Stream(ByteStream),
    /// MSE: SourceBuffer provides segments
    Mse(MseSource),
}
```

Demuxer implementations:

| Container | Implementation | Notes |
| --- | --- | --- |
| MP4 / fMP4 | mp4parse (Mozilla, pure Rust) | Also handles MPEG-DASH segments |
| WebM | Rust WebM parser | Matroska subset |
| Ogg | ogg crate (pure Rust) | Streaming-friendly |
| WAV | Inline parser | Trivial format |
| MPEG-TS (compat) | TS parser | HLS segments |

## 20.4 Media Player

### 20.4.1 Architecture

MediaPlayer is the central coordinator for a single `<video>` or `<audio>` element:

```rust
pub struct MediaPlayer {
    /// Current state
    state: PlayerState,
    /// Demuxer running on a dedicated thread
    demuxer: Box<dyn MediaDemuxer>,
    /// Proxy to decoder process
    decoder_proxy: DecoderProxy,
    /// Audio output
    audio_renderer: AudioRenderer,
    /// Video frame scheduler
    video_renderer: VideoRenderer,
    /// A/V sync clock
    clock: MediaClock,
    /// Buffered time ranges
    buffered: TimeRanges,
    /// Playback rate
    playback_rate: f64,
    /// Volume and mute state
    volume: f64,
    muted: bool,
}

pub enum PlayerState {
    Idle,
    Loading,
    Ready,
    Playing,
    Paused,
    Seeking(Duration),
    Ended,
    Error(MediaError),
}
```

### 20.4.2 Playback Pipeline

```
Data source (network / MSE)
  │
  ▼
MediaDemuxer (container parsing)
  │  extracts encoded audio/video packets
  ▼
DecoderProxy ──── IPC ───► Decoder Process
  │                         │  decode audio/video
  │  ◄── decoded frames ───┤
  ▼
┌─────────────┐  ┌──────────────┐
│AudioRenderer│  │VideoRenderer │
│ (audio      │  │ (frame queue │
│  device or  │  │  + compositor│
│  Web Audio) │  │  layer)      │
└──────┬──────┘  └──────┬───────┘
       │                │
       ▼                ▼
  Audio output     Compositor (Ch. 15)
  (platform API)   (video texture layer)
```

### 20.4.3 A/V Synchronization

The MediaClock drives synchronization between audio and video:

```rust
pub struct MediaClock {
    /// Reference clock: audio device position (most accurate)
    audio_position: Arc<AtomicU64>,  // microseconds
    /// Fallback: system timer when audio is muted/absent
    system_base: Instant,
    system_offset: Duration,
    /// Playback rate multiplier
    rate: f64,
}

impl MediaClock {
    /// Current media time.
    pub fn position(&self) -> Duration {
        // Prefer audio device clock (hardware-driven, drift-free)
        // Fallback to system clock if no audio output
        let audio_pos = self.audio_position.load(Ordering::Relaxed);
        if audio_pos > 0 {
            Duration::from_micros(audio_pos)
        } else {
            let elapsed = self.system_base.elapsed();
            self.system_offset + elapsed.mul_f64(self.rate)
        }
    }
}
```

The VideoRenderer uses the clock to schedule frame presentation:

```rust
impl VideoRenderer {
    fn select_frame(&self, clock: &MediaClock) -> Option<&VideoFrame> {
        let target = clock.position();

        // Find the frame with timestamp closest to (but not after) target
        // Drop late frames (> 1 frame duration behind)
        // Hold early frames until their timestamp
        self.frame_queue.iter()
            .filter(|f| f.timestamp <= target)
            .last()
    }
}
```

Audio is the master clock. Video adjusts to match audio timing. If video falls behind (decoder too slow), frames are dropped. If video is ahead, the renderer holds the current frame.

### 20.4.4 Seeking

```
User seeks to time T
  │
  ├─ 1. Flush decoder pipeline (discard pending frames)
  ├─ 2. Demuxer seeks to nearest keyframe before T
  ├─ 3. Resume decoding from keyframe
  ├─ 4. Decode (discarding frames before T)
  ├─ 5. First frame at or after T → display
  └─ 6. Resume normal playback
```

For fragmented MP4 (DASH/HLS), seeking may require fetching a different segment via the network.

## 20.5 Process Architecture

### 20.5.1 Decoder Process

Media decoding runs in a sandboxed Decoder Process (extending Ch. 5's process model):

```rust
pub enum ProcessType {
    Browser,
    Renderer,
    Network,
    Gpu,
    Decoder,   // ← new
}
```

Rationale for process isolation:
- Codec libraries (especially C-based: libvpx, dav1d, platform APIs) are high-risk for memory safety bugs.
- A decoder crash affects only media playback, not the page or browser.
- Sandboxing limits decoder process capabilities (no network, no filesystem, no GPU — unless HW decode).

**IPC protocol:**

```rust
// Renderer → Decoder
pub enum DecoderRequest {
    Configure(DecoderConfig),
    Decode(EncodedPacket),         // shared memory buffer
    Flush,
    Shutdown,
}

// Decoder → Renderer
pub enum DecoderResponse {
    Configured(DecoderCapabilities),
    Frame(DecodedFrame),           // shared memory buffer (video) or inline (audio)
    Flushed,
    Error(DecoderError),
}
```

Encoded packets and decoded video frames are transferred via shared memory to avoid copying large buffers across process boundaries.

### 20.5.2 Hardware-Accelerated Decode

For hardware decode (VA-API, VideoToolbox, DXVA):

```
Renderer Process              Decoder Process              GPU Process
    │                             │                            │
    ├── Decode(packet) ──────────►│                            │
    │                             ├── HW decode request ──────►│
    │                             │                            ├── GPU decode
    │                             │                            │   (dedicated HW unit)
    │                             │◄── texture handle ─────────┤
    │◄── Frame(hw_texture) ───────┤                            │
    │                                                          │
    ├── compositor imports texture directly ───────────────────►│
    │   (zero-copy: decoded texture → compositor layer)        │
```

Hardware-decoded frames arrive as GPU texture handles. The compositor (Ch. 15) imports these textures directly into the composited scene — no CPU readback. This is the optimal path for video playback power efficiency.

### 20.5.3 elidex-app SingleProcess

In elidex-app SingleProcess mode, the decoder runs in-process on a dedicated thread pool. No IPC overhead, but no process isolation. Acceptable for trusted app content.

## 20.6 HTMLMediaElement

### 20.6.1 ECS Representation

`<video>` and `<audio>` elements are ECS entities with media-specific components:

```rust
pub struct MediaElement {
    pub player: MediaPlayer,
    pub network_state: NetworkState,
    pub ready_state: ReadyState,
    pub current_src: String,
}

pub struct VideoSurface {
    /// Layer in the compositor (Ch. 15) for video frame display.
    pub layer_id: LayerId,
    /// Intrinsic video dimensions (affects CSS layout as replaced element).
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,
}
```

The HTMLMediaElement API (`play()`, `pause()`, `currentTime`, `volume`, events) is exposed through ScriptSession (Ch. 13) and maps to MediaPlayer operations.

### 20.6.2 Video Display

Video frames are composited as a dedicated layer (Ch. 15 §15.4):

```
HTML LayoutSystem
  │  assigns CSS box to <video> (e.g., 640×360 at position (100, 200))
  ▼
VideoRenderer
  │  selects current frame from decoded queue
  │  uploads frame texture (or uses HW texture directly)
  ▼
Layer Tree (Ch. 15 §15.4)
  │  video layer at video element position
  │  object-fit CSS property controls scaling within box
  ▼
Compositor
  │  composites video layer with other content
```

`object-fit` (contain, cover, fill, none, scale-down) is applied by the compositor when mapping the video texture to the CSS box.

### 20.6.3 Poster and Controls

- **Poster**: `poster` attribute loads an image (Ch. 18 pipeline) displayed before playback starts.
- **Controls**: Default browser controls are rendered by the BrowserShell (Ch. 24) as an overlay. For elidex-app, controls are the app's responsibility.

## 20.7 MediaSource Extensions (MSE)

MSE enables JavaScript-driven adaptive streaming (DASH, HLS via JavaScript):

```rust
pub struct MediaSourceHandle {
    /// Active SourceBuffers
    source_buffers: Vec<SourceBuffer>,
    /// Duration set by JavaScript
    duration: f64,
    /// Ready state
    ready_state: MseReadyState,
}

pub struct SourceBuffer {
    pub id: SourceBufferId,
    pub mime_type: String,
    pub codec: Codec,
    /// Buffered time ranges
    pub buffered: TimeRanges,
    /// Append buffer: JS provides encoded segments
    pub pending_append: VecDeque<Bytes>,
}
```

MSE flow:
```
JavaScript (adaptive bitrate logic)
  │
  ├── new MediaSource()
  ├── video.src = URL.createObjectURL(mediaSource)
  ├── sourceBuffer = mediaSource.addSourceBuffer('video/mp4; codecs="avc1.42E01E"')
  │
  │   [fetch segment from CDN]
  ├── sourceBuffer.appendBuffer(segment)
  │     → demuxer parses segment
  │     → packets queued for decoder
  │
  │   [adaptive: switch quality]
  ├── sourceBuffer.appendBuffer(higher_quality_segment)
  │
  └── mediaSource.endOfStream()
```

SourceBuffer data flows into the same demuxer → decoder → renderer pipeline as standard media. The only difference is the data source: JS-provided segments instead of a continuous network stream.

## 20.8 Encrypted Media Extensions (EME)

### 20.8.1 Architecture

EME provides a standardized API for DRM-protected content. Elidex defines the EME interface and CDM plugin slot, but does not ship a CDM implementation initially:

```rust
pub trait ContentDecryptionModule: Send {
    /// Create a session for license exchange.
    fn create_session(&mut self, session_type: SessionType) -> Result<SessionId, CdmError>;

    /// Generate a license request.
    fn generate_request(
        &mut self,
        session: SessionId,
        init_data_type: &str,
        init_data: &[u8],
    ) -> Result<Bytes, CdmError>;

    /// Provide a license response from the license server.
    fn update_session(
        &mut self,
        session: SessionId,
        response: &[u8],
    ) -> Result<(), CdmError>;

    /// Decrypt an encrypted media sample.
    fn decrypt(
        &mut self,
        encrypted: &[u8],
        iv: &[u8],
        key_id: &[u8],
        subsample_info: &[SubsampleEntry],
    ) -> Result<Bytes, CdmError>;

    /// Close and release a session.
    fn close_session(&mut self, session: SessionId) -> Result<(), CdmError>;
}

pub enum SessionType {
    Temporary,
    PersistentLicense,
}
```

### 20.8.2 CDM Integration Flow

```
JavaScript                    Renderer Process         CDM Process (future)
    │                             │                        │
    ├── navigator.requestMediaKeySystemAccess()           │
    │   ("com.widevine.alpha", configs)                   │
    │                             │                        │
    │   [elidex checks if CDM plugin is available]        │
    │                             │                        │
    ├── mediaKeys.createSession() │                        │
    ├── session.generateRequest() ─►│                      │
    │                             ├─── IPC ───────────────►│
    │                             │                        ├── generate_request()
    │◄── "message" event ─────────┤◄── license request ────┤
    │                             │                        │
    │   [app sends request to license server]              │
    │   [app receives license response]                    │
    │                             │                        │
    ├── session.update(response) ─►│                       │
    │                             ├─── IPC ───────────────►│
    │                             │                        ├── update_session()
    │                             │                        ├── (keys now available)
    │                             │                        │
    │   [encrypted packets arrive]│                        │
    │                             ├── decrypt(packet) ────►│
    │                             │◄── decrypted packet ───┤
    │                             ├── decode as normal     │
```

### 20.8.3 Initial Status

Elidex v1 ships without a CDM. The EME JavaScript API returns `NotSupportedError` for all key systems. The CDM trait and CDM Process architecture are defined so that a CDM can be integrated in the future without architectural changes. This affects Netflix, Disney+, Spotify, and other DRM-dependent services.

Clear Key (a trivial, non-proprietary CDM for testing) may be implemented as a reference CDM.

## 20.9 Web Audio API

### 20.9.1 Overview

The Web Audio API provides a graph-based audio processing pipeline for games, music applications, and real-time audio effects.

```
AudioContext
  │
  ├── Source nodes
  │   ├── AudioBufferSourceNode (decoded audio buffer)
  │   ├── MediaElementAudioSourceNode (<audio>/<video>)
  │   ├── MediaStreamAudioSourceNode (getUserMedia)
  │   └── OscillatorNode (generated waveform)
  │
  ├── Processing nodes
  │   ├── GainNode
  │   ├── BiquadFilterNode
  │   ├── ConvolverNode (reverb via impulse response)
  │   ├── DelayNode
  │   ├── DynamicsCompressorNode
  │   ├── WaveShaperNode (distortion)
  │   ├── StereoPannerNode
  │   ├── AnalyserNode (FFT for visualization)
  │   ├── ChannelSplitterNode / ChannelMergerNode
  │   └── AudioWorkletNode (custom processing in Wasm/JS)
  │
  └── Destination
      └── AudioDestinationNode → platform audio output
```

### 20.9.2 Audio Thread

Web Audio requires a dedicated real-time audio thread (extending Ch. 6's thread model):

```rust
pub struct AudioThread {
    /// Audio graph evaluated on this thread.
    graph: AudioGraph,
    /// Audio device callback period (typically 128–1024 samples).
    buffer_size: usize,
    /// Sample rate (typically 44100 or 48000 Hz).
    sample_rate: u32,
}
```

The audio thread has real-time constraints:
- Must fill the audio buffer within the callback deadline (e.g., ~2.9ms for 128 samples at 44.1kHz).
- No allocation, no locks, no I/O on the hot path.
- Lock-free ring buffers for communication with the main thread.

```rust
/// Lock-free command queue: main thread → audio thread
pub struct AudioCommandQueue {
    queue: crossbeam::queue::SegQueue<AudioCommand>,
}

pub enum AudioCommand {
    Connect { source: NodeId, destination: NodeId, output: u32, input: u32 },
    Disconnect { source: NodeId, destination: NodeId },
    SetParam { node: NodeId, param: ParamId, value: f32, time: f64 },
    ScheduleParam { node: NodeId, param: ParamId, automation: ParamAutomation },
    StartNode { node: NodeId, when: f64 },
    StopNode { node: NodeId, when: f64 },
}
```

### 20.9.3 AudioWorklet

AudioWorklet allows user-defined audio processing that runs on the audio thread:

```rust
pub struct AudioWorkletProcessor {
    /// Wasm or JS module loaded into audio thread context.
    /// Processes 128-sample render quanta.
    module: WorkletModule,
}
```

AudioWorklet runs on the audio thread, not on a separate Worker thread. It must meet the same real-time constraints. The AudioWorkletGlobalScope is a restricted environment: no DOM, no network, limited API surface.

Communication between the main thread and AudioWorklet is via MessagePort (lock-free ring buffer underneath).

### 20.9.4 Audio Graph Evaluation

```rust
impl AudioGraph {
    /// Evaluate the graph for one render quantum (128 samples).
    /// Called by the audio device callback on the audio thread.
    pub fn render(&mut self, output: &mut [f32]) {
        // 1. Process commands from main thread (lock-free queue)
        self.process_commands();

        // 2. Topological sort of nodes (cached, invalidated on connect/disconnect)
        // 3. Evaluate each node in order
        for node_id in &self.evaluation_order {
            let node = &mut self.nodes[*node_id];
            // Mix input buffers
            let input = self.collect_inputs(*node_id);
            // Process
            node.process(&input, &mut self.scratch_buffer);
            // Store output for downstream nodes
            self.outputs[*node_id] = self.scratch_buffer.clone();
        }

        // 4. Copy destination node output to device buffer
        output.copy_from_slice(&self.outputs[self.destination]);
    }
}
```

### 20.9.5 AudioParam Automation

Web Audio AudioParams support scheduled automation:

```rust
pub enum ParamAutomation {
    SetValueAtTime { value: f32, time: f64 },
    LinearRampToValueAtTime { value: f32, end_time: f64 },
    ExponentialRampToValueAtTime { value: f32, end_time: f64 },
    SetTargetAtTime { target: f32, start_time: f64, time_constant: f64 },
    SetValueCurveAtTime { values: Vec<f32>, start_time: f64, duration: f64 },
}
```

Automation is evaluated sample-by-sample on the audio thread for sample-accurate timing.

## 20.10 Media Capture

### 20.10.1 getUserMedia / getDisplayMedia

Media capture APIs provide access to camera, microphone, and screen content:

```rust
pub struct MediaStream {
    pub id: String,
    pub tracks: Vec<MediaStreamTrack>,
}

pub struct MediaStreamTrack {
    pub id: String,
    pub kind: TrackKind,
    pub label: String,
    pub constraints: MediaTrackConstraints,
    pub state: TrackState,
}

pub enum TrackState {
    Live,
    Ended,
}
```

Permission flow integrates with Ch. 8:
- `getUserMedia({ video: true })` → Camera permission prompt
- `getUserMedia({ audio: true })` → Microphone permission prompt
- `getDisplayMedia()` → ScreenCapture permission + platform screen picker

### 20.10.2 MediaStream Integration

MediaStream tracks can be routed to:

| Destination | Mechanism |
| --- | --- |
| `<video>` element | `video.srcObject = stream` |
| Web Audio | `audioContext.createMediaStreamSource(stream)` |
| MediaRecorder | Record to Blob (via BlobStore, Ch. 21) |
| WebRTC (future) | `peerConnection.addTrack(track, stream)` |
| Canvas | `canvas.captureStream()` / `ctx.drawImage(video, ...)` |

### 20.10.3 Camera/Microphone Access

Camera and microphone access is handled by the Browser Process (privileged, unsandboxed) and streamed to the Renderer:

```
Browser Process                          Renderer Process
    │                                        │
    ├── Open camera device                   │
    ├── Capture frames → shared memory ─────►│
    │                                        ├── MediaStream track
    │                                        ├── (feed to <video> or Web Audio)
    │                                        │
    ├── Open microphone                      │
    ├── Capture audio → shared memory ──────►│
    │                                        ├── MediaStream track
```

## 20.11 WebRTC (Interface Definition)

WebRTC is a large standalone subsystem. This section defines the integration interface only; full design is deferred to a future chapter.

### 20.11.1 Integration Points

```rust
/// WebRTC integration surface with the media pipeline.
pub trait RtcMediaInterface {
    /// Add a local MediaStreamTrack to a peer connection.
    fn add_track(&mut self, track: &MediaStreamTrack, stream: &MediaStream);

    /// Receive a remote track from a peer connection.
    fn on_track(&self) -> Receiver<(MediaStreamTrack, MediaStream)>;

    /// Get stats for monitoring.
    fn get_stats(&self) -> RtcStats;
}
```

### 20.11.2 Scope Boundary

| In scope (this chapter) | Deferred (future WebRTC chapter) |
| --- | --- |
| MediaStream / MediaStreamTrack model | ICE / STUN / TURN |
| getUserMedia / getDisplayMedia | SDP negotiation |
| MediaStream → `<video>`, Web Audio, canvas | SRTP / DTLS encryption |
| MediaRecorder | SCTP data channels |
| — | RTCPeerConnection full lifecycle |
| — | Codec negotiation (SDP codec params) |
| — | Bandwidth estimation / congestion control |

## 20.12 Platform Audio Output

### 20.12.1 Audio Output Abstraction

```rust
pub trait AudioOutput: Send {
    /// Open audio device with desired configuration.
    fn open(&mut self, config: AudioOutputConfig) -> Result<(), AudioError>;

    /// Start playback. The callback will be called periodically to fill the buffer.
    fn start(&mut self, callback: AudioCallback) -> Result<(), AudioError>;

    /// Stop playback.
    fn stop(&mut self) -> Result<(), AudioError>;

    /// Query device capabilities.
    fn capabilities(&self) -> AudioDeviceCapabilities;
}

pub struct AudioOutputConfig {
    pub sample_rate: u32,
    pub channels: u32,
    pub buffer_size: u32,  // samples per callback
}

pub type AudioCallback = Box<dyn FnMut(&mut [f32]) + Send>;
```

Platform implementations:

| Platform | API | Notes |
| --- | --- | --- |
| macOS / iOS | CoreAudio (AudioUnit) | Low-latency, hardware-mixed |
| Windows | WASAPI | Exclusive and shared mode |
| Linux | PipeWire / PulseAudio | PipeWire preferred (lower latency) |
| Android | AAudio / Oboe | Oboe provides cross-API abstraction |

### 20.12.2 Audio Routing

Both HTMLMediaElement and Web Audio feed into the same AudioOutput:

```
HTMLMediaElement AudioRenderer ──►┐
                                  ├── AudioMixer ──► AudioOutput ──► speakers
Web Audio AudioDestinationNode ──►┘
```

The AudioMixer sums multiple audio sources. Per-tab muting and volume control are applied before mixing.

## 20.13 elidex-app Media

| Aspect | elidex-browser | elidex-app |
| --- | --- | --- |
| HTMLMediaElement | Full support | Full support |
| MSE | Full support | Full support |
| EME/DRM | CDM plugin slot (v1: empty) | CDM plugin slot (v1: empty) |
| Web Audio | Full support | Full support |
| getUserMedia | Permission prompt | `App::grant(Camera)`, `App::grant(Microphone)` |
| getDisplayMedia | Permission prompt | `App::grant(ScreenCapture)` |
| WebRTC | Future | Future |
| Codec strategy | Mixed (default) | Configurable per-app |
| Decoder process | Separate process | In-process (SingleProcess mode) |
| Audio output | Shared with other tabs | App-exclusive audio session |

elidex-app can configure codec availability at build time. Apps that don't use media can exclude the entire media pipeline to reduce binary size. Apps that need specific codecs (e.g., medical imaging with specialized formats) can register custom MediaDecoder implementations.
