# Babata Tasks Workbench Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the dashboard into a bright left-rail console and turn `Tasks` into a single-page workbench with a root-task timeline, expandable task tree, and task-folder browser.

**Architecture:** Keep the existing React + Vite frontend embedded in the Rust server, but replace the top-nav shell with a compact sidebar layout. Extend the HTTP layer with a root-scoped recursive task-tree payload and an artifact-content endpoint so the frontend can render a stable workbench without over-fetching or reconstructing incomplete data from local tree fragments.

**Tech Stack:** Rust, Axum, serde, tokio, React, TypeScript, react-router-dom, Vitest, Testing Library

---

## Planned File Map

### Rust HTTP and task-query layer

- Modify: `src/http/get_task_tree.rs`
  Replace the local-context tree response with a root-scoped recursive tree payload that the workbench can render directly.
- Modify: `src/http/mod.rs`
  Keep route registration aligned with any new task-tree or artifact-preview endpoints.
- Modify: `src/http/list_task_artifacts.rs`
  Keep artifact listing focused on metadata and directory reconstruction; stop relying on per-file inline previews as the main UI transport.
- Create: `src/http/get_task_artifact_content.rs`
  Return text preview content for a selected artifact path and explicit unsupported responses for non-text files or missing files.
- Modify: `src/http/get_task.rs`
  Preserve task-summary fields required by workbench rows and tree nodes.
- Modify: `src/task/manager.rs`
  Expose any helper needed to validate task ownership before serving artifact content.
- Modify: `tests/dashboard_api.rs`
  Add root-tree and artifact-content API coverage.

### Frontend shell and shared task workbench models

- Modify: `web/src/App.tsx`
  Remove the current masthead-heavy shell composition and adopt the compact sidebar-first app frame.
- Modify: `web/src/App.test.tsx`
  Assert the new shell renders left navigation and still lands in the default route successfully.
- Modify: `web/src/components/AppShell.tsx`
  Rebuild the shell markup around a compact left rail and right-side content column.
- Create: `web/src/components/SidebarNav.tsx`
  Hold the global navigation rail so shell structure stays focused and testable.
- Modify: `web/src/components/Panel.tsx`
  Support the lighter workbench surface styling without duplicating shell-specific wrappers.
- Modify: `web/src/components/StatusBadge.tsx`
  Tune task-state presentation for the bright theme while preserving semantics.
- Modify: `web/src/styles/app.css`
  Replace the dark forge palette with the bright console theme and add layout rules for the new shell and workbench.
- Modify: `web/src/api/types.ts`
  Add recursive task-tree and artifact-content response types plus any UI-facing node shapes.
- Create: `web/src/utils/tasks.ts`
  Centralize root-list, timeline, tree-selection, and artifact-tree shaping helpers.
- Create: `web/src/utils/tasks.test.ts`
  Verify timeline grouping, recursive tree building, and artifact-path tree reconstruction with pure unit tests.

### Tasks workbench UI

- Modify: `web/src/pages/TasksPage.tsx`
  Replace the flat filter/list page with the three-surface workbench and URL-backed view state.
- Modify: `web/src/pages/TasksPage.test.tsx`
  Cover root-only mode, timeline toggle, tree selection, and file preview behavior.
- Create: `web/src/components/tasks/RootTaskList.tsx`
  Render the default root-only list and the alternate all-task time flow.
- Create: `web/src/components/tasks/TaskTreePane.tsx`
  Render the recursive tree with expand/collapse behavior.
- Create: `web/src/components/tasks/TaskFolderPane.tsx`
  Render artifact directories on the left and selected-file preview on the right.
- Create: `web/src/components/tasks/WorkbenchToolbar.tsx`
  Hold the `root / timeline` switch and any compact task-scoped controls needed by the page.

### Secondary page adaptation and verification

- Modify: `web/src/pages/OverviewPage.tsx`
- Modify: `web/src/pages/CreatePage.tsx`
- Modify: `web/src/pages/SystemPage.tsx`
  Adjust spacing and card composition so these pages sit correctly inside the new shell without the removed masthead assumptions.
- Modify: `web/src/pages/OverviewPage.test.tsx`
- Modify: `web/src/pages/CreatePage.test.tsx`
- Modify: `web/src/pages/SystemPage.test.tsx`
  Keep the page-level tests aligned with the new shell and theme assumptions.

## Task 1: Add Root-Scoped Tree and Artifact-Content APIs

**Files:**
- Modify: `src/http/get_task_tree.rs`
- Create: `src/http/get_task_artifact_content.rs`
- Modify: `src/http/list_task_artifacts.rs`
- Modify: `src/http/mod.rs`
- Modify: `src/task/manager.rs`
- Modify: `tests/dashboard_api.rs`

