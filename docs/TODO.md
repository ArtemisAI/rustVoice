# rustVoice TODO

## 🖥 UI / UX

- [ ] Fix icon scaling for high-DPI displays.
- [ ] Add tooltips for Settings options.
- [ ] Implement transcription "Task" toggle (Transcribe vs Translate) in the actual decoder logic.
- [ ] Add more granular timestamps toggle.

## ⚙ Core Engine

- [ ] **Remote Desktop Fix:** Investigate the "random characters" issue in RustDesk. Try clipboard-based pasting as an alternative to emulated keystrokes.
- [ ] **GPU Auto-Detection:** Display more prominent warnings if GPU/CUDA is available but not being used.
- [ ] **Model Persistence:** Optimize model caching to avoid re-checking metadata on every launch.

## 📦 Deployment & Organization

- [ ] Rename `v5_rust_typer` to `v6_rust_typer` (clean path state).
- [ ] Create a standalone `.bat` or installer for easy user deployment.
- [ ] Clear temporary isolated target directories once build stability is confirmed.
