# Babata Dashboard Design

## 1. Background

Babata is not a chat-first product. Its core abstraction is a task-driven agent runtime:

- every prompt becomes a task
- tasks run with minimal control states: `running`, `done`, `canceled`, `paused`
- semantic progress lives in `task.md` and `progress.md`
- tasks can form parent-child trees
- the system is optimized for observation, control, interruption, and recovery

The existing HTTP layer already exposes the first slice of a local control plane:

- health check
- create task
- list task
- get task
- pause task
- resume task
- cancel task
- relaunch task

This makes a local dashboard a natural next step, but the current API surface is too thin for a high-quality task console.

## 2. Product Positioning

Babata Dashboard is a `Local Task Control Plane`.

It is not:

- a multi-user SaaS admin
- a chat transcript product
- a workflow builder
- a full system settings console in v1

It should feel like a local runtime console that lets a user:

- understand what the system is doing now
- inspect the semantic state of a task
- control tasks safely
- navigate task trees
- inspect logs and artifacts when available

## 3. Goals

The first dashboard release should:

1. Launch from a single CLI entrypoint with a browser-first local experience.
2. Show a high-signal overview of system and task activity.
3. Provide a usable task explorer with filtering and direct control actions.
4. Make task detail pages centered on `progress.md` and `task.md`, not just database metadata.
5. Support task tree navigation, artifact visibility, and log visibility where data exists.
6. Use a lightweight frontend embedded into the Rust service, preserving a single-app local workflow.

The first dashboard release should not:

- require realtime streaming infrastructure as a prerequisite
- pretend transcript or event-stream capabilities already exist
- expand into full configuration management
- solve every future observability use case up front

## 4. Confirmed Product Decisions

The following decisions were confirmed during design review:

- Scope target: `more complete version`
- Entry experience: `babata dashboard` opens the local dashboard
- Landing page: `Overview`
- Frontend architecture: lightweight frontend embedded into the Rust service
- Visual direction: `Forge Panels`
- Refresh model: polling first, not streaming first
- Backend scope: extend API and CLI as needed to make the dashboard a real control plane

## 5. User Experience

### 5.1 Launch Experience

Babata adds a dedicated CLI entrypoint:

```bash
babata dashboard
```

Expected behavior:

1. Ensure the local HTTP service is available.
2. Start the dashboard-capable server when needed.
3. Attempt to open the default browser automatically.
4. Print the exact local URL either way.
5. Support `--no-open` for headless, remote, or scripted usage.

The dashboard should preserve the OpenClaw-like feel of "one command opens the local control UI" without turning Babata into a split deployment model.

### 5.2 Mental Model

The UI should teach the user that:

- they are observing a set of concurrent tasks
- each task has control state and semantic state
- semantic state matters more than raw status labels for long-running work
- tasks may be part of a tree
- the dashboard is a runtime console, not a persistent chat session

## 6. Technical Architecture

### 6.1 Repository Shape

Babata remains a single local application, but the repository gains a lightweight frontend:

```text
/
├─ src/                  # Rust application and HTTP server
├─ web/                  # Vite + React + TypeScript dashboard
└─ docs/superpowers/specs/
```

### 6.2 Runtime Model

- Rust remains the system entrypoint and the owner of HTTP routes, task data access, and CLI.
- The dashboard frontend is built as static assets.
- Production assets are served by the Rust HTTP server.
- Development mode may use a separate dev server, but that is a developer convenience only.
- API routes live under `/api/...`.
- Non-API dashboard routes serve the frontend shell.

### 6.3 Technology Choices

- Frontend: `Vite + React + TypeScript`
- Backend: existing `axum` HTTP service
- State refresh: polling
- Asset serving: Rust-hosted static bundle

This is intentionally lighter than a fully separated SPA platform, while still being maintainable for task explorer and detail interactions.

## 7. Information Architecture

The dashboard exposes four top-level views:

- `Overview`
- `Tasks`
- `Create`
- `System`