- [ ] **Step 1: Write the failing API tests**

Add a recursive tree test in `tests/dashboard_api.rs`:

```rust
#[tokio::test]
async fn task_tree_returns_recursive_root_hierarchy() {
    let response = app
        .oneshot(Request::builder().uri(format!("/api/tasks/{task_id}/tree")).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    assert!(body["root"].is_object());
    assert!(body["root"]["children"].is_array());
}
```

Add an artifact-content test:

```rust
#[tokio::test]
async fn task_artifact_content_returns_text_preview_for_selected_file() {
    let response = app
        .oneshot(Request::builder().uri(format!("/api/tasks/{task_id}/artifacts/content?path=notes/output.md")).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    assert_eq!(body["path"], "notes/output.md");
    assert_eq!(body["is_text"], true);
}
```

- [ ] **Step 2: Run the targeted API tests and verify RED**

Run: `cargo test --test dashboard_api task_tree_returns_recursive_root_hierarchy -- --exact`
Expected: FAIL because the current tree payload only returns `parent/current/children`.

Run: `cargo test --test dashboard_api task_artifact_content_returns_text_preview_for_selected_file -- --exact`
Expected: FAIL because the artifact-content route does not exist.

- [ ] **Step 3: Implement the minimal Rust API changes**

Change `src/http/get_task_tree.rs` to produce a recursive node shape based on `TaskManager::list_root_tree(...)`, for example:

```rust
#[derive(Debug, Serialize)]
struct TaskTreeNodeResponse {
    task: TaskResponse,
    children: Vec<TaskTreeNodeResponse>,
}
```

Add `src/http/get_task_artifact_content.rs` with a query param:

```rust
#[derive(Debug, Deserialize)]
struct ArtifactContentQuery {
    path: String,
}
```

Return a small JSON payload with:

- `task_id`
- `path`
- `is_text`
- `size_bytes`
- `content`
- `reason` when preview is unsupported

In `src/http/list_task_artifacts.rs`, keep the listing endpoint focused on file metadata:

```rust
struct ArtifactResponse {
    path: String,
    size_bytes: u64,
    is_text: bool,
}
```

Register the new route in `src/http/mod.rs`:

```rust
.route("/tasks/{task_id}/artifacts/content", get(get_task_artifact_content::handle))
```

- [ ] **Step 4: Run the targeted tests to verify GREEN**

Run: `cargo test --test dashboard_api task_tree_returns_recursive_root_hierarchy -- --exact`
Expected: PASS

Run: `cargo test --test dashboard_api task_artifact_content_returns_text_preview_for_selected_file -- --exact`
Expected: PASS

- [ ] **Step 5: Commit the API slice**

```bash
git add src/http/get_task_tree.rs src/http/get_task_artifact_content.rs src/http/list_task_artifacts.rs src/http/mod.rs src/task/manager.rs tests/dashboard_api.rs
git commit -m "feat: add workbench tree and artifact content APIs"
```

## Task 2: Build Pure Frontend Data Helpers for Timeline, Tree, and File Surfaces

**Files:**
- Modify: `web/src/api/types.ts`
- Create: `web/src/utils/tasks.ts`
- Create: `web/src/utils/tasks.test.ts`

- [ ] **Step 1: Write the failing pure-data tests**

Create `web/src/utils/tasks.test.ts` with focused tests:

```ts
import { buildArtifactTree, buildTaskTree, selectInitialTaskId } from './tasks';

test('buildTaskTree nests descendants beneath the selected root task', () => {
  expect(buildTaskTree(tasks, rootId).children).toHaveLength(1);
});

test('buildArtifactTree groups slash-delimited paths into directories', () => {
  expect(buildArtifactTree([{ path: 'notes/output.md', size_bytes: 12, is_text: true }])).toMatchObject({
    children: expect.arrayContaining([expect.objectContaining({ name: 'notes', kind: 'directory' })]),
  });
});
```

Also add a test for the default list mode:

```ts
test('deriveRootTaskRows returns root tasks in descending created_at order', () => {
  expect(deriveRootTaskRows(tasks).map((task) => task.task_id)).toEqual([newerRootId, olderRootId]);
});
```

- [ ] **Step 2: Run the helper tests and verify RED**

Run: `npm test -- --run web/src/utils/tasks.test.ts`
Expected: FAIL because the helper module and new response types do not exist.

- [ ] **Step 3: Implement the minimal TypeScript helpers**

