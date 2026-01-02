use eframe::egui;
use enigo::{Enigo, Key, Keyboard, Direction, Settings};
use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use arboard::Clipboard;
use parking_lot::Mutex;
use rand::Rng;
use crossbeam_channel::{unbounded, Sender, Receiver};
use rdev::{listen, EventType, Key as RdevKey};
use rfd::FileDialog;

// Voice transcription modules (Candle)
mod audio;
mod model;
mod decoder;
mod transcribe;
mod settings;

use audio::{AudioCapture, list_input_devices, get_default_input_device_name};
use model::{ModelManager, WhisperModel};
use transcribe::{WhisperTranscriber, TranscriptionResult};
use settings::AppSettings;

// --- Global Constants ---
const NEIGHBORS: &[(&str, &str)] = &[
    ("a", "qwsz"), ("b", "vghn"), ("c", "xdfv"), ("d", "serfcx"), ("e", "wsdr"), ("f", "drtgv"),
    ("g", "ftyhb"), ("h", "gyunj"), ("i", "ujko"), ("j", "hunik"), ("k", "jiolm"), ("l", "kop"),
    ("m", "njk"), ("n", "bhjm"), ("o", "iklp"), ("p", "ol"), ("q", "wa"), ("r", "edft"),
    ("s", "awedxz"), ("t", "rfgy"), ("u", "yhji"), ("v", "cfgb"), ("w", "qase"), ("x", "zsdc"),
    ("y", "tghu"), ("z", "asx"), (" ", " ")
];

// --- App State ---
struct AutoTyperApp {
    text_to_type: String,
    status_msg: String,
    progress: f32,
    
    // Config
    // Config
    settings: AppSettings,
    speed_cpm: Arc<AtomicUsize>,
    mode: usize, // 0=Natural, 1=SuperHuman, 2=Turbo, 3=Block
    
    // Control
    running: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    pause_pending: Arc<AtomicBool>,
    stop_requested: Arc<AtomicBool>,
    
    // Channels
    status_rx: Receiver<(String, f32, bool)>, // msg, progress, is_paused
    
    // Voice Transcription (v6)
    audio_capture: Option<AudioCapture>,
    transcriber: Option<Arc<WhisperTranscriber>>,
    transcription_rx: Option<Receiver<TranscriptionResult>>,
    model_load_rx: Option<Receiver<anyhow::Result<Arc<WhisperTranscriber>>>>,
    is_dictating: bool,
    pending_transcription: String,
    model_status: String,
    model_progress: f32,
    
    // File Playback
    file_playback_stop: Arc<AtomicBool>,
    
    // UI State
    show_settings: bool,
    selected_model: WhisperModel,
    
    // Audio Device Selection
    available_mics: Vec<String>,
    selected_mic: Option<String>,
}

#[derive(Clone)]
enum AppMode {
    Natural,
    SuperHuman,
    Turbo,
    Block,
}

impl AutoTyperApp {
    fn new(cc: &eframe::CreationContext<'_>, status_rx: Receiver<(String, f32, bool)>, 
           running: Arc<AtomicBool>, paused: Arc<AtomicBool>, pause_pending: Arc<AtomicBool>, 
           stop_requested: Arc<AtomicBool>, speed_cpm: Arc<AtomicUsize>) -> Self {
        
        setup_custom_fonts(&cc.egui_ctx);
        configure_styles(&cc.egui_ctx);

        let settings = AppSettings::load();
        
        // Apply loaded settings
        speed_cpm.store(settings.typing_speed_cpm, Ordering::Relaxed);
        
        // Extract model selection before moving settings
        let selected_model = WhisperModel::from_settings_str(&settings.model_size);
        
        Self {
            text_to_type: String::new(),
            status_msg: "Ready. Double-Tap ESC to Stop.".to_owned(),
            progress: 0.0,
            settings,
            speed_cpm,
            mode: 1, // Default SuperHuman
            running,
            paused,
            pause_pending,
            stop_requested,
            status_rx,
            // Voice transcription (v6)
            audio_capture: None,
            transcriber: None,
            transcription_rx: None,
            model_load_rx: None,
            is_dictating: false,
            pending_transcription: String::new(),
            model_status: "Model not loaded".to_string(),
            model_progress: 0.0,
            file_playback_stop: Arc::new(AtomicBool::new(false)),
            show_settings: false,
            selected_model,
            // Audio device selection
            available_mics: list_input_devices(),
            selected_mic: get_default_input_device_name(),
        }
    }