Navigation should stay restrained. The product is task-centric, so these views are sufficient for the first release.

### 7.1 Overview

Purpose: answer "what is the system doing right now?"

Primary modules:

- status summary blocks for `running`, `paused`, `done`, `canceled`, `total`
- active attention area for the tasks most likely to need action
- recent tasks
- recent completions
- visible create-task entrypoint

This page is the default landing view for `babata dashboard`.

### 7.2 Tasks

Purpose: browse, filter, and control tasks.

Primary modules:

- filter and query toolbar
- task list
- quick actions per row
- direct navigation to task detail

Expected filters:

- `status`
- `agent`
- `root_only`
- `never_ends`
- text `query`
- pagination controls

### 7.3 Task Detail

Purpose: answer "what is this task doing, where is it blocked, and what can I do?"

This is the product's core page. It should be structured around five panels:

1. `Summary`
2. `Semantic State`
3. `Tree`
4. `Runtime`
5. `Outputs`

Panel intent:

- `Summary`: stable metadata and top-level actions
- `Semantic State`: `progress.md` and `task.md`, with `progress.md` visually primary
- `Tree`: parent, current node, and child task navigation
- `Runtime`: control actions, latest action result, refresh state
- `Outputs`: logs and artifacts when available

### 7.4 Create

Purpose: create a task explicitly and predictably.

Form fields stay minimal:

- `prompt`
- `agent`
- `parent_task_id`
- `never_ends`

On success, the user should be redirected to the new task detail page.

### 7.5 System

Purpose: provide operational context without becoming a settings product.

Initial modules:

- service health
- data directory
- listen address / URL
- application version
- frontend build version

## 8. API Design

The dashboard should not be forced to over-compose thin primitive endpoints. Babata should keep existing routes where useful, but add a UI-oriented API surface under `/api`.

### 8.1 Overview Endpoint

`GET /api/overview`

Returns the aggregate data required by the overview screen:

- status counts
- active tasks
- recent tasks
- recent completions
- optionally attention-worthy tasks such as paused tasks or tasks awaiting user intervention

### 8.2 Task List Endpoint

`GET /api/tasks`

Supports:

- `status`
- `agent`
- `root_only`
- `never_ends`
- `query`
- `limit`
- `offset`

List items should include enough fields for row display and common actions without requiring follow-up detail requests.

### 8.3 Task Summary Endpoint

`GET /api/tasks/:task_id`

Returns:

- metadata
- parent/root relationships
- status
- agent
- created timestamp
- action availability
- lightweight observability hints relevant to the detail shell

### 8.4 Task Content Endpoint

`GET /api/tasks/:task_id/content`

Returns the semantic recovery surface:

- `task.md`
- `progress.md`
- selected agent-specific files when available and stable enough to expose

This endpoint is critical because Babata's value is not captured by metadata alone.

### 8.5 Task Tree Endpoint

`GET /api/tasks/:task_id/tree`

Returns the task tree slice needed for UI navigation. Minimum useful shape:

- parent task
- current task
- direct children

If implementation cost is reasonable, returning the whole root tree is preferred.

### 8.6 Task Logs Endpoint

`GET /api/tasks/:task_id/logs`

This endpoint should use capability detection:

- if agent-specific logs are available, return them
- if only partial logs are available, expose partial logs clearly
- if logs are not available, return an explicit empty or unsupported state

The dashboard must not invent a unified logging model that the backend does not actually have.

### 8.7 Task Artifacts Endpoint

`GET /api/tasks/:task_id/artifacts`

Returns:

- artifact file list
- basic metadata
- text preview support where appropriate

Binary inspection can remain out of scope for the first release.

### 8.8 Task Mutation Endpoints

The dashboard uses UI-facing API paths for:

- `POST /api/tasks`
- `POST /api/tasks/:task_id/pause`
- `POST /api/tasks/:task_id/resume`
- `POST /api/tasks/:task_id/cancel`
- `POST /api/tasks/:task_id/relaunch`

