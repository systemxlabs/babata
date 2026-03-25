# Babata Dashboard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a local OpenClaw-style dashboard for Babata that launches via `babata dashboard`, serves a lightweight embedded web UI, and exposes task-focused observation/control pages.

**Architecture:** Keep Babata as a single local application. Extend the existing Rust/Axum server with `/api` dashboard endpoints and static asset serving, add a top-level `dashboard` CLI command for service orchestration plus browser open behavior, and build the UI as a small `Vite + React + TypeScript` app compiled into Rust-served assets.

**Tech Stack:** Rust, Axum, clap, rusqlite, tokio, rust-embed, mime_guess, Vite, React, TypeScript, react-router-dom, Vitest, Testing Library

---

## Planned File Map

### Rust CLI and bootstrap

- Modify: `src/main.rs`
  Dispatch the new top-level `dashboard` command.
- Modify: `src/cli/mod.rs`
  Export the new dashboard module.
- Modify: `src/cli/args.rs`
  Add `Command::Dashboard { no_open: bool }`.
- Create: `src/cli/dashboard.rs`
  Implement dashboard startup flow, URL printing, `--no-open`, and browser-launch helpers.
- Test: `tests/dashboard_cli.rs`
  Verify command parsing and helper behavior without opening a real browser.

### Rust HTTP and assets

- Modify: `src/http/mod.rs`
  Split API routes under `/api`, add dashboard shell routes, and register new endpoint modules.
- Create: `src/http/assets.rs`
  Serve embedded dashboard assets and SPA fallback.
- Create: `src/http/get_overview.rs`
  Return aggregated overview payloads.
- Create: `src/http/get_system.rs`
  Return runtime and build metadata.
- Create: `src/http/get_task_content.rs`
  Return `task.md` and `progress.md`.
- Create: `src/http/get_task_tree.rs`
  Return parent/current/children tree data.
- Create: `src/http/list_task_artifacts.rs`
  Return artifact file metadata and previewable content.
- Create: `src/http/get_task_logs.rs`
  Return log content or explicit unsupported state.
- Modify: `src/http/list_tasks.rs`
  Add richer task filtering and a dashboard-friendly response shape.
- Modify: `src/http/get_task.rs`
  Expand task summary payload for the dashboard detail shell.
- Modify: `src/http/create_task.rs`
  Keep existing semantics but mount under `/api/tasks`.
- Modify: `src/http/control_task.rs`
  Return action-friendly responses for dashboard mutation UX.
- Test: `tests/dashboard_api.rs`
  Exercise dashboard shell, `/api/overview`, `/api/system`, and task detail endpoints.

### Rust task model and persistence

- Modify: `src/task/manager.rs`
  Preserve task directories after completion/cancel and add helpers for task files.
- Modify: `src/task/store.rs`
  Add richer list/filter/query support and tree lookups.
- Create: `src/task/query.rs`
  Hold reusable dashboard query/filter structs to avoid further bloating `store.rs`.
- Modify: `src/task/mod.rs`
  Export query types and any new helper functions.
- Test: `src/task/manager.rs`
  Add retention tests for completed/canceled root tasks.
- Test: `src/task/store.rs`
  Add tests for root-only, text query, and tree-oriented queries.

### Frontend workspace

- Create: `web/package.json`
  Frontend scripts and dependencies.
- Create: `web/tsconfig.json`
  TypeScript configuration.
- Create: `web/vite.config.ts`
  Build to `web/dist` for Rust serving.
- Create: `web/index.html`
  Vite entry document.
- Create: `web/src/main.tsx`
  React bootstrap.
- Create: `web/src/App.tsx`
  Router and app shell composition.
- Create: `web/src/App.test.tsx`
  Frontend smoke test for the shared shell.
- Create: `web/src/styles/app.css`
  Forge Panels theme tokens and base layout rules.
- Create: `web/src/api/types.ts`
  Shared TypeScript API contracts.
- Create: `web/src/api/client.ts`
  Fetch wrapper for `/api`.
- Create: `web/src/hooks/usePolling.ts`
  Shared polling hook for overview and detail pages.
- Create: `web/src/components/AppShell.tsx`
  Top-level navigation shell.
- Create: `web/src/components/Panel.tsx`
  Shared panel primitive.
- Create: `web/src/components/StatusBadge.tsx`
  Shared task status display.
- Create: `web/src/components/Toolbar.tsx`
  Shared filter/action toolbar layout.