    /// Select and play an audio file for transcription
    fn upload_audio_file(&mut self) {
        if self.transcriber.is_none() {
             self.status_msg = "Model not loaded. Please load model first.".to_string();
             return;
        }

        println!("DEBUG: upload_audio_file called");
        if let Some(path) = FileDialog::new()
            .add_filter("Audio", &["mp3", "wav", "m4a", "ogg", "flac"])
            .set_directory("/") // Default to root to ensure it doesn't get stuck? Or just remove set_directory if it exists (it doesn't)
            .pick_file() 
        {
            println!("DEBUG: File selected: {:?}", path);
            let stop_signal = Arc::new(AtomicBool::new(false));
            self.file_playback_stop = stop_signal.clone();
            
            // Channel for audio chunks
            let (audio_tx, audio_rx) = unbounded();
            
            // Spawn file reader thread
            let path_clone = path.clone();
            let stop_clone = stop_signal.clone();
            
            thread::spawn(move || {
                match audio::decode_audio_file(&path_clone) {
                    Ok(samples) => {
                         let chunk_size = 16000 / 2; // 500ms at 16kHz
                         for chunk in samples.chunks(chunk_size) {
                             if stop_clone.load(Ordering::Relaxed) { break; }
                             if audio_tx.send(chunk.to_vec()).is_err() { break; }
                             // Real-time simulation: Sleep 500ms
                             // We can go slightly faster (e.g. 0.8x sleep) to feel snappier but let's stick to 1.0x
                             thread::sleep(Duration::from_millis(480)); 
                         }
                    }
                    Err(e) => {
                        log::error!("File decode error: {}", e);
                    }
                }
            });

            // Start Transcriber with this RX
            if let Some(transcriber) = &self.transcriber {
                let t = transcriber.clone();
                let (tx, rx) = unbounded();
                self.transcription_rx = Some(rx);
                t.start(audio_rx, tx);
                
                self.is_dictating = true;
                self.status_msg = format!("Playing: {:?}", path.file_name().unwrap_or_default());
            }
        }
    }
    
    /// Start voice dictation
    fn start_dictation(&mut self) {
        // Initialize audio capture
        match AudioCapture::new() {
            Ok(mut capture) => {
                let mic_name = self.selected_mic.as_deref();
                if let Err(e) = capture.start_with_device(mic_name) {
                    self.status_msg = format!("Audio error: {}", e);
                    return;
                }
                
                let audio_rx = capture.audio_receiver();
                
                // Check if transcriber is loaded
                if let Some(transcriber) = &self.transcriber {
                    // Clone Arc to send to thread
                    let t = transcriber.clone();
                    // Create channel for results
                    let (tx, rx) = unbounded();
                    self.transcription_rx = Some(rx);
                    
                    t.start(audio_rx, tx);
                    
                    self.is_dictating = true;
                    self.status_msg = "🎙 Listening...".to_string();
                } else {
                    self.status_msg = "Model not loaded. Click 'Load Model' first.".to_string();
                    return;
                }
                
                self.audio_capture = Some(capture);
            }
            Err(e) => {
                self.status_msg = format!("Failed to start audio: {}", e);
            }
        }
    }
    
