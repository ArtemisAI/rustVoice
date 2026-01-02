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
