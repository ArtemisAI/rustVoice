# rustVoice 🎙️⌨️

A professional AI-powered suite for natural keyboard simulation and real-time voice transcription. **rustVoice** bridges the gap between human speech and digital input, specifically optimized for remote-desktop accessibility (RustDesk, VNC, RDP) and high-performance workflow automation.

---

## 🌟 Key Features

### 🎙 AI Transcription Engine

The heart of **rustVoice**, built for low latency and high accuracy.

- **Real-time Voice-to-Input:** Powered by OpenAI's Whisper (via Hugging Face Candle) in pure Rust.
- **Microphone Management:** Intelligent device selection and hot-swapping in the GUI.
- **GPU Acceleration:** CUDA support for near-instant transcription.
- **Direct-to-Input:** Types transcribed text directly into any active window or IDE.
- **Privacy First:** local processing—no audio leaves your machine.

### 🐍 Legacy Python SDK

For text-based typing simulation and legacy automation.

- **Smart Pause:** Natural word-boundary detection.
- **Super-Human Mode:** Realistic typo injection and auto-correction.

---

## 📂 Project Structure

```text
rustVoice/
├── v5_rust_typer/        # Core AI Voice Transcription Engine
├── Python_SDK/           # Legacy Python Text Typer Scripts
├── docs/                 # Quick start, issues, and roadmap
├── test/                 # Sample audio files
└── README.md             # This file
```

---

## 🚀 Quick Start

For detailed instructions, see the **[Quick Start Guide](docs/quick_start.md)**.

### 1. Launching rustVoice (Rust)

```powershell
cd v5_rust_typer
cargo run --release
```

### 2. Using the Python SDK

```powershell
pip install keyboard pyperclip tk
python Python_SDK/Exam_Auto_Typer_v4.py
```

---

## 🛡 Security & Best Practices

- **No Private Data:** All local logs, model caches, and environment variables are excluded via `.gitignore`.
- **Clean Code:** Modular architecture separating audio capture, transcription, and typing simulation.
- **No External Dependencies:** The Rust version uses a pure implementation, reducing the attack surface.

---

## 📄 Documentation

For more detailed information, please refer to the `docs/` folder:

- [Usage Guide](docs/AUTO_TYPER_USAGE.md)
- [Known Issues & Roadmap](docs/Issues.md)
- [Project Progress](docs/Progress.md)
- [TODO List](docs/TODO.md)

---

## ⚖ License

*Project created for specialized input automation and accessibility research.*
