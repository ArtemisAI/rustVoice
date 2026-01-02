# Quick Start Guide 🚀

Welcome to **rustVoice**! This guide will help you get up and running with the AI-powered voice transcription and typing suite.

## 🛠 Prerequisites

### For AI Voice Transcription (Rust)

- **Rust Toolchain:** [Install Rust](https://rustup.rs/) (v1.75+ recommended).
- **C++ Build Tools:** Install "Desktop development with C++" via the [Visual Studio Installer](https://visualstudio.microsoft.com/visual-cpp-build-tools/).
- **CUDA (Optional):** For NVIDIA GPU acceleration, install the [CUDA Toolkit](https://developer.nvidia.com/cuda-toolkit).

### For Legacy Scripts (Python)

- **Python 3.8+**
- **Dependencies:** `pip install keyboard pyperclip tk`

---

## 🚀 Setting Up the Rust Engine

### 1. Build & Run

Navigate to the backend directory and run the application:

```powershell
cd apps/rustvoice
cargo run --release
```

### 2. First Launch: Model Download

On the first run, click the **"Load Model"** button in the Settings. **rustVoice** will automatically download the required Whisper AI models (config, tokenizer, and weights) directly from Hugging Face into your local cache.

### 3. Usage

1. Open the application.
2. Select your **Microphone** from the Settings menu.
3. Click **"Dictate"** to start the AI transcription.
4. Switch to your target window (e.g., VS Code). **rustVoice** will begin typing what you say.

---

## 🐍 Using the Python SDK

The Python scripts are located in the `Python_SDK/` directory. They are useful for text-to-typing automation.

```powershell
# Install required libraries
pip install keyboard pyperclip tk

# Run the production version (v4)
python Python_SDK/Exam_Auto_Typer_v4.py
```

---

## ⚙️ Configuration & Hardware

### GPU Acceleration

If you have a compatible NVIDIA GPU, you can build **rustVoice** with CUDA support for faster processing:

```powershell
cargo build --release --features cuda
```

### Remote Desktop (RustDesk / VNC)

If typing into a remote machine, ensure the **Typing Speed** in Settings is set to a "Natural" or slightly slower pace to allow for network latency.

---

## 🆘 Troubleshooting

- **No audio captured:** Check your microphone permissions in Windows Settings and ensure the correct device is selected in the **rustVoice** Settings menu.
- **Model load failed:** Ensure you have an active internet connection for the initial model download.
- **Random characters:** If typing into a remote IDE results in repetitive letters, try reducing the CPM (Characters Per Minute) in the Settings.