Add `web/src/api/types.ts` shapes for the new API contracts:

```ts
export interface TaskTreeNode {
  task: TaskSummary;
  children: TaskTreeNode[];
}

export interface RootTaskTreeResponse {
  root_task_id: string;
  root: TaskTreeNode;
}

export interface TaskArtifactContentResponse {
  task_id: string;
  path: string;
  is_text: boolean;
  size_bytes: number;
  content: string | null;
  reason?: string;
}
```

Implement helper functions in `web/src/utils/tasks.ts`:

- `deriveRootTaskRows(tasks)`
- `deriveTimelineRows(tasks)`
- `buildTaskTree(tasks, rootTaskId)` for fallback shaping
- `flattenTreeIds(node)` or similar for expansion state
- `buildArtifactTree(artifacts)`
- `selectInitialTaskId(rootTree)`

Keep these pure so the UI components do not own sorting or graph reconstruction logic.

- [ ] **Step 4: Re-run the helper tests to verify GREEN**

Run: `npm test -- --run web/src/utils/tasks.test.ts`
Expected: PASS

- [ ] **Step 5: Commit the frontend data layer**

```bash
git add web/src/api/types.ts web/src/utils/tasks.ts web/src/utils/tasks.test.ts
git commit -m "feat: add task workbench view models"
```

## Task 3: Rebuild the Dashboard Shell Around a Compact Left Rail and Bright Theme

**Files:**
- Modify: `web/src/App.tsx`
- Modify: `web/src/App.test.tsx`
- Modify: `web/src/components/AppShell.tsx`
- Create: `web/src/components/SidebarNav.tsx`
- Modify: `web/src/components/Panel.tsx`
- Modify: `web/src/components/StatusBadge.tsx`
- Modify: `web/src/styles/app.css`
- Modify: `web/src/pages/OverviewPage.tsx`
- Modify: `web/src/pages/CreatePage.tsx`
- Modify: `web/src/pages/SystemPage.tsx`
- Modify: `web/src/pages/OverviewPage.test.tsx`
- Modify: `web/src/pages/CreatePage.test.tsx`
- Modify: `web/src/pages/SystemPage.test.tsx`

- [ ] **Step 1: Write the failing shell tests**

Update `web/src/App.test.tsx` so it expects the compact rail instead of the current masthead:

```ts
test('renders the dashboard shell with left navigation rail', async () => {
  render(<App />);

  expect(await screen.findByRole('navigation', { name: 'Primary' })).toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'Tasks' })).toBeInTheDocument();
  expect(screen.queryByRole('heading', { name: 'Local task control plane' })).not.toBeInTheDocument();
});
```

Add or update page tests so they assert page content still renders inside the new shell.

- [ ] **Step 2: Run the shell and page tests and verify RED**

Run: `npm test -- --run web/src/App.test.tsx web/src/pages/OverviewPage.test.tsx web/src/pages/CreatePage.test.tsx web/src/pages/SystemPage.test.tsx`
Expected: FAIL because the current shell still renders the old masthead and dark theme structure.

- [ ] **Step 3: Implement the minimal shell/theme redesign**

Create a focused rail component in `web/src/components/SidebarNav.tsx`:

```tsx
export function SidebarNav({ items }: SidebarNavProps) {
  return (
    <nav aria-label="Primary" className="sidebar-nav">
      {items.map((item) => (
        <NavLink key={item.href} to={item.href} className={...}>
          <span>{item.label}</span>
        </NavLink>
      ))}
    </nav>
  );
}
```

Refactor `web/src/components/AppShell.tsx` into:

- left `aside` rail
- right content column
- compact toolbar area at the top of the content column

In `web/src/styles/app.css`:

- replace the dark root palette with light theme tokens
- add compact left-rail sizing
- add workbench-friendly surface spacing
- preserve responsive stacking for smaller widths

Update the non-task pages only enough to sit correctly in the new shell. Do not redesign their data flows in this task.

- [ ] **Step 4: Re-run the shell tests to verify GREEN**