- Create: `web/src/pages/OverviewPage.tsx`
- Create: `web/src/pages/TasksPage.tsx`
- Create: `web/src/pages/TaskDetailPage.tsx`
- Create: `web/src/pages/CreatePage.tsx`
- Create: `web/src/pages/SystemPage.tsx`
- Create: `web/src/test/setup.ts`
  Vitest setup.
- Create: `web/src/pages/OverviewPage.test.tsx`
- Create: `web/src/pages/TasksPage.test.tsx`
- Create: `web/src/pages/TaskDetailPage.test.tsx`
- Create: `web/src/pages/CreatePage.test.tsx`
- Create: `web/src/pages/SystemPage.test.tsx`

### Build and docs

- Modify: `Cargo.toml`
  Add any missing test/runtime deps needed by dashboard serving and HTTP tests.
- Create: `build.rs`
  Build the frontend before Rust compiles embedded assets.
- Modify: `README.md`
  Document `babata dashboard`, frontend prerequisites, and verification commands.

## Task 1: Add Dashboard CLI Surface

**Files:**
- Create: `src/cli/dashboard.rs`
- Modify: `src/main.rs`
- Modify: `src/cli/mod.rs`
- Modify: `src/cli/args.rs`
- Test: `tests/dashboard_cli.rs`

- [ ] **Step 1: Write the failing CLI parsing and helper tests**

```rust
use clap::Parser;
use babata::cli::Command;

#[test]
fn dashboard_command_parses_no_open() {
    let command = Command::parse_from(["babata", "dashboard", "--no-open"]);
    assert!(matches!(command, Command::Dashboard { no_open: true }));
}
```

Also add a pure helper test for the browser launch path in `tests/dashboard_cli.rs`, for example verifying that `dashboard_url(18800)` returns `http://127.0.0.1:18800/`.

- [ ] **Step 2: Run the targeted test command to verify it fails**

Run: `cargo test --test dashboard_cli dashboard_command_parses_no_open -- --exact`

Expected: FAIL because `Command::Dashboard` and `tests/dashboard_cli.rs` do not exist yet.

- [ ] **Step 3: Implement the new top-level command and dashboard launcher**

Add a new clap variant in `src/cli/args.rs`:

```rust
#[command(about = "Start or open the local dashboard")]
Dashboard {
    #[arg(long, help = "Print the URL without opening the browser")]
    no_open: bool,
}
```

Implement `src/cli/dashboard.rs` with:

```rust
pub fn run(no_open: bool) {
    if let Err(err) = run_dashboard(no_open) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
```

Inside `run_dashboard`, reuse `server::start_background_service()` when needed, print the exact URL, and keep browser-opening logic behind a testable helper function.

- [ ] **Step 4: Re-run the dashboard CLI tests**

Run: `cargo test --test dashboard_cli`

Expected: PASS

- [ ] **Step 5: Commit the CLI surface**

```bash
git add src/main.rs src/cli/mod.rs src/cli/args.rs src/cli/dashboard.rs tests/dashboard_cli.rs
git commit -m "feat: add dashboard cli command"
```

## Task 2: Scaffold Frontend Workspace and Rust Asset Embedding

**Files:**
- Create: `build.rs`
- Modify: `Cargo.toml`
- Modify: `src/http/mod.rs`
- Create: `src/http/assets.rs`
- Create: `web/package.json`
- Create: `web/tsconfig.json`
- Create: `web/vite.config.ts`
- Create: `web/index.html`
- Create: `web/src/main.tsx`
- Create: `web/src/App.tsx`
- Create: `web/src/styles/app.css`
- Create: `web/src/test/setup.ts`
- Test: `tests/dashboard_api.rs`

- [ ] **Step 1: Write failing tests for the dashboard shell route**

Add a test in `tests/dashboard_api.rs` that requests `/` from the Axum router and expects HTML:

```rust
#[tokio::test]
async fn dashboard_root_serves_html_shell() {
    let app = test_router();
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/html; charset=utf-8");
}
```

Also add a frontend smoke test that renders `App` and checks for the top navigation label.

- [ ] **Step 2: Run the shell tests and confirm they fail**

Run: `cargo test --test dashboard_api dashboard_root_serves_html_shell -- --exact`

Expected: FAIL because the router only exposes API-ish routes today.

Run: `npm --prefix web test -- --run`

Expected: FAIL because the frontend workspace does not exist yet.

- [ ] **Step 3: Add the frontend toolchain, minimal app shell, and asset build pipeline**

