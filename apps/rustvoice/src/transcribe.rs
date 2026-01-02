use anyhow::{Result, anyhow};
use candle_core as candle;
use candle_core::{Device, Tensor};
use candle_transformers::models::whisper::{self as m, Config, audio};
use crate::decoder::{self, Decoder, Model, Task};
use crate::model::ModelPaths;
use crossbeam_channel::{Receiver, Sender};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use tokenizers::Tokenizer;
use byteorder::{ByteOrder, LittleEndian};

pub struct TranscriptionResult {
    pub pending: String,
    pub confirmed: String,
}

pub struct WhisperTranscriber {
    model: Model,
    tokenizer: Tokenizer,
    mel_filters: Vec<f32>,
    device: Device,
    config: Config,
}

impl WhisperTranscriber {
    pub fn new(paths: ModelPaths, mel_filters_path: PathBuf) -> Result<Self> {
        let device = Device::new_cuda(0).unwrap_or(Device::Cpu);
        log::info!("Using device: {:?}", device);

        let config: Config = serde_json::from_str(&std::fs::read_to_string(&paths.config)?)?;
        let tokenizer = Tokenizer::from_file(&paths.tokenizer).map_err(|e| anyhow!(e))?;
        
        // Load model weights
        let vb = unsafe { 
            candle_nn::VarBuilder::from_mmaped_safetensors(&[paths.model], m::DTYPE, &device)? 
        };
        let model = Model::Normal(m::model::Whisper::load(&vb, config.clone())?);

        // Load mel filters
        let mel_bytes = std::fs::read(&mel_filters_path)?;
        let mut mel_filters = vec![0f32; mel_bytes.len() / 4];
        LittleEndian::read_f32_into(&mel_bytes, &mut mel_filters);

        Ok(Self {
            model,
            tokenizer,
            mel_filters,
            device,
            config,
        })
    }

    pub fn start(self: Arc<Self>, rx: Receiver<Vec<f32>>, tx: Sender<TranscriptionResult>) {
        thread::spawn(move || {
            let mut audio_buffer: Vec<f32> = Vec::new();
            let sample_rate = m::SAMPLE_RATE as usize; // 16000
            let window_size = sample_rate * 30; // 30 seconds
            let step_size = sample_rate * 3; // 3 seconds (process every 3s of new audio? No, process frequently)
            
            // For real-time, we want to process every ~0.5s, but look at context.
            // Simplified: Accumulate. When we have > 1s, transcribe.
            // To prevent infinite buffer, keep last 30s.
            
            loop {
                // Non-blocking drain
                while let Ok(chunk) = rx.try_recv() {
                    audio_buffer.extend_from_slice(&chunk);
                    log::debug!("Received audio chunk, buffer now {} samples", audio_buffer.len());
                }

                // If we have enough data to be worth transcribing (> 1s)
                if audio_buffer.len() > sample_rate {
                    // Cap buffer at 30s
                    if audio_buffer.len() > window_size {
                        let excess = audio_buffer.len() - window_size;
                        audio_buffer.drain(0..excess);
                    }

                    // Transcribe
                    match self.transcribe_segment(&audio_buffer) {
                        Ok(text) => {
                            if !text.trim().is_empty() {
                                let _ = tx.send(TranscriptionResult {
                                    pending: text.clone(),
                                    confirmed: "".to_string(), // TODO: Implement confirmation logic (LocalAgreement)
                                });
                            }
                        }
                        Err(e) => log::error!("Transcription error: {}", e),
                    }
                }

                // Sleep briefly to avoid busy loop
                thread::sleep(std::time::Duration::from_millis(200));
            }
        });
    }

    fn transcribe_segment(&self, pcm_data: &[f32]) -> Result<String> {
        let mel = audio::pcm_to_mel(&self.config, pcm_data, &self.mel_filters);
        let mel_len = mel.len();
        log::debug!("Transcribing {} samples -> {} mel bins", pcm_data.len(), mel_len / self.config.num_mel_bins);
        let mel_tensor = Tensor::from_vec(
            mel,
            (1, self.config.num_mel_bins, mel_len / self.config.num_mel_bins),
            &self.device,
        )?;

        // Create a new decoder for this segment
        // We use default seed for deterministic results? Or random?
        let mut decoder = Decoder::new(
            match &self.model {
                Model::Normal(m) => Model::Normal(m.clone()), // Clone wrapper, cheap for Arc weights?
                Model::Quantized(m) => Model::Quantized(m.clone())
            }, 
            self.tokenizer.clone(), 
            299792458, // Seed
            &self.device, 
            None, 
            Some(Task::Transcribe), 
            true, // Timestamps 
            None, 
            false // Verbose
        )?;

        let segments = match decoder.run(&mel_tensor) {
            Ok(segs) => segs,
            Err(e) => {
                log::error!("Decoder run failed: {:?}", e);
                return Err(e.into());
            }
        };
        
        let mut full_text = String::new();
        for seg in segments {
            full_text.push_str(&seg.dr.text);
            full_text.push(' ');
        }
        
        Ok(full_text.trim().to_string())
    }
    pub fn get_device_name(&self) -> String {
        format!("{:?}", self.device)
    }
}
