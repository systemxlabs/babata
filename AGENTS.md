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

## 5. Testing & UI Validation

- Start the local server with `BABATA_SERVER_PORT=<port> cargo run -- server serve`.
- Open `http://127.0.0.1:<port>/` in a browser to inspect the Web UI.
- Prefer browser-automation tools (e.g., `browser-use`, Playwright, Puppeteer) to verify rendering and interactions.
- When adding or changing frontend-related features, confirm the UI behaves correctly through automation or manual inspection.

## 6. Development Workflow

- **Backend code**: Before every commit, run `cargo fmt`, `cargo clippy`, and `cargo test` to ensure formatting, static analysis, and tests all pass.
- **Frontend code**: After modifying frontend code, perform the equivalent validation (type checking, build checks, unit tests, etc.) according to the frontend project's configuration.
- All CI checks must pass before requesting code review or merging.