Create `web/package.json` with scripts like:

```json
{
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "test": "vitest"
  }
}
```

Create `build.rs` to run:

```rust
let status = Command::new("npm")
    .args(["--prefix", "web", "run", "build"])
    .status()?;
if !status.success() {
    panic!("dashboard frontend build failed");
}
println!("cargo:rerun-if-changed=web/src");
println!("cargo:rerun-if-changed=web/index.html");
```

In `src/http/assets.rs`, use `rust_embed::RustEmbed` against `web/dist` and serve:

- `/`
- `/tasks`
- `/tasks/:task_id`
- `/create`
- `/system`

all with the built `index.html`, while static asset requests serve files from the embedded bundle.

- [ ] **Step 4: Re-run frontend and shell tests**

Run: `npm --prefix web test -- --run`

Expected: PASS

Run: `cargo test --test dashboard_api dashboard_root_serves_html_shell -- --exact`

Expected: PASS

- [ ] **Step 5: Commit the workspace scaffold**

```bash
git add build.rs Cargo.toml src/http/mod.rs src/http/assets.rs web tests/dashboard_api.rs
git commit -m "feat: scaffold dashboard frontend and asset serving"
```

## Task 3: Preserve Task Directories and Add Dashboard Query Primitives

**Files:**
- Create: `src/task/query.rs`
- Modify: `src/task/mod.rs`
- Modify: `src/task/store.rs`
- Modify: `src/task/manager.rs`
- Test: `src/task/store.rs`
- Test: `src/task/manager.rs`

- [ ] **Step 1: Write failing task retention and query tests**

Add a manager test that proves completed root tasks keep their directory:

```rust
#[tokio::test]
async fn completed_root_task_directory_is_retained() {
    // create task, complete it, then assert task_dir(task_id).exists()
}
```

Add store tests for richer queries, for example:

```rust
#[test]
fn list_tasks_filters_root_only() {
    let filters = TaskListQuery {
        root_only: true,
        ..Default::default()
    };
    let tasks = store.list_tasks_filtered(&filters).unwrap();
    assert!(tasks.iter().all(|task| task.parent_task_id.is_none()));
}
```

- [ ] **Step 2: Run the targeted task tests and verify they fail**

Run: `cargo test completed_root_task_directory_is_retained --lib -- --exact`

Expected: FAIL because `TaskManager` still removes root task directories on completion.

Run: `cargo test list_tasks_filters_root_only --lib -- --exact`

Expected: FAIL because richer dashboard filters do not exist.

- [ ] **Step 3: Implement retention-safe task lifecycle and query structs**

Create `src/task/query.rs` with a reusable filter type:

```rust
#[derive(Debug, Clone, Default)]
pub struct TaskListQuery {
    pub status: Option<TaskStatus>,
    pub agent: Option<String>,
    pub root_only: bool,
    pub never_ends: Option<bool>,
    pub query: Option<String>,
    pub limit: usize,
    pub offset: usize,
}
```

Update `TaskStore` with methods such as:

- `list_tasks_filtered(&TaskListQuery) -> BabataResult<Vec<TaskRecord>>`
- `list_root_tree(root_task_id: Uuid) -> BabataResult<Vec<TaskRecord>>`

Remove the root-directory deletion path from `TaskManager::handle_task_completed` and `TaskManager::cancel_task`.

- [ ] **Step 4: Re-run the task model tests**

Run: `cargo test completed_root_task_directory_is_retained --lib -- --exact`

Expected: PASS

Run: `cargo test list_tasks_filters_root_only --lib -- --exact`

Expected: PASS

- [ ] **Step 5: Commit the persistence changes**

```bash
git add src/task/query.rs src/task/mod.rs src/task/store.rs src/task/manager.rs
git commit -m "feat: preserve task directories for dashboard inspection"
```

## Task 4: Add Overview, System, and Dashboard-Friendly Task APIs

**Files:**
- Create: `src/http/get_overview.rs`
- Create: `src/http/get_system.rs`
- Modify: `src/http/mod.rs`
- Modify: `src/http/list_tasks.rs`
- Modify: `src/http/get_task.rs`
- Modify: `src/http/create_task.rs`
- Modify: `src/http/control_task.rs`
- Test: `tests/dashboard_api.rs`

- [ ] **Step 1: Write failing API tests for overview and system data**

Add tests such as:

```rust
#[tokio::test]
async fn overview_returns_status_counts_and_recent_tasks() {
    let app = seeded_router();
    let response = app
        .oneshot(Request::builder().uri("/api/overview").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_text(response).await.contains("\"counts\""));
}
```