    /// Stop voice dictation or file playback
    fn stop_dictation(&mut self) {
        // Stop Mic
        if let Some(mut capture) = self.audio_capture.take() {
            capture.stop();
        }
        // Stop File
        self.file_playback_stop.store(true, Ordering::Relaxed);
        
        // Transcriber thread stops when channel disconnects (audio_rx dropped)
        self.is_dictating = false;
        self.pending_transcription.clear();
        self.status_msg = "Dictation/Playback stopped.".to_string();
    }
    
    /// Load the Whisper model
    fn load_model(&mut self) {
        if self.transcriber.is_some() { return; }
        
        let selected = self.selected_model;
        self.model_status = format!("Downloading {}...", selected.display_name());
        self.model_progress = 0.0;
        
        let (tx, rx) = unbounded();
        self.model_load_rx = Some(rx);

        thread::spawn(move || {
            let manager = match ModelManager::new() {
                Ok(m) => m,
                Err(e) => {
                    let _ = tx.send(Err(anyhow::anyhow!("Manager init failed: {}", e)));
                    return;
                }
            };
            
            // Fetch Model (using selected model)
            let model_paths = match manager.fetch_model(selected) {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(Err(anyhow::anyhow!("Download failed: {}", e)));
                    return;
                }
            };
            
            // Fetch Mel Filters (80 bins for standard models, 128 for large-v3 if added)
            let mel_paths = match manager.fetch_mel_filters(80) {
                 Ok(p) => p,
                 Err(e) => {
                     let _ = tx.send(Err(anyhow::anyhow!("Mel filter download failed: {}", e)));
                     return;
                 }
            };

            // Load Transcriber
            match WhisperTranscriber::new(model_paths, mel_paths) {
                Ok(t) => {
                    let _ = tx.send(Ok(Arc::new(t)));
                }
                Err(e) => {
                    let _ = tx.send(Err(anyhow::anyhow!("Load failed: {}", e)));
                }
            }
        });
    }
}

