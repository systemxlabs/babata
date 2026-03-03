# Repository Guidelines

## Project Structure & Module Organization
- `src/` contains all Rust code.
- `src/main.rs` is the CLI entrypoint; `src/lib.rs` exports modules.
- Core domains are split into folders: `src/cli`, `src/provider`, `src/channel`, `src/job`, `src/config`, `src/tool`, `src/memory`.
- Cross-cutting modules include `src/logging.rs`, `src/error.rs`, `src/task.rs`, and `src/system_prompt.rs`.
- `services/` stores service templates (Linux/macOS).
- `.github/workflows/ci.yaml` defines CI checks.
- `system_prompts/` contains default prompt docs used by onboarding/export logic.

## Build, Test, and Development Commands
- `cargo check --workspace`  
  Fast compile check used in CI.
- `cargo test --all-features`  
  Runs all unit tests.
- `cargo fmt --all --check`  
  Enforces formatting.
- `cargo clippy --workspace --all-features -- -D warnings`  
  Lint gate; warnings are treated as errors in CI.
- `cargo run -- <args>`  
  Run CLI locally (example: `cargo run -- onboard`).

## Coding Style & Naming Conventions
- Follow Rust defaults: 4-space indentation, `rustfmt` output, no manual style drift.
- Naming:
  - `snake_case` for modules/functions/variables
  - `PascalCase` for structs/enums/traits
  - `SCREAMING_SNAKE_CASE` for constants
- Keep modules focused by domain (do not put unrelated logic into `main.rs`).
- Prefer explicit error context via `BabataError` and `BabataResult`.

## Testing Guidelines
- Put tests close to implementation under `#[cfg(test)]` in the same module file.
- Use descriptive test names such as `at_schedule_returns_some_when_time_is_future`.
- Add/adjust tests for behavior changes, especially around scheduler, provider, channel, and service logic.
- Before opening a PR, run full checks locally:
  `cargo fmt --all --check && cargo clippy --workspace --all-features -- -D warnings && cargo test --all-features`.

## Commit & Pull Request Guidelines
- Follow the repository’s commit style: `<area>: <imperative summary>`.
  - Examples: `server: wait for Windows service stop/start during restart`, `config: remove timezone field from cron schedule`.
- Keep commits scoped and atomic; avoid mixing unrelated refactors.
- PRs should include:
  - what changed and why
  - risk/impact notes (runtime, config, service behavior)
  - verification steps and command outputs used.