Add a system endpoint test that checks for `version` and `http_addr`.

- [ ] **Step 2: Run the overview/system tests and confirm they fail**

Run: `cargo test --test dashboard_api overview_returns_status_counts_and_recent_tasks -- --exact`

Expected: FAIL because `/api/overview` does not exist.

- [ ] **Step 3: Implement `/api` routing and overview/system payloads**

Add API routes in `src/http/mod.rs`:

```rust
.route("/api/overview", get(get_overview::handle))
.route("/api/system", get(get_system::handle))
.route("/api/tasks", get(list_tasks::handle).post(create_task::handle))
.route("/api/tasks/{task_id}", get(get_task::handle))
```

Update list/get responses so the dashboard receives fields it needs in one request:

- action availability
- root/parent relationships
- `never_ends`
- stable timestamps

Return simple action results from pause/resume/cancel/relaunch:

```json
{ "ok": true, "task_id": "...", "action": "pause" }
```

- [ ] **Step 4: Re-run the overview and system API tests**

Run: `cargo test --test dashboard_api overview_returns_status_counts_and_recent_tasks -- --exact`

Expected: PASS

Run: `cargo test --test dashboard_api system_endpoint_returns_runtime_metadata -- --exact`

Expected: PASS

- [ ] **Step 5: Commit the overview/system API slice**

```bash
git add src/http/mod.rs src/http/get_overview.rs src/http/get_system.rs src/http/list_tasks.rs src/http/get_task.rs src/http/create_task.rs src/http/control_task.rs tests/dashboard_api.rs
git commit -m "feat: add dashboard overview and system APIs"
```

## Task 5: Add Task Detail Content, Tree, Artifact, and Log APIs

**Files:**
- Create: `src/http/get_task_content.rs`
- Create: `src/http/get_task_tree.rs`
- Create: `src/http/list_task_artifacts.rs`
- Create: `src/http/get_task_logs.rs`
- Modify: `src/http/mod.rs`
- Modify: `src/task/manager.rs`
- Modify: `src/task/store.rs`
- Test: `tests/dashboard_api.rs`

- [ ] **Step 1: Write failing API tests for detail-only data**

Add tests for:

- `/api/tasks/:task_id/content` returns `task_markdown` and `progress_markdown`
- `/api/tasks/:task_id/tree` returns parent/current/children structure
- `/api/tasks/:task_id/artifacts` returns a file list
- `/api/tasks/:task_id/logs` returns either logs or an explicit unsupported marker

Example:

```rust
#[tokio::test]
async fn task_content_returns_task_and_progress_markdown() {
    let response = seeded_router()
        .oneshot(Request::builder().uri(&format!("/api/tasks/{task_id}/content")).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_text(response).await.contains("\"progress_markdown\""));
}
```

- [ ] **Step 2: Run the task detail API tests and verify they fail**

Run: `cargo test --test dashboard_api task_content_returns_task_and_progress_markdown -- --exact`

Expected: FAIL because the route and file-reading helpers do not exist.

- [ ] **Step 3: Implement detail endpoints against real task directories**

Add file-reading helpers that resolve:

- `task.md`
- `progress.md`
- `artifacts/`
- agent-specific log candidates such as `stdout`, `stderr`, or `codex-last-message.md`

Return explicit unsupported states for logs:

```json
{ "supported": false, "reason": "No known log files for this agent" }
```

Keep the tree payload small and stable:

```json
{
  "root_task_id": "...",
  "parent": null,
  "current": { "...": "..." },
  "children": []
}
```

- [ ] **Step 4: Re-run the task detail API tests**

Run: `cargo test --test dashboard_api task_content_returns_task_and_progress_markdown -- --exact`

Expected: PASS

Run: `cargo test --test dashboard_api task_logs_returns_unsupported_state_when_no_files_exist -- --exact`

Expected: PASS

- [ ] **Step 5: Commit the task detail API slice**

```bash
git add src/http/mod.rs src/http/get_task_content.rs src/http/get_task_tree.rs src/http/list_task_artifacts.rs src/http/get_task_logs.rs src/task/manager.rs src/task/store.rs tests/dashboard_api.rs
git commit -m "feat: add dashboard task detail APIs"
```

## Task 6: Build the Frontend App Shell, Theme, and Shared Data Layer