impl eframe::App for AutoTyperApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Process messages from thread
        while let Ok((msg, prog, _is_paused)) = self.status_rx.try_recv() {
            self.status_msg = msg;
            self.progress = prog;
        }
        
        // Process model loading updates
        if let Some(rx) = &self.model_load_rx {
             if let Ok(result) = rx.try_recv() {
                 match result {
                     Ok(transcriber) => {
                          let device = transcriber.get_device_name();
                          self.transcriber = Some(transcriber);
                          self.model_status = format!("Model Ready (Candle 🕯️) on {}", device);
                         self.model_progress = 1.0;
                         self.status_msg = "Model loaded successfully.".to_string();
                         self.model_load_rx = None; // Done
                     }
                     Err(e) => {
                         self.model_status = format!("Error: {}", e);
                         self.status_msg = format!("Model load failed: {}", e);
                         self.model_load_rx = None; // Done
                     }
                 }
             }
        }
        
        // Process transcription results
        if let Some(rx) = &self.transcription_rx {
            while let Ok(result) = rx.try_recv() {
                // Append confirmed text to text_to_type
                if !result.confirmed.is_empty() && !self.text_to_type.ends_with(&result.confirmed) {
                    // Find new confirmed text
                    let existing_len = self.text_to_type.len();
                    if result.confirmed.len() > existing_len {
                        let new_text = &result.confirmed[existing_len..];
                        self.text_to_type.push_str(new_text);
                    }
                }
                self.pending_transcription = result.pending;
            }
        }

        // Opacity check - commented out for compatibility
        // frame.set_window_opacity(self.opacity);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("rustVoice v6 (AI Edition) 🦀🎙");
            if self.is_dictating {
                ui.horizontal(|ui| {
                     ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "🔴 LISTENING");
                     ui.spinner();
                });
            }
            ui.label(egui::RichText::new(&self.model_status).small().weak());
            ui.add_space(10.0);

            // Header Controls
            ui.horizontal(|ui| {
                if ui.button("⚙ Settings").clicked() {
                    self.show_settings = !self.show_settings;
                }
                
                ui.separator();
                
                if ui.button("📋 Paste").clicked() {
                    if let Ok(mut clipboard) = Clipboard::new() {
                        if let Ok(text) = clipboard.get_text() {
                            self.text_to_type = text;
                        }
                    }
                }
                if ui.button("🗑 Clear").clicked() {
                    self.text_to_type.clear();
                }

                ui.separator();

                // Load Model
                if ui.add_enabled(self.transcriber.is_none() && self.model_load_rx.is_none(), egui::Button::new("📥 Load Model")).clicked() {
                    self.load_model();
                }

                ui.separator();
                
                // Dictate button
                let dictate_text = if self.is_dictating { "🛑 Stop" } else { "🎙 Dictate" };
                let dictate_enabled = self.transcriber.is_some() || !self.is_dictating;
                if ui.add_enabled(dictate_enabled, egui::Button::new(dictate_text)).clicked() {
                    if self.is_dictating {
                        self.stop_dictation();
                    } else {
                        self.start_dictation();
                    }
                }
                
                if ui.add_enabled(!self.is_dictating && self.transcriber.is_some(), egui::Button::new("📂 Upload Audio")).clicked() {
                    self.upload_audio_file();
                }
            });

            ui.add_space(10.0);

            // Settings Panel
            if self.show_settings {
                let mut is_open = self.show_settings;
                egui::Window::new("⚙ Settings")
                    .default_pos([200.0, 150.0])
                    .default_width(350.0)
                    .collapsible(true)
                    .open(&mut is_open)
                    .show(ctx, |ui| {
                        // (Closure content remains the same)
                        // ===== 🎨 Appearance Section =====
                        ui.heading("🎨 Appearance");
                        ui.add_space(5.0);
                        
                        if ui.checkbox(&mut self.settings.dark_mode, "Dark Mode").changed() {
                            self.settings.save();
                            configure_styles(ctx);
                        }
                        
                        ui.horizontal(|ui| {
                            ui.label("Opacity:");
                            if ui.add(egui::Slider::new(&mut self.settings.opacity, 0.3..=1.0).show_value(true)).changed() {
                                self.settings.save();
                            }
                        });
                        
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(5.0);
                        
                        // ===== 🤖 Model Section =====
                        ui.heading("🤖 Whisper Model");
                        ui.add_space(8.0);
                        
                        let current_model_name = self.selected_model.display_name();
                        egui::ComboBox::from_label("Model Size")
                            .selected_text(current_model_name)
                            .show_ui(ui, |ui| {
                                for model in WhisperModel::all() {
                                    let is_selected = *model == self.selected_model;
                                    if ui.selectable_label(is_selected, model.display_name()).clicked() {
                                        self.selected_model = *model;
                                        self.settings.model_size = model.to_settings_str().to_string();
                                        self.settings.save();
                                        // Clear loaded model if selection changed
                                        if self.transcriber.is_some() {
                                            self.transcriber = None;
                                            self.model_status = "Model changed. Click 'Load Model' to apply.".to_string();
                                        }
                                    }
                                }
                            });
                        
                        ui.label(egui::RichText::new("Change requires reloading the model.").small().weak());
                        
                        ui.add_space(5.0);
                        if ui.button("📥 Load Model").clicked() {
                            self.load_model();
                        }
                        
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(8.0);
                        
                        // ===== 🎤 Audio Section =====
                        ui.heading("🎤 Audio Input");
                        ui.add_space(8.0);
                        
                        let current_mic = self.selected_mic.as_deref().unwrap_or("Default (Auto)");
                        egui::ComboBox::from_label("Microphone")
                            .selected_text(current_mic)
                            .show_ui(ui, |ui| {
                                // Default option
                                if ui.selectable_label(self.selected_mic.is_none(), "Default (Auto)").clicked() {
                                    self.selected_mic = None;
                                }
                                
                                for mic in &self.available_mics {
                                    let is_selected = Some(mic) == self.selected_mic.as_ref();
                                    if ui.selectable_label(is_selected, mic).clicked() {
                                        self.selected_mic = Some(mic.clone());
                                    }
                                }
                            });
                        
                        if ui.button("🔄 Refresh Devices").clicked() {
                            self.available_mics = list_input_devices();
                            if self.selected_mic.is_none() {
                                self.selected_mic = get_default_input_device_name();
                            }
                        }
                        
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // ===== 🎙 Transcription Section =====
                        ui.heading("🎙 Transcription");
                        ui.add_space(5.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("Task:");
                            if ui.radio_value(&mut self.settings.task, "transcribe".to_string(), "Transcribe").changed() {
                                self.settings.save();
                            }
                            if ui.radio_value(&mut self.settings.task, "translate".to_string(), "Translate to English").changed() {
                                self.settings.save();
                            }
                        });
                        
                        if ui.checkbox(&mut self.settings.timestamps, "Show Timestamps").changed() {
                            self.settings.save();
                        }
                        
                        if ui.checkbox(&mut self.settings.verbose, "Verbose Logging (Debug)").changed() {
                            self.settings.save();
                        }
                        
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(5.0);
                        
                        // ===== ⌨ Typing Section =====
                        ui.heading("⌨ Typing");
                        ui.add_space(5.0);
                        
                        ui.horizontal(|ui| {
                            ui.label("Default CPM:");
                            if ui.add(egui::Slider::new(&mut self.settings.typing_speed_cpm, 300..=5000)).changed() {
                                self.settings.save();
                            }
                        });
                    });
                self.show_settings = is_open;
            }

            // Text Area
            ui.add(egui::TextEdit::multiline(&mut self.text_to_type)
                .hint_text("Paste text here...")
                .desired_width(f32::INFINITY)
                .desired_rows(10));
            
            ui.add_space(10.0);

            // Mode Selection
            ui.horizontal(|ui| {
                ui.label("Mode:");
                egui::ComboBox::from_id_salt("mode_cb")
                    .selected_text(match self.mode {
                        0 => "Natural",
                        1 => "Super-Human (Typo+Correct)",
                        2 => "Turbo (Instant)",
                        3 => "Block (Line-by-Line)",
                        _ => "Unknown",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.mode, 0, "Natural");
                        ui.selectable_value(&mut self.mode, 1, "Super-Human (Typo+Correct)");
                        ui.selectable_value(&mut self.mode, 2, "Turbo (Instant)");
                        ui.selectable_value(&mut self.mode, 3, "Block (Line-by-Line)");
                    });
            });

            // Speed Control
            ui.add_space(5.0);
            let cpm = self.speed_cpm.load(Ordering::Relaxed);
            let mut cpm_val = cpm;
            ui.horizontal(|ui| {
                ui.label("Speed:");
                if ui.add(egui::Slider::new(&mut cpm_val, 300..=5000).text("CPM")).changed() {
                    self.speed_cpm.store(cpm_val, Ordering::Relaxed);
                    // Update settings default too? Maybe not, keep transient
                }
            });
            ui.label(egui::RichText::new(get_funny_label(cpm)).italics().weak());
            ui.label(egui::RichText::new("Hotkeys: Alt+Shift+ (+/-) to change speed.").small().weak());

            ui.add_space(15.0);

            // Action Buttons
            ui.horizontal(|ui| {
                let is_running = self.running.load(Ordering::Relaxed);
                
                if ui.add_enabled(!is_running, egui::Button::new("▶ START (5s)").min_size(egui::vec2(100.0, 30.0))).clicked() {
                     // Start Logic
                     start_typing_thread(
                         self.text_to_type.clone(),
                         self.mode,
                         self.speed_cpm.clone(),
                         self.running.clone(),
                         self.paused.clone(),
                         self.pause_pending.clone(),
                         self.stop_requested.clone(),
                         self.status_rx.clone(), // This is wrong, need Sender. Creating channel in main.
                     );
                }

                let is_paused = self.paused.load(Ordering::Relaxed);
                let pause_text = if is_paused { "▶ RESUME (ESC)" } else { "⏸ PAUSE (ESC)" };
                
                if ui.add_enabled(is_running, egui::Button::new(pause_text).min_size(egui::vec2(100.0, 30.0))).clicked() {
                    if is_paused {
                         self.paused.store(false, Ordering::Relaxed);
                         self.pause_pending.store(false, Ordering::Relaxed);
                    } else {
                        // Smart Pause Check? For button usually immediate or smart, sticking to smart.
                        self.pause_pending.store(true, Ordering::Relaxed);
                    }
                }

                if ui.add_enabled(is_running, egui::Button::new("⏹ STOP (2xESC)").min_size(egui::vec2(100.0, 30.0))).clicked() {
                    self.stop_requested.store(true, Ordering::Relaxed);
                }
            });

            ui.add_space(10.0);
            ui.label(&self.status_msg);
            ui.add(egui::ProgressBar::new(self.progress));
        });
        
        // Repaint for updates
        ctx.request_repaint();
    }
}