Mutation responses should be explicit enough for the UI to display immediate success or failure feedback.

### 8.9 System Endpoint

`GET /api/system`

Returns:

- health state
- listen address / URL
- data directory
- server version
- dashboard build version

## 9. Data Model and Backend Constraints

The dashboard should align with Babata's actual persistence model:

- structured control state comes from `task.db`
- semantic state comes from task directory files
- artifacts come from task directories

The first release should preserve "unified skeleton + agent-specific panels":

- common task structure is always visible
- agent-specific observability can appear when present
- missing agent-specific data must render as unavailable, not broken

### 9.1 Historical Retention

The dashboard requires task detail pages to remain inspectable after completion or cancellation.

Therefore, first-release implementation should align behavior with the architecture intent that task directories are retained for:

- `done`
- `canceled`
- `paused`

If current behavior still deletes root task trees after completion or cancelation, that behavior must be corrected as part of the dashboard work.

## 10. Refresh Model

The first release uses polling, not streaming.

Rationale:

- the main dashboard value comes from stable observability and control, not high-frequency event delivery
- polling is enough for overview status, task transitions, and semantic progress inspection
- streaming is most valuable later for logs and possibly progress updates, but it should not block the first release

### 10.1 Polling Strategy

- `Overview`: periodic refresh, default every 5 seconds
- `Task Detail`: periodic refresh, default every 5 seconds
- mutation success: immediate refetch of affected data
- logs and artifacts: load on demand, with explicit refresh when needed

### 10.2 User Controls

The UI should expose:

- last refresh timestamp
- auto-refresh enabled state
- manual refresh action

This keeps the refresh model legible and avoids implying stronger realtime guarantees than the system provides.

## 11. Interaction Design

### 11.1 Control Actions

`pause`, `resume`, `cancel`, and `relaunch` must provide clear action feedback:

- entering a pending state while request is in flight
- optimistic UI only where safe
- concise success feedback
- explicit error feedback with backend message surfaced

Additional constraints:

- `cancel` requires confirmation
- `relaunch` requires a non-empty reason

### 11.2 Empty and Unsupported States

The dashboard should be explicit when data does not exist:

- no logs available
- no artifacts produced
- no child tasks
- agent-specific panel not supported

This is better than fake placeholders that imply missing backend work is already implemented.

## 12. Visual Direction

The selected design direction is `Forge Panels`.

The dashboard should feel:

- heavy
- tool-like
- operational
- local-runtime-oriented

It should not feel like:

- a generic SaaS admin
- a default template with interchangeable cards
- a chat product

### 12.1 Visual Constraints

- dark, industrial base palette
- warm metal or signal-color accents instead of purple branding defaults
- panel, rail, and block composition rather than airy card grids
- strong hierarchy for status, semantic progress, and control affordances
- desktop-first layout that still remains usable on narrow screens

## 13. Frontend Composition

The frontend should be decomposed into clear units:

- app shell and routing
- overview module
- task explorer module
- task detail module
- create task module
- system module
- API client layer
- polling hooks
- shared UI primitives such as status badge, panel, toolbar, dialogs, and empty states

This keeps the lightweight frontend maintainable without over-architecting it into a platform.

## 14. Delivery Boundaries

The first dashboard release should deliver:

- `babata dashboard`
- browser auto-open with `--no-open`
- embedded dashboard frontend
- overview page
- task explorer
- task detail with semantic state emphasis
- task tree
- artifact visibility
- log visibility where supported
- polling-based refresh
- system page

It should deliberately defer:

- generalized streaming infrastructure
- transcript-style chat replay
- full settings management
- multi-user auth and remote deployment concerns

## 15. Success Criteria

The dashboard design is successful if a user can:

1. run `babata dashboard`
2. land in a browser on a clear local overview page
3. identify active and problematic tasks quickly
4. create a new task
5. inspect a task's `progress.md` and `task.md`
6. navigate task relationships
7. control a task safely
8. inspect outputs where available
9. understand what data is unavailable without being misled

