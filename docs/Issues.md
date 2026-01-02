# Known Issues & Observations

## 1. Remote Machine Typing (RustDesk / Windows Voice Access Alternative)

**Context:** The primary motivation for this tool is to provide voice-to-text typing on remote machines (e.g., via RustDesk) where Windows Voice Access often fails to bridge the input to the external screen.

**Issue:**

- In initial tests typing into a RustDesk connected screen, the typer outputted random repetitive characters (e.g., `ddddddd:ddddd`) instead of the transcribed text.
- **Hypothesis:** This may be an encoding issue OR a timing issue with how the `enigo` crate simulates keypresses over a low-latency remote desktop protocol.
- **TODO:** Investigate "Safe Mode" typing or character-by-character delays for remote sessions.

## 2. Hugging Face Hub URL Error (Resolved with Workaround)

**Issue:**

- The `hf-hub` Rust crate failed with a `RelativeUrlWithoutBase` error when attempting to download models.
- **Cause:** Likely due to the lack of a default base URL or specialized environment configuration in certain Windows setups.
- **Status:** **Resolved.** Replaced `hf-hub` dependency with a custom `reqwest` based Model Manager that uses direct HTTP downloads from `huggingface.co`.

## 3. GPU/CUDA Support (Deployment Issue)

**Observation:**

- The application currently defaults to CPU in most builds.
- **Requirement:** To enable full GPU acceleration, the app must be compiled with the `cuda` feature: `cargo build --release --features cuda`.
- **Dependency:** Requires NVIDIA drivers and the CUDA Toolkit to be correctly installed on the host system.

## 4. UI Rendering (Ongoing)

**Observation:**

- Some icons or UI elements may appear small or missing depending on system DPI or asset loading.
- **Status:** Improved in latest version by increasing spacing and font sizes in Settings.

## 🚨 CRITICAL HANDOVER: Audio Buffer Mismatch (Assignee: @copilot)

**Issue**: The application panics or fails to process audio due to a buffer size mismatch.

- **Symptoms**: `Insufficient buffer size 480 for input channel 0, expected 1024` (Panic/Error from `rubato`?).
- **Cause**:
  - The default input device (microphone) provides audio in chunks of **480 samples** (typically 10ms at 48kHz).
  - The `rubato` FFT resampler (`FftFixedIn`) is initialized with a **fixed input block size of 1024 frames**.
  - `apps/rustvoice/src/audio.rs` passes the raw 480-sample buffer directly to the resampler, causing it to reject the input or crash.

**Current State (WIP in `apps/rustvoice/src/audio.rs`)**:

- I attempted to introduce an `input_buffer` (`Arc<Mutex<Vec<f32>>>`) to accumulate incoming chunks.
- **Logic**: Append incoming (480) samples to `input_buffer` -> Loop while `input_buffer.len() >= 1024` -> Feed 1024 chunks to resampler -> Output results.
- **Status**: The fix was applied but user reports "not working". It needs:
  1. Verification of the accumulation logic in `process_audio_data`.
  2. Verification that `FftFixedIn` is correctly initialized for the *accumulated* chunk size (1024).
  3. Debugging of the `cpal` stream builder to ensure it's not dropping frames or deadlocking on the mutex.

**Action Required (@copilot)**:

1. Debug `process_audio_data` in `apps/rustvoice/src/audio.rs`.
2. Ensure `input_buffer` successfully accumulates to 1024 before calling `resampler.process()`.
3. Confirm `audio_tx` receives the resampled output.
4. Test with `cargo run --release` and `RUST_LOG=debug`.
