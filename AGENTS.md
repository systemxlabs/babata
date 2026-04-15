# Agent Collaboration Guidelines

This document outlines the engineering preferences and cultural principles for all agents (human or AI) contributing to the `babata` project.

## 1. Prefer Strong Typing

- Leverage the type system to make invalid states unrepresentable.
- Avoid raw strings and magic numbers when a typed enum, newtype, or domain model can express intent more precisely.
- In Rust, embrace `enum`, `struct`, and the type-state pattern. In TypeScript, favor strict `type` / `interface` definitions and avoid implicit `any`.
- Treat compiler errors as design feedback; do not suppress them with casts or unsafe coercions without explicit justification.

## 2. Readability & Maintainability First

- Write code for the reader six months from now.
- Keep functions small and focused on a single responsibility.
- Use descriptive names for variables, functions, and types. Optimize for clarity over brevity.
- Favor explicit, boring code over clever one-liners.
- Document the "why" in comments when the code itself cannot express intent.

## 3. Refactor Proactively

- Leave the codebase cleaner than you found it (Boy Scout Rule).
- When a change reveals duplication, unclear abstractions, or tight coupling, refactor as part of the same logical unit of work.
- Refactoring should be incremental and backed by existing or new tests; avoid massive rewrites in a single PR.
- Challenge technical debt early. Small, continuous improvements compound into a healthy architecture.

## 4. General Principles

- **Tests are guard rails**: Add or update tests alongside code changes.
- **Consistency matters**: Follow existing patterns unless they violate the principles above.
- **Security by default**: Validate inputs, sanitize outputs, and never log secrets.
- **Code in English**: All variable names, function names, type names, comments, and doc strings must be written in **English**. Even when the project serves Chinese-speaking users, the codebase itself should remain in English to maintain international readability and a uniform developer experience.

## 5. Testing & UI Validation

- Start the local server with `BABATA_SERVER_PORT=<port> cargo run -- server serve`.
- Open `http://127.0.0.1:<port>/` in a browser to inspect the Web UI.
- Prefer browser-automation tools (e.g., `browser-use`, Playwright, Puppeteer) to verify rendering and interactions.
- When adding or changing frontend-related features, confirm the UI behaves correctly through automation or manual inspection.

## 6. Development Workflow

- **Backend code**: Before every commit, run `cargo fmt`, `cargo clippy`, and `cargo test` to ensure formatting, static analysis, and tests all pass.
- **Frontend code**: After modifying frontend code, perform the equivalent validation (type checking, build checks, unit tests, etc.) according to the frontend project's configuration.
- All CI checks must pass before requesting code review or merging.

## 7. Architecture Overview

Understanding the directory layout helps contributors locate code and respect module boundaries.

### `web/` — Frontend
- `web/src/pages/` — Page-level React components (Dashboard, Tasks, Agents, Channels, Providers, Skills).
- `web/src/components/` — Reusable and domain-specific UI components, including `TaskDetailModal`, `FileExplorer`, `AgentDetailModal`, `SkillDetailModal`, and the base `ui` kit.
- `web/src/hooks/` — Custom React hooks (e.g., mobile detection).
- `web/src/lib/` — Utility functions and helpers.
- `web/src/api.ts` — HTTP client wrappers for the backend APIs.
- `web/src/types.ts` — Shared TypeScript type definitions.

### `src/` — Backend (Rust)
- `src/agent/` — Agent definitions, runner, and lifecycle management.
- `src/channel/` — Multi-channel ingestion adapters (e.g., WeChat, Telegram) that turn incoming messages into tasks.
- `src/cli/` — Command-line interface: argument parsing, subcommands, and onboarding flows.
- `src/config/` — Configuration loading for providers and channels.
- `src/http/` — HTTP server, REST API handlers, and SPA fallback for the Web UI.
- `src/memory/` — Agent memory storage abstractions and persistence.
- `src/provider/` — LLM provider implementations (OpenAI, Anthropic, DeepSeek, Kimi, Moonshot, MiniMax, custom, etc.).
- `src/system_prompt/` — System prompt templates injected into agent contexts.
- `src/task/` — Asynchronous task scheduling, execution, state management, and persistence.
- `src/tool/` — Built-in tool registry and implementations (file editing, shell, task control, browser automation, etc.).
- Root-level files (`main.rs`, `lib.rs`, `message.rs`, `skill.rs`, `logging.rs`, `error.rs`, `utils.rs`) — Application entry, library exports, core data models, logging, error handling, and shared utilities.

Before modifying code, review the relevant directory's responsibilities to keep changes localized and consistent.

## 8. Testing Philosophy

- Only add tests when they verify internal behavior, edge cases, or protect against regressions.
- Do not write tests for the sake of coverage; avoid testing trivial getters, setters, or pure pass-through logic.
- Unless explicitly requested, do not create meaningless test files or boilerplate test suites.
