# Babata

Babata is an exploration of a fully AI-driven agent system that aims to provide only a minimal foundational mechanism, while delegating as much logic and execution as possible to AI.

## Dashboard

Babata ships with a local dashboard that behaves like a lightweight control plane for tasks. The dashboard is served by the built-in Axum HTTP server and opens in your browser with a single command:

```bash
cargo run -- dashboard
```

The command prints the local URL and, by default, opens your browser to:

```text
http://127.0.0.1:18800
```

If you only want the URL without opening a browser window:

```bash
cargo run -- dashboard --no-open
```

### Frontend Build Requirement

The dashboard frontend is an embedded `Vite + React + TypeScript` app. Rust builds it through `build.rs`, which runs:

```bash
npm --prefix web run build
```

That means local Rust builds require `node` and `npm` to be installed and available on `PATH`.

### Local Verification

Use these commands to verify the full dashboard stack locally:

```bash
npm --prefix web test -- --run
npm --prefix web run build
cargo test --test dashboard_cli
cargo test --test dashboard_api
cargo test
cargo run -- dashboard --no-open
```
