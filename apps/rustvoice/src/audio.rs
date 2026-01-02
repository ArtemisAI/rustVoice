//! Audio capture module for voice transcription
//! 
//! Uses cpal to capture audio from the default microphone,
//! resamples to 16kHz mono (required by Whisper), and sends
//! audio chunks for transcription.

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, Stream, StreamConfig};
use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use rubato::{FftFixedIn, Resampler};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Target sample rate for Whisper (16kHz)
const WHISPER_SAMPLE_RATE: u32 = 16000;

/// Audio chunk duration in milliseconds
const CHUNK_DURATION_MS: u32 = 500;

/// Audio capture handle
pub struct AudioCapture {
    stream: Option<Stream>,
    is_recording: Arc<AtomicBool>,
    audio_rx: Receiver<Vec<f32>>,
    _audio_tx: Sender<Vec<f32>>,
    current_device_name: Option<String>,
}

/// Get list of available input devices
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    let mut devices = Vec::new();
    
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                devices.push(name);
            }
        }
    }
    
    devices
}

/// Get the default input device name
pub fn get_default_input_device_name() -> Option<String> {
    let host = cpal::default_host();
    host.default_input_device().and_then(|d| d.name().ok())
}

impl AudioCapture {
    /// Create a new audio capture instance
    pub fn new() -> Result<Self> {
        let (audio_tx, audio_rx) = bounded(16);
        
        Ok(Self {
            stream: None,
            is_recording: Arc::new(AtomicBool::new(false)),
            audio_rx,
            _audio_tx: audio_tx,
            current_device_name: None,
        })
    }
    
    /// Get current device name
    pub fn get_current_device(&self) -> Option<&str> {
        self.current_device_name.as_deref()
    }
    