fn get_funny_label(cpm: usize) -> String {
    if cpm < 500 { "🐢 Grandma".to_string() }
    else if cpm < 1200 { "👨‍💼 Average Joe".to_string() }
    else if cpm < 2000 { "⚡ Pro Gamer".to_string() }
    else if cpm < 3000 { "🐒 ADHD Monkey".to_string() }
    else if cpm < 4500 { "🤖 Matrix Mode".to_string() }
    else { "🚀 TO THE MOON".to_string() }
}

fn main() -> Result<(), eframe::Error> {
    println!("DEBUG: Starting main...");
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 600.0])
            .with_transparent(true)
            .with_always_on_top()
            .with_decorations(true),
        ..Default::default()
    };

    // Shared State
    let running = Arc::new(AtomicBool::new(false));
    let paused = Arc::new(AtomicBool::new(false));
    let pause_pending = Arc::new(AtomicBool::new(false));
    let stop_requested = Arc::new(AtomicBool::new(false));
    let speed_cpm = Arc::new(AtomicUsize::new(1200));

    let (tx, rx) = unbounded();
    *GLOBAL_SENDER.lock() = Some(tx.clone());

    // Global Input Listener (ESC & Hotkeys)
    let r_run = running.clone();
    let r_stop = stop_requested.clone();
    let r_pause = paused.clone();
    let r_pend = pause_pending.clone();
    let r_speed = speed_cpm.clone();
    let r_tx = tx.clone();
    
    thread::spawn(move || {
        println!("DEBUG: Typo thread spawned");
        let mut last_esc = Instant::now();
        // Variables for modifier state tracking
        let mut alt_down = false;
        let mut shift_down = false;

        if let Err(error) = listen(move |event| {
            match event.event_type {
                EventType::KeyPress(key) => {
                    match key {
                        RdevKey::Escape => {
                            if r_run.load(Ordering::Relaxed) {
                                if last_esc.elapsed() < Duration::from_millis(500) {
                                    r_stop.store(true, Ordering::Relaxed);
                                    let _ = r_tx.send(("STOPPED (Double ESC)".into(), 0.0, false));
                                } else {
                                    // Toggle Smart Pause
                                    if r_pause.load(Ordering::Relaxed) {
                                        r_pause.store(false, Ordering::Relaxed);
                                        r_pend.store(false, Ordering::Relaxed);
                                        let _ = r_tx.send(("RESUMED".into(), 0.0, false)); 
                                    } else {
                                        r_pend.store(true, Ordering::Relaxed);
                                        let _ = r_tx.send(("Pausing at next space...".into(), 0.0, false)); 
                                    }
                                }
                                last_esc = Instant::now();
                            }
                        }
                        RdevKey::Alt | RdevKey::AltGr => alt_down = true,
                        RdevKey::ShiftLeft | RdevKey::ShiftRight => shift_down = true,
                        // Speed Up: + or =
                        RdevKey::Equal | RdevKey::KpPlus => {
                            if alt_down && shift_down {
                                let old = r_speed.load(Ordering::Relaxed);
                                r_speed.store(old + 100, Ordering::Relaxed);
                                let _ = r_tx.send((format!("Speed UP: {}", old+100), 0.0, false));
                            }
                        },
                        // Speed Down: - or _
                        RdevKey::Minus | RdevKey::KpMinus => {
                             if alt_down && shift_down {
                                let old = r_speed.load(Ordering::Relaxed);
                                if old > 100 {
                                    r_speed.store(old - 100, Ordering::Relaxed);
                                    let _ = r_tx.send((format!("Speed DOWN: {}", old-100), 0.0, false));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                EventType::KeyRelease(key) => {
                    match key {
                        RdevKey::Alt | RdevKey::AltGr => alt_down = false,
                        RdevKey::ShiftLeft | RdevKey::ShiftRight => shift_down = false,
                        _ => {}
                    }
                }
                _ => {}
            }
        }) {
            println!("Error: {:?}", error);
        }
    });

    eframe::run_native(
        "rustVoice",
        options,
        Box::new(move |cc| {
            println!("DEBUG: Creating App Context");
            Ok(Box::new(AutoTyperApp::new(cc, rx, running, paused, pause_pending, stop_requested, speed_cpm)))
        }),
    )
}

// --- Typing Logic ---
lazy_static::lazy_static! {
    static ref GLOBAL_SENDER: Mutex<Option<Sender<(String, f32, bool)>>> = Mutex::new(None);
}

fn start_typing_thread(
    text: String,
    mode: usize,
    speed_cpm: Arc<AtomicUsize>,
    running: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    pause_pending: Arc<AtomicBool>,
    stop_requested: Arc<AtomicBool>,
    _rx: Receiver<(String, f32, bool)>, 
) {
    // Actually we don't need _rx here.
    // We need to access GLOBAL_SENDER to send updates back.
    
    running.store(true, Ordering::Relaxed);
    paused.store(false, Ordering::Relaxed);
    pause_pending.store(false, Ordering::Relaxed);
    stop_requested.store(false, Ordering::Relaxed);

    thread::spawn(move || {
        let mut enigo = Enigo::new(&enigo::Settings::default()).unwrap();
        let total_chars = text.len();
        
        // Countdown
        for i in (1..=5).rev() {
            if stop_requested.load(Ordering::Relaxed) { break; }
             send_status(format!("Starting in {}s...", i), 0.0, false);
            thread::sleep(Duration::from_secs(1));
        }

        if !stop_requested.load(Ordering::Relaxed) {
             send_status("Typing...".into(), 0.0, false);
             
             let mut i = 0;
             let chars: Vec<char> = text.chars().collect();
             
             while i < chars.len() {
                 if stop_requested.load(Ordering::Relaxed) { break; }
                 
                 // Handle Pausing
                 check_smart_pause(&paused, &pause_pending, chars[i]);
                 while paused.load(Ordering::Relaxed) {
                      if stop_requested.load(Ordering::Relaxed) { break; }
                      send_status("PAUSED".into(), (i as f32 / total_chars as f32), true);
                      thread::sleep(Duration::from_millis(100));
                 }
                 
                 let ch = chars[i];
                 let cpm = speed_cpm.load(Ordering::Relaxed) as u64;
                 if cpm == 0 { thread::sleep(Duration::from_millis(100)); continue; }
                 let base_delay_ms = 60000 / cpm; // Milliseconds per char
                 
                 match mode {
                     1 => { // Super-Human
                         // Paragraph Pause
                         if ch == '\n' {
                             let _ = enigo.key(Key::Return, Direction::Click);
                             let think = rand::thread_rng().gen_range(1000..3000);
                             send_status("Thinking...".into(), (i as f32 / total_chars as f32), false);
                             thread::sleep(Duration::from_millis(think));
                         } else {
                             // Typo Logic
                             // let mut typed_correct = false; // Unused
                             if rand::thread_rng().gen_bool(0.03) { // 3% typo
                                if let Some(neighbor) = get_neighbor(ch) {
                                    let _ = enigo.text(&neighbor.to_string());
                                     thread::sleep(Duration::from_millis((base_delay_ms as f32 * 1.5) as u64)); // reaction
                                    let _ = enigo.key(Key::Backspace, Direction::Click);
                                    thread::sleep(Duration::from_millis(100));
                                }
                             }
                             let _ = enigo.text(&ch.to_string());
                         }
                     },
                     2 => { // Turbo
                         // Actually this loop is inefficient for turbo, but implementing char by char for consistent structure
                         // For real turbo we'd dump it all. Let's do char for now or refactor.
                         // Simplification: Rust enigo sequence is fast.
                         // .. implementing simple char type for now to save complexity
                         let _ = enigo.text(&ch.to_string());
                     }
                     _ => { // Natural
                          let _ = enigo.text(&ch.to_string());
                     }
                 }
                 
                 // Jitter
                 let jitter = rand::thread_rng().gen_range(0.9..1.1);
                 let delay = (base_delay_ms as f32 * jitter) as u64;
                 thread::sleep(Duration::from_millis(delay));

                 i += 1;
                 
                 if i % 10 == 0 {
                    send_status(format!("Typing... {}%", (i * 100 / total_chars)), (i as f32 / total_chars as f32), false);
                 }
             }
        }

        running.store(false, Ordering::Relaxed);
        send_status("Done!".into(), 1.0, false);
    });
}

fn check_smart_pause(paused: &Arc<AtomicBool>, pending: &Arc<AtomicBool>, ch: char) {
    if pending.load(Ordering::Relaxed) {
        if ch == ' ' || ch == '\n' || ch == '\t' {
            paused.store(true, Ordering::Relaxed);
            pending.store(false, Ordering::Relaxed);
        }
    }
}

fn send_status(msg: String, prog: f32, paused: bool) {
    let guard = GLOBAL_SENDER.lock();
    if let Some(tx) = &*guard {
        let _ = tx.send((msg, prog, paused));
    }
}

fn get_neighbor(c: char) -> Option<char> {
    let lower = c.to_lowercase().next()?;
    for (k, v) in NEIGHBORS {
        if k.starts_with(lower) {
            let idx = rand::thread_rng().gen_range(0..v.len());
            let n_char = v.chars().nth(idx)?;
             return if c.is_uppercase() { Some(n_char.to_ascii_uppercase()) } else { Some(n_char) };
        }
    }
    None
}

// Helpers for UI
fn setup_custom_fonts(ctx: &egui::Context) {
    let fonts = egui::FontDefinitions::default();
    // Use default, sufficient for now
    ctx.set_fonts(fonts);
}
fn configure_styles(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    
    // Aesthetic Dark Theme
    style.visuals = egui::Visuals::dark();
    style.visuals.window_fill = egui::Color32::from_rgb(30, 30, 46); // Catppuccin Base -ish
    style.visuals.panel_fill = egui::Color32::from_rgb(30, 30, 46);
    
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 30, 46);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(49, 50, 68); // Surface0
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(69, 71, 90); // Surface1
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(88, 91, 112); // Surface2
    
    style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(205, 214, 244); // Text
    
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(137, 180, 250); // Blue
    style.visuals.selection.stroke.color = egui::Color32::WHITE;

    // Corner rounding
    style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
    style.visuals.window_rounding = egui::Rounding::same(8.0);
    
    ctx.set_style(style);
}

// End of file
