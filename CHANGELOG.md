# Changelog

All notable changes to this project will be documented in this file.

## [v6.0.0] - 2026-01-01

### Added

- **AI Voice Transcription:** Integrated OpenAI Whisper (via Candle) for real-time speech-to-typing.
- **Microphone Selection:** GUI-based audio device selection and refreshing.
- **Direct HTTP Model Downloader:** Custom metadata/model fetching to bypass `hf-hub` URL issues.
- **Professional Project Structure:** Root project reorganization with `docs/`, `Python_SDK/`, and improved `.gitignore`.
- **Comprehensive Documentation:** Added `Issues.md`, `Progress.md`, and `TODO.md`.

### Changed

- **Backend Language:** Migrated core transcription logic to Pure Rust (removed `whisper-rs` C bindings).
- **Settings UI:** Overhauled settings with larger fonts, better spacing, and consistent icons.
- **Typer Logic:** Refined typing simulation for faster, more reliable input.

## [v5.0.0] - Legacy Rust

### Added

- Initial Rust prototype.
- Global hotkeys support.

## [v4.0.0] - Python Peak

### Added

- **Smart Pause:** Natural word-boundary detection.
- **Super-Human Mode:** Typos and auto-correction simulation.
- **Funny Speedometer:** Dynamic UI labels based on CPM.