    /// Start recording from a specific device by name (or default if None)
    pub fn start_with_device(&mut self, device_name: Option<&str>) -> Result<()> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Ok(()); // Already recording
        }
        
        let host = cpal::default_host();
        
        let device = if let Some(name) = device_name {
            // Find specific device
            let mut found_device = None;
            if let Ok(devices) = host.input_devices() {
                for d in devices {
                    if d.name().map(|n| n == name).unwrap_or(false) {
                        found_device = Some(d);
                        break;
                    }
                }
            }
            found_device.ok_or_else(|| anyhow!("Device not found: {}", name))?
        } else {
            host.default_input_device()
                .ok_or_else(|| anyhow!("No input device available"))?
        };
        
        self.current_device_name = device.name().ok();
        log::info!("Using input device: {}", device.name().unwrap_or_default());
        
        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        
        log::info!("Input config: {}Hz, {} channels, {:?}", 
                   sample_rate, channels, config.sample_format());
        
        // Calculate buffer size for chunk duration
        let samples_per_chunk = (WHISPER_SAMPLE_RATE * CHUNK_DURATION_MS / 1000) as usize;
        
        // Create resampler if needed
        let resampler = if sample_rate != WHISPER_SAMPLE_RATE {
            Some(Arc::new(Mutex::new(
                FftFixedIn::<f32>::new(
                    sample_rate as usize,
                    WHISPER_SAMPLE_RATE as usize,
                    1024,
                    2,
                    1, // Mono output
                )?
            )))
        } else {
            None
        };
        
        let audio_tx = self._audio_tx.clone();
        let is_recording = self.is_recording.clone();
        let buffer = Arc::new(Mutex::new(Vec::<f32>::with_capacity(samples_per_chunk * 2)));
        
        let buffer_clone = buffer.clone();
        // Buffer for the Resampler (needs fixed chunk input, e.g., 1024)
        let input_buffer = Arc::new(Mutex::new(Vec::<f32>::with_capacity(2048)));
        let input_buffer_clone = input_buffer.clone();

        let resampler_clone = resampler.clone();
        
        let err_fn = |err| log::error!("Audio stream error: {}", err);
        
        let stream = match config.sample_format() {
            SampleFormat::F32 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &_| {
                        process_audio_data(
                            data,
                            channels,
                            sample_rate,
                            &input_buffer_clone, // New buffer for accumulating 1024 chunks
                            &buffer_clone,
                            &resampler_clone,
                            &audio_tx,
                            samples_per_chunk,
                        );
                    },
                    err_fn,
                    None,
                )?
            }
            SampleFormat::I16 => {
                let buffer_clone = buffer.clone();
                let input_buffer_clone = input_buffer.clone();
                let resampler_clone = resampler.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &_| {
                        let float_data: Vec<f32> = data.iter()
                            .map(|&s| s as f32 / i16::MAX as f32)
                            .collect();
                        process_audio_data(
                            &float_data,
                            channels,
                            sample_rate,
                            &input_buffer_clone,
                            &buffer_clone,
                            &resampler_clone,
                            &audio_tx,
                            samples_per_chunk,
                        );
                    },
                    err_fn,
                    None,
                )?
            }
            sample_format => {
                return Err(anyhow!("Unsupported sample format: {:?}", sample_format));
            }
        };
        
        stream.play()?;
        self.stream = Some(stream);
        self.is_recording.store(true, Ordering::Relaxed);
        
        log::info!("Audio capture started");
        Ok(())
    }
    
    /// Start recording from the default microphone
    pub fn start(&mut self) -> Result<()> {
        self.start_with_device(None)
    }
    
    /// Stop recording
    pub fn stop(&mut self) {
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
        self.is_recording.store(false, Ordering::Relaxed);
        log::info!("Audio capture stopped");
    }
    
    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }
    
    /// Get receiver for audio chunks
    pub fn audio_receiver(&self) -> Receiver<Vec<f32>> {
        self.audio_rx.clone()
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Process incoming audio data, resample to 16kHz mono, and send chunks
fn process_audio_data(
    data: &[f32],
    channels: usize,
    _sample_rate: u32,
    input_buffer: &Arc<Mutex<Vec<f32>>>, // Accumulator for resampler input
    buffer: &Arc<Mutex<Vec<f32>>>,       // Accumulator for Whisper chunks
    resampler: &Option<Arc<Mutex<FftFixedIn<f32>>>>,
    audio_tx: &Sender<Vec<f32>>,
    samples_per_chunk: usize,
) {
    // Convert to mono by averaging channels
    let mono: Vec<f32> = if channels > 1 {
        data.chunks(channels)
            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
            .collect()
    } else {
        data.to_vec()
    };
    
    // Resample if necessary
    if let Some(resampler) = resampler {
        // 1. Append new data to input_buffer
        {
            let mut in_buf = input_buffer.lock();
            in_buf.extend(mono);
        } // Release input_buffer lock
        
        // 2. Process in chunks of 1024 (FftFixedIn requirement)
        let input_needed = 1024; // Fixed size for FftFixedIn
        
        loop {
            // Check if we have enough data (acquire and release lock quickly)
            let has_enough = {
                let in_buf = input_buffer.lock();
                in_buf.len() >= input_needed
            };
            
            if !has_enough {
                break;
            }
            
            // Extract one chunk to process
            let chunk = {
                let mut in_buf = input_buffer.lock();
                in_buf.drain(..input_needed).collect::<Vec<f32>>()
            }; // Release input_buffer lock before resampling
            
            // Resample the chunk
            let waves_in = vec![chunk];
            let processed = {
                let mut resampler_lock = resampler.lock();
                match resampler_lock.process(&waves_in, None) {
                    Ok(output) => output.into_iter().next().unwrap_or_default(),
                    Err(e) => {
                        log::error!("Resampling error: {}", e);
                        continue; // Skip this chunk on error
                    }
                }
            }; // Release resampler lock
            
            // Add resampled data to output buffer
            {
                let mut out_buf = buffer.lock();
                out_buf.extend(processed);
            } // Release output buffer lock
        }
    } else {
        // No resampling needed, pass through
        let mut buf = buffer.lock();
        buf.extend(mono);
    }
    
    // Check output buffer for full chunks to send to Whisper
    let mut buf = buffer.lock();
    if buf.len() >= samples_per_chunk {
        let chunk: Vec<f32> = buf.drain(..samples_per_chunk).collect();
        if audio_tx.try_send(chunk).is_err() {
            log::warn!("Audio buffer full, dropping chunk");
        }
    }
}

/// Decode an audio file to 16kHz mono (F32) using Symphonia
pub fn decode_audio_file(path: &std::path::Path) -> anyhow::Result<Vec<f32>> {
    use symphonia::core::audio::Signal;
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let src = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("no supported audio tracks"))?;

    let dec_opts: DecoderOptions = Default::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)?;

    let track_id = track.id;
    let mut samples: Vec<f32> = Vec::new();

    let source_sample_rate = track.codec_params.sample_rate.ok_or_else(|| anyhow!("val"))?;

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                let duration = decoded.capacity();
                
                if spec.channels.count() == 1 {
                    if let symphonia::core::audio::AudioBufferRef::F32(buf) = &decoded {
                         samples.extend_from_slice(buf.planes().planes()[0]);
                    } else {
                        let mut buf = symphonia::core::audio::AudioBuffer::<f32>::new(duration as u64, spec);
                        decoded.convert(&mut buf);
                        samples.extend_from_slice(buf.planes().planes()[0]);
                    }
                } else {
                     let mut buf = symphonia::core::audio::AudioBuffer::<f32>::new(duration as u64, spec);
                     decoded.convert(&mut buf);
                     let planes = buf.planes();
                     let p0 = planes.planes()[0];
                     samples.extend_from_slice(p0);
                }
            }
            Err(e) => {
                 log::warn!("Decode packet error: {}", e);
                 break;
            }
        }
    }
    
    // Resample to 16000 Hz if needed
    if source_sample_rate != 16000 {
        use rubato::{SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};

        let params = SincInterpolationParameters {
            sinc_len: 128,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            window: WindowFunction::BlackmanHarris2,
            oversampling_factor: 128,
        };
        
        let mut resampler = SincFixedIn::<f32>::new(
            16000.0 / source_sample_rate as f64,
            2.0,
            params,
            samples.len(),
            1
        ).map_err(|_| anyhow!("resampler init failed"))?;
        
        let waves_in = vec![samples];
        let waves_out = resampler.process(&waves_in, None).map_err(|_| anyhow!("resampling failed"))?;
        
        return Ok(waves_out[0].clone());
    }

    Ok(samples)
}

