//! Whisper model management module (Candle / Hugging Face)
//! 
//! Handles fetching Whisper models using direct HTTP downloads.

use anyhow::Result;
use std::path::PathBuf;
use std::io::Write;

/// Model variants available
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WhisperModel {
    TinyEn,
    BaseEn,
    SmallEn,
    Tiny, // Multilingual
    Base,
    Small,
}

impl WhisperModel {
    /// Get the Hugging Face Repo ID
    pub fn repo_id(&self) -> &'static str {
        match self {
            WhisperModel::TinyEn => "openai/whisper-tiny.en",
            WhisperModel::BaseEn => "openai/whisper-base.en",
            WhisperModel::SmallEn => "openai/whisper-small.en",
            WhisperModel::Tiny => "openai/whisper-tiny",
            WhisperModel::Base => "openai/whisper-base",
            WhisperModel::Small => "openai/whisper-small",
        }
    }
    
    /// Get the specific revision (commit hash) to pin if needed, or "main"
    pub fn revision(&self) -> &'static str {
        "main"
    }

    /// Human-readable display name with size info
    pub fn display_name(&self) -> &'static str {
        match self {
            WhisperModel::TinyEn => "Tiny.en (39MB, Fast)",
            WhisperModel::BaseEn => "Base.en (74MB, Balanced)",
            WhisperModel::SmallEn => "Small.en (244MB, Accurate)",
            WhisperModel::Tiny => "Tiny (39MB, Multilingual)",
            WhisperModel::Base => "Base (74MB, Multilingual)",
            WhisperModel::Small => "Small (244MB, Multilingual)",
        }
    }

    /// Convert from settings string
    pub fn from_settings_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "tiny_en" | "tiny.en" => WhisperModel::TinyEn,
            "base_en" | "base.en" => WhisperModel::BaseEn,
            "small_en" | "small.en" => WhisperModel::SmallEn,
            "tiny" => WhisperModel::Tiny,
            "base" => WhisperModel::Base,
            "small" => WhisperModel::Small,
            _ => WhisperModel::BaseEn, // Default fallback
        }
    }

    /// Convert to settings string
    pub fn to_settings_str(&self) -> &'static str {
        match self {
            WhisperModel::TinyEn => "tiny_en",
            WhisperModel::BaseEn => "base_en",
            WhisperModel::SmallEn => "small_en",
            WhisperModel::Tiny => "tiny",
            WhisperModel::Base => "base",
            WhisperModel::Small => "small",
        }
    }

    /// Get all available models
    pub fn all() -> &'static [WhisperModel] {
        &[
            WhisperModel::TinyEn,
            WhisperModel::BaseEn,
            WhisperModel::SmallEn,
            WhisperModel::Tiny,
            WhisperModel::Base,
            WhisperModel::Small,
        ]
    }
}

impl Default for WhisperModel {
    fn default() -> Self {
        WhisperModel::BaseEn
    }
}

/// Paths to the essential files for a Whisper model
#[derive(Debug, Clone)]
pub struct ModelPaths {
    pub model: PathBuf,
    pub tokenizer: PathBuf,
    pub config: PathBuf,
}

/// Model manager for fetching models from HF Hub via direct HTTP
pub struct ModelManager {
    cache_dir: PathBuf,
    client: reqwest::blocking::Client,
}

impl ModelManager {
    pub fn new() -> Result<Self> {
        let project_dirs = directories::ProjectDirs::from("com", "auto-typer", "v6")
            .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?;
        let cache_dir = project_dirs.cache_dir().to_path_buf();
        std::fs::create_dir_all(&cache_dir)?;
        
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;
        
        Ok(Self { cache_dir, client })
    }

    /// Download a file from HuggingFace Hub
    fn download_hf_file(&self, repo_id: &str, filename: &str) -> Result<PathBuf> {
        // Create repo-specific cache directory
        let repo_cache = self.cache_dir.join(repo_id.replace('/', "_"));
        std::fs::create_dir_all(&repo_cache)?;
        
        let file_path = repo_cache.join(filename);
        
        // Return cached file if exists
        if file_path.exists() {
            log::info!("Using cached: {:?}", file_path);
            return Ok(file_path);
        }
        
        // Build HuggingFace Hub URL
        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            repo_id, filename
        );
        
        log::info!("Downloading: {}", url);
        
        let response = self.client.get(&url).send()?;
        
        if !response.status().is_success() {
            anyhow::bail!("HTTP {}: {}", response.status(), url);
        }
        
        // Get content length for progress
        let total_size = response.content_length().unwrap_or(0);
        log::info!("File size: {} bytes", total_size);
        
        // Download to file
        let bytes = response.bytes()?;
        let mut file = std::fs::File::create(&file_path)?;
        file.write_all(&bytes)?;
        
        log::info!("Downloaded: {:?}", file_path);
        Ok(file_path)
    }

    /// Fetch the model files. This blocks while downloading.
    pub fn fetch_model(&self, model: WhisperModel) -> Result<ModelPaths> {
        let repo_id = model.repo_id();
        log::info!("=== Fetching model: {} ===", repo_id);
        
        let config = self.download_hf_file(repo_id, "config.json")?;
        let tokenizer = self.download_hf_file(repo_id, "tokenizer.json")?;
        let model_path = self.download_hf_file(repo_id, "model.safetensors")?;
        
        log::info!("=== Model fetch complete ===");

        Ok(ModelPaths {
            model: model_path,
            tokenizer,
            config,
        })
    }

    /// Fetch the Mel filter bytes from the Candle repository
    pub fn fetch_mel_filters(&self, mel_bins: usize) -> Result<PathBuf> {
        let filename = match mel_bins {
            80 => "melfilters.bytes",
            128 => "melfilters128.bytes",
            _ => anyhow::bail!("Unsupported mel bins: {}", mel_bins),
        };
        
        let path = self.cache_dir.join(filename);

        if path.exists() {
             log::info!("Mel filters found cached at {:?}", path);
             return Ok(path);
        }

        let url = format!(
            "https://raw.githubusercontent.com/huggingface/candle/main/candle-examples/examples/whisper/{}",
            filename
        );
        
        log::info!("Downloading mel filters from {}", url);
        let response = self.client.get(&url).send()?;
        let bytes = response.bytes()?;
        std::fs::write(&path, bytes)?;
        
        Ok(path)
    }
}