**Files:**
- Create: `web/src/api/types.ts`
- Create: `web/src/api/client.ts`
- Create: `web/src/hooks/usePolling.ts`
- Create: `web/src/components/AppShell.tsx`
- Create: `web/src/components/Panel.tsx`
- Create: `web/src/components/StatusBadge.tsx`
- Create: `web/src/components/Toolbar.tsx`
- Modify: `web/src/App.tsx`
- Modify: `web/src/App.test.tsx`
- Modify: `web/src/styles/app.css`
- Test: `web/src/App.test.tsx`

- [ ] **Step 1: Write a failing frontend test for the shared shell**

Add a test that renders the app and checks for the top-level navigation labels:

```tsx
it("renders dashboard navigation", () => {
  render(<App />);
  expect(screen.getByText("Overview")).toBeInTheDocument();
  expect(screen.getByText("Tasks")).toBeInTheDocument();
  expect(screen.getByText("Create")).toBeInTheDocument();
  expect(screen.getByText("System")).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the frontend shell test and verify it fails**

Run: `npm --prefix web test -- --run App`

Expected: FAIL because the shared shell, routing, and test setup are incomplete.

- [ ] **Step 3: Implement the shared shell and polling primitives**

Create a small API client:

```ts
export async function apiGet<T>(path: string): Promise<T> {
  const response = await fetch(`/api${path}`);
  if (!response.ok) throw new Error(await response.text());
  return response.json() as Promise<T>;
}
```

Create `usePolling` with:

- configurable interval
- manual refresh
- last-refreshed timestamp
- auto-refresh toggle

Apply the `Forge Panels` visual language in `web/src/styles/app.css` using CSS variables for:

- background layers
- panel surfaces
- accent colors
- status tokens

- [ ] **Step 4: Re-run the shared frontend tests**

Run: `npm --prefix web test -- --run`

Expected: PASS for the shell-level tests

- [ ] **Step 5: Commit the shell and shared UI layer**

```bash
git add web/src/App.tsx web/src/App.test.tsx web/src/styles/app.css web/src/api web/src/hooks web/src/components web/src/test
git commit -m "feat: add dashboard shell and shared frontend primitives"
```

## Task 7: Implement Overview, Create, and System Pages

**Files:**
- Create: `web/src/pages/OverviewPage.tsx`
- Create: `web/src/pages/CreatePage.tsx`
- Create: `web/src/pages/SystemPage.tsx`
- Create: `web/src/pages/OverviewPage.test.tsx`
- Create: `web/src/pages/CreatePage.test.tsx`
- Modify: `web/src/App.tsx`
- Test: `web/src/pages/SystemPage.test.tsx`

- [ ] **Step 1: Write failing page tests**

Add tests for:

- overview renders status counters and recent tasks from `/api/overview`
- create form validates empty prompt before submit
- system page renders version and URL from `/api/system`

Example:

```tsx
it("blocks create submit when prompt is empty", async () => {
  render(<CreatePage />);
  await user.click(screen.getByRole("button", { name: /create task/i }));
  expect(screen.getByText(/prompt is required/i)).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the page tests and confirm they fail**

Run: `npm --prefix web test -- --run OverviewPage CreatePage SystemPage`

Expected: FAIL because the pages are not implemented.

- [ ] **Step 3: Implement the first page set**

On `OverviewPage`:

- fetch `/api/overview`
- auto-refresh every 5 seconds by default
- surface active tasks and recent tasks

On `CreatePage`:

- submit to `POST /api/tasks`
- redirect to `/tasks/:task_id` on success

On `SystemPage`:

- fetch `/api/system`
- show health, version, data dir, and dashboard URL

- [ ] **Step 4: Re-run the page tests**

Run: `npm --prefix web test -- --run OverviewPage CreatePage SystemPage`

Expected: PASS

- [ ] **Step 5: Commit the first page set**

```bash
git add web/src/App.tsx web/src/pages/OverviewPage.tsx web/src/pages/CreatePage.tsx web/src/pages/SystemPage.tsx web/src/pages/OverviewPage.test.tsx web/src/pages/CreatePage.test.tsx web/src/pages/SystemPage.test.tsx
git commit -m "feat: add dashboard overview create and system pages"
```

## Task 8: Implement Task Explorer and Task Detail Pages

**Files:**
- Create: `web/src/pages/TasksPage.tsx`
- Create: `web/src/pages/TaskDetailPage.tsx`
- Create: `web/src/pages/TasksPage.test.tsx`
- Create: `web/src/pages/TaskDetailPage.test.tsx`
- Modify: `web/src/api/types.ts`
- Modify: `web/src/api/client.ts`
- Modify: `web/src/components/StatusBadge.tsx`
- Modify: `web/src/components/Toolbar.tsx`

- [ ] **Step 1: Write failing tests for task browsing and detail controls**

Add tests that prove:

- tasks page sends status/query filters to `/api/tasks`
- task detail renders `progress.md` before `task.md`
- cancel action requires confirmation
- relaunch action requires a reason

Example:

```tsx
it("renders semantic progress as the primary detail panel", async () => {
  render(<TaskDetailPage />);
  expect(await screen.findByText(/progress/i)).toBeInTheDocument();
  expect(screen.getByText(/task definition/i)).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the task page tests and confirm they fail**

Run: `npm --prefix web test -- --run TasksPage TaskDetailPage`

Expected: FAIL because the task pages and control dialogs do not exist.

- [ ] **Step 3: Implement the task explorer**

Build `TasksPage` with:

- filter toolbar for `status`, `agent`, `root_only`, `never_ends`, `query`
- task rows that link to `/tasks/:task_id`
- quick `pause`, `resume`, and `cancel` actions

Use URL search params so refresh/share preserves filter state.

- [ ] **Step 4: Implement the task detail page**

Build `TaskDetailPage` with these sections:

- `Summary`
- `Semantic State`
- `Tree`
- `Runtime`
- `Outputs`

Fetch:

- `/api/tasks/:task_id`
- `/api/tasks/:task_id/content`
- `/api/tasks/:task_id/tree`
- `/api/tasks/:task_id/logs`
- `/api/tasks/:task_id/artifacts`

Refresh summary/content/tree every 5 seconds, but keep logs and artifacts on demand or manual refresh.

- [ ] **Step 5: Re-run the task page tests**

Run: `npm --prefix web test -- --run TasksPage TaskDetailPage`

Expected: PASS

- [ ] **Step 6: Commit the task explorer and detail pages**

```bash
git add web/src/pages/TasksPage.tsx web/src/pages/TaskDetailPage.tsx web/src/pages/TasksPage.test.tsx web/src/pages/TaskDetailPage.test.tsx web/src/api/types.ts web/src/api/client.ts web/src/components/StatusBadge.tsx web/src/components/Toolbar.tsx
git commit -m "feat: add dashboard task explorer and detail pages"
```

## Task 9: End-to-End Integration, Documentation, and Final Verification

**Files:**
- Modify: `README.md`
- Modify: `tests/dashboard_api.rs`
- Modify: `tests/dashboard_cli.rs`
- Modify: `web/src/pages/*.test.tsx` as needed

- [ ] **Step 1: Add the final verification tests before polish**

Add or extend tests for:

- `babata dashboard --no-open` prints the dashboard URL without trying to open a browser
- `GET /tasks/<id>` and other SPA URLs still serve `index.html`
- API endpoints surface explicit unsupported states instead of empty 500s

- [ ] **Step 2: Run the focused verification commands and record failures**

Run: `npm --prefix web run build`

Expected: PASS and create `web/dist`

Run: `npm --prefix web test -- --run`

Expected: PASS

Run: `cargo test --test dashboard_cli`

Expected: PASS

Run: `cargo test --test dashboard_api`

Expected: PASS

If any command fails, fix the issue before continuing.

- [ ] **Step 3: Update user-facing documentation**

Document in `README.md`:

- what `babata dashboard` does
- `--no-open`
- Node/npm requirement for building embedded assets
- local verification commands

- [ ] **Step 4: Run the full project verification**

Run: `cargo test`

Expected: PASS

Run: `cargo run -- dashboard --no-open`

Expected: process prints a local URL such as `http://127.0.0.1:18800/` and does not attempt to open a browser

- [ ] **Step 5: Commit the integrated dashboard release**

```bash
git add README.md tests/dashboard_api.rs tests/dashboard_cli.rs web/src build.rs
git commit -m "feat: ship babata dashboard"
```

## Plan Review

**Status:** Approved

**Issues:** None found in local review.

**Recommendations (advisory):**

- Keep `tests/dashboard_api.rs` using seeded in-memory or temp-dir task data so the HTTP layer stays fast to verify.
- If `build.rs` proves too heavy in practice, replace it with a checked, well-documented prebuild script before implementation drifts too far.
- Preserve old non-`/api` task routes as compatibility aliases only if that does not complicate the frontend integration path.
