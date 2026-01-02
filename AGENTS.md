# Agent Collaboration Protocol (Artemis 🤝 Copilot)

This document defines the Agent-to-Agent (A2A) workflow for solving technical issues in the `rustVoice` project. This protocol ensures seamless synergy between **Artemis (Lead Agent)** and **Copilot (Implementation Agent)**.

## 🔄 Collaboration Lifecycle

### 1. Issue Identification & Creation (Artemis)

- **Action**: Artemis identifies a bug, regression, or feature gap.
- **Requirement**: Create a detailed GitHub Issue via `gh issue create`.
- **Metadata**: Include logs, specific file paths, line numbers, and expected technical behavior.
- **Assignment**: Formally notify `@copilot` in the issue description or body to trigger development.

### 2. Work Verification & Initialization (Artemis)

- **Action**: Use `gh pr list` and `gh pr view` to verify Copilot has acknowledged the task and created a PR.
- **Wait Period**: Allow a **60-minute window** for Copilot to perform initial research and push a plan or draft.

### 3. Local Checkout & Testing (Artemis)

- **Action**: Pull Copilot's PR branch locally for verification.

  ```powershell
  gh pr checkout <number>
  ```

- **Execution**: Run the build and perform regression testing.

  ```powershell
  cargo run --release
  ```

- **Debugging**: Perform local trace analysis if the problem persists.

### 4. Review & Feedback (Artemis)

- **Scenario A (Success)**: Merge the PR and document the fix in `docs/CHANGELOG.md` and `walkthrough.md`.
- **Scenario B (Iteration Required)**: Provide a detailed "Change Request" in the PR comments. Include new logs or failing trace data.
- **Assignment**: Notify `@copilot` to iterate on the specific failure point.

### 5. Final Closure

- Once the local verification passes, the branch is merged, and the issue is closed.

---
*Note: This protocol prioritizes the Rust core (`apps/rustvoice`). Python SDK development is legacy and handled only on demand.*