Run: `npm test -- --run web/src/App.test.tsx web/src/pages/OverviewPage.test.tsx web/src/pages/CreatePage.test.tsx web/src/pages/SystemPage.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit the shell redesign**

```bash
git add web/src/App.tsx web/src/App.test.tsx web/src/components/AppShell.tsx web/src/components/SidebarNav.tsx web/src/components/Panel.tsx web/src/components/StatusBadge.tsx web/src/styles/app.css web/src/pages/OverviewPage.tsx web/src/pages/OverviewPage.test.tsx web/src/pages/CreatePage.tsx web/src/pages/CreatePage.test.tsx web/src/pages/SystemPage.tsx web/src/pages/SystemPage.test.tsx
git commit -m "feat: redesign dashboard shell with compact sidebar"
```

## Task 4: Replace the Flat Tasks Page with the Workbench

**Files:**
- Modify: `web/src/pages/TasksPage.tsx`
- Modify: `web/src/pages/TasksPage.test.tsx`
- Create: `web/src/components/tasks/WorkbenchToolbar.tsx`
- Create: `web/src/components/tasks/RootTaskList.tsx`
- Create: `web/src/components/tasks/TaskTreePane.tsx`
- Create: `web/src/components/tasks/TaskFolderPane.tsx`
- Modify: `web/src/api/client.ts`

- [ ] **Step 1: Write the failing workbench behavior tests**

Expand `web/src/pages/TasksPage.test.tsx` with behaviors that reflect the approved design:

```ts
test('tasks page defaults to root-task mode and fetches root-only tasks', async () => {
  render(
    <MemoryRouter initialEntries={['/tasks']}>
      <TasksPage />
    </MemoryRouter>,
  );

  expect(await screen.findByRole('button', { name: 'Root tasks' })).toHaveAttribute('aria-pressed', 'true');
  expect(fetchMock).toHaveBeenCalledWith('/api/tasks?root_only=true', expect.anything());
});
```

Add tests for:

- toggling to all-task timeline mode
- clicking a root row updates the tree pane
- clicking a tree node loads task folder content
- clicking a file updates the preview pane
- URL search params keep `view`, `root_task_id`, `task_id`, and `file`

- [ ] **Step 2: Run the page test and verify RED**

Run: `npm test -- --run web/src/pages/TasksPage.test.tsx`
Expected: FAIL because the current page is still a flat filter/list view.

- [ ] **Step 3: Implement the minimal workbench UI**

Refactor `web/src/pages/TasksPage.tsx` so it fetches:

- `/api/tasks?root_only=true` for default mode
- `/api/tasks` for timeline mode
- `/api/tasks/${selectedTaskId}/tree` for recursive tree data
- `/api/tasks/${selectedTaskId}/artifacts` for metadata
- `/api/tasks/${selectedTaskId}/artifacts/content?path=...` for preview content

Break rendering into focused components:

`RootTaskList.tsx`

```tsx
export function RootTaskList({ rows, selectedTaskId, onSelect }: RootTaskListProps) {
  return <ul>{/* clickable task rows */}</ul>;
}
```

`TaskTreePane.tsx`

```tsx
export function TaskTreePane({ root, selectedTaskId, expandedIds, onToggle, onSelect }: TaskTreePaneProps) {
  return <div>{/* recursive nodes */}</div>;
}
```

`TaskFolderPane.tsx`

```tsx
export function TaskFolderPane({ tree, selectedPath, preview, onSelectFile }: TaskFolderPaneProps) {
  return <section>{/* file tree + preview */}</section>;
}
```

Use `useSearchParams` to keep state in the URL:

- `view`
- `root_task_id`
- `task_id`
- `file`

Keep manual refresh wired through the existing toolbar flow. Do not reintroduce the old filter form in this task.

- [ ] **Step 4: Re-run the Tasks page tests to verify GREEN**

Run: `npm test -- --run web/src/pages/TasksPage.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit the workbench page**

```bash
git add web/src/pages/TasksPage.tsx web/src/pages/TasksPage.test.tsx web/src/components/tasks/WorkbenchToolbar.tsx web/src/components/tasks/RootTaskList.tsx web/src/components/tasks/TaskTreePane.tsx web/src/components/tasks/TaskFolderPane.tsx web/src/api/client.ts
git commit -m "feat: build tasks workbench UI"
```

## Task 5: Run Full Verification and Fix Remaining Regressions

**Files:**
- Modify: any files touched by regressions found in verification

- [ ] **Step 1: Run the full frontend test suite**

Run: `npm test -- --run`
Expected: PASS

- [ ] **Step 2: Run the dashboard API regression suite**

Run: `cargo test --test dashboard_api -- --nocapture`
Expected: PASS

- [ ] **Step 3: Run a production frontend build**

Run: `npm run build`
Expected: PASS

- [ ] **Step 4: Run a Rust formatting pass and targeted test rerun if needed**

Run: `cargo fmt --all`
Expected: PASS with no diff or only intended formatting changes.

If formatting changes Rust files, re-run:

Run: `cargo test --test dashboard_api -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit the verification follow-ups**

```bash
git add -A
git commit -m "chore: finalize tasks workbench redesign"
```
