# Exam Auto-Typer User Guide

This tool allows you to paste formatted answers (Markdown, code, etc.) and have them automatically typed into the exam environment (VS Code, Browser, Locked Browser, etc.).

## 🚀 Quick Start

1. **Run the Script**:

    ```powershell
    python "Exam_Auto_Typer_GUI.py"
    ```

2. **Prepare Content**:
    - Copy your answer from the AI chat.
    - Click **📋 Load Clipboard** or paste directly into the text area.
    - Review/Edit the text if needed.

3. **Start Typing**:
    - Adjust **Speed (CPM)** if needed (Default: 1200 CPM is safe and fast).
    - Click **▶ START (5s)**.
    - **IMMEDIATELY switch** to the target window (Exam software/Browser) and click in the answer box.
    - Wait for the typing to complete.

## ⏸ Controls

- **Pause**: Click `⏸ PAUSE` to temporarily stop typing (e.g., if a pop-up appears). The resume button will blink.
- **Resume**: Click `▶ RESUME` to continue from where it left off.
- **Stop**: Click `⏹ STOP` to completely reset the process.

## 🛑 SAFETY / PANIC BUTTON

- Press **`ESC` key** on your keyboard at ANY time to instantly abort typing.
- Move your mouse pointer to any of the four corners of the screen to also trigger a fail-safe abort.
