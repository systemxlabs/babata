# Babata Tasks Workbench Design

## 1. Background

Babata already has a functioning dashboard shell and task pages, but the current UI still behaves like a generic admin panel:

- navigation sits across the top instead of acting like a control-console rail
- the visual palette is dark and heavy
- the `Tasks` page is still a filtered flat list
- task relationships are visible only in isolated detail views
- task-produced files are not available as a first-class workspace surface

The next iteration should make the dashboard feel closer to an operational console such as OpenClaw: narrow left navigation, brighter surfaces, and a task-centric workspace on the right.

This design is a focused follow-up to `docs/superpowers/specs/2026-03-25-babata-dashboard-design.md`. It does not redefine the whole dashboard product. It defines the next UI slice for shell layout and the `Tasks` workbench.

## 2. Scope

This design covers:

- a new global shell layout with a compact left sidebar
- a brighter visual theme across the dashboard
- a redesigned `Tasks` page built as a single workbench
- three primary `Tasks` surfaces:
  - `Root task list`
  - `Task tree`
  - `Task folder`
- a default root-task timeline with an optional switch to all-task time flow

This design explicitly does not cover:

- `Workspace folder`
- realtime streaming transport
- broader configuration UX changes
- redesigning every page beyond what is needed to fit the new shell

## 3. Product Decisions Confirmed

The following decisions were confirmed during brainstorming:

- Layout direction: OpenClaw-like shell with left navigation and right content
- Sidebar density: more compact than the current shell
- `Tasks` page architecture: single-page workbench
- `Root task list` behavior: default to root tasks only, with a switch to show all tasks in time order
- `Task folder` behavior: file tree plus file content preview
- `Workspace folder`: out of scope for this iteration

## 4. Goals

The redesign should:

1. Replace the current top-heavy shell with a compact left-rail application layout.
2. Shift the dashboard to a bright operational palette without losing clear task-state emphasis.
3. Make `Tasks` usable as a task relationship workspace, not just a list view.
4. Let users move quickly between root tasks, child tasks, and task-produced files.
5. Preserve URL-addressable UI state for the selected mode, root task, task, and file.

The redesign should not:

- turn the `Tasks` page into a generalized IDE
- require workspace browsing before task browsing is useful
- depend on speculative backend systems that do not support the confirmed UI

## 5. User Experience

### 5.1 Global Shell

The dashboard shell becomes a two-column application frame:

- left: narrow global sidebar
- right: active page content

The sidebar should feel stable and compact. It should prioritize:

- application identity
- top-level navigation
- current section highlight

It should not repeat the large hero masthead currently shown on every page.

### 5.2 Visual Direction

The dashboard should move from the current dark forge-like palette to a brighter, operational palette:

- page background: off-white or pale gray with subtle warmth
- panels: near-white or very light neutral surfaces
- borders: light but clearly visible
- active accents: warm orange-red derived from the current brand direction
- status colors: preserved but tuned for light backgrounds

The visual target is not "white SaaS dashboard". It should still feel purposeful and instrument-like:

- denser spacing than marketing UI
- strong separators between navigation, list, tree, and file areas
- restrained motion
- compact controls

## 6. Information Architecture

### 6.1 Global Navigation

Top-level destinations remain:

- `Overview`
- `Tasks`
- `Create`
- `System`

The change is structural, not taxonomic. The same views stay available, but navigation moves into the left rail.

### 6.2 Tasks Page

The `Tasks` page becomes a single workbench composed of three primary modules:

1. `Root task list`
2. `Task tree`
3. `Task folder`

Layout:

- upper row:
  - left: `Root task list`
  - right: `Task tree`
- lower row:
  - full width: `Task folder` with file tree and preview

This keeps the relationship views visible together:

- list answers "which task chain am I in?"
- tree answers "where in the chain am I?"
- folder answers "what did this specific task produce?"

## 7. Tasks Workbench Behavior

### 7.1 Root Task List

Default mode:

- show root tasks only
- sort by `created_at` descending

Optional alternate mode:

- switch to `all tasks`
- still sort by `created_at` descending
- preserve root context by showing root-related metadata in each row

Each row should display enough context to scan quickly:

- description
- task status
- agent
- created time
- short task id
- never-ends indicator when applicable

The selected row determines the active root context for the page.

### 7.2 Task Tree

The task tree shows the task structure for the active root context.

Requirements:

- tree nodes support expand and collapse
- selected node is visibly highlighted
- each node shows:
  - description
  - status
  - short id
  - created time or relative recency
- clicking a node updates the active task selection without leaving the page

The tree must remain relationship-oriented, not log-oriented. It is for navigating hierarchy quickly.

### 7.3 Task Folder

The task folder area is scoped to the selected task, not the root task as a whole.

Structure:

- left side: file tree
- right side: file preview

Expected behavior:

- reconstruct directories from artifact paths
- support nested browsing
- preview text files inline
- show metadata and a non-preview state for non-text files
- preserve currently selected file when possible while switching tasks

This area is intentionally a task-output browser, not a general workspace explorer.

## 8. Interaction Model

### 8.1 Initial Selection

When the `Tasks` page loads:

1. fetch the default root-task list
2. select the first row automatically when data exists
3. load the tree for that root context
4. select a default task node
5. load that task's folder contents

The default selected task may be:

- the root task itself, or
- the most recent active node within the selected root

Implementation may choose the simpler option first, but the behavior must be deterministic.

### 8.2 List-to-Tree Link

When the user clicks a row in `Root task list`:

- selected root context changes
- `Task tree` updates to that root
- selected task updates to the root or the chosen default node
- `Task folder` updates to the selected task

### 8.3 Tree-to-Folder Link

When the user clicks a node in `Task tree`:

- root context stays the same
- selected task changes
- `Task folder` updates to the clicked task

### 8.4 File Preview Link

When the user clicks a file in the task folder:

- selected file changes
- preview updates in place
- page navigation should not change

## 9. URL State

The workbench should preserve meaningful state in the URL so refresh and deep-linking remain useful.

Expected state keys:

- `view=root|timeline`
- `root_task_id`
- `task_id`
- `file`

This is important because the page is becoming stateful. Without URL persistence, reloading would be disruptive during investigation work.

## 10. Data Requirements

### 10.1 Existing Data That Can Be Reused

The current API surface already supports key parts of the design:

- task list data from `/api/tasks`
- task detail metadata from `/api/tasks/:task_id`
- local tree context from `/api/tasks/:task_id/tree`
- artifact listing from `/api/tasks/:task_id/artifacts`
- task content and logs from their existing endpoints

These are sufficient for:

- root task list
- all-task timeline
- task folder tree reconstruction
- text preview for task-scoped files

### 10.2 Data Gap

The current tree endpoint is not sufficient for a complete root-scoped expandable tree because it returns only local context (`parent`, `current`, `children`) rather than a full recursive structure.

The preferred backend addition is a root-scoped tree endpoint that returns the whole task hierarchy for a selected root task.

Preferred API shape:

`GET /api/tasks/:root_task_id/tree?scope=root`

Expected payload characteristics:

- root task node
- recursive child nodes
- summary data per node sufficient for tree rendering

### 10.3 Fallback Strategy

If the backend tree endpoint is not added in the first implementation batch, the frontend may temporarily construct a partial tree from the flat task list using:

- `task_id`
- `parent_task_id`
- `root_task_id`

This is acceptable only as a short-term implementation step. The design target remains a true root-scoped tree API.

## 11. Responsive Behavior

Desktop is the primary target.

Expected desktop behavior:

- compact fixed-width sidebar
- two-up relationship row
- full-width lower file surface

Tablet and narrow widths should degrade by stacking:

- root task list
- task tree
- task folder

The left sidebar may collapse to icon-first mode on smaller widths, but the mental model must remain the same.

## 12. Testing Strategy

This redesign needs behavior tests, not only rendering snapshots.

Frontend tests should cover:

- shell navigation rendering and active-state behavior
- root-only vs all-task timeline switching
- row selection updates tree and folder state
- tree node selection updates folder state
- file selection updates preview state
- URL state reflects current workbench state

Backend tests should cover any new tree endpoint or file-preview support added to serve this workbench.

## 13. Delivery Strategy

The redesign should be implemented in two phases.

### Phase 1

- replace the global shell with compact left navigation
- apply the brighter visual theme
- build the `Tasks` workbench layout
- implement root-task list and timeline toggle
- implement task folder file tree and text preview

### Phase 2

- add or upgrade backend support for a full root-scoped recursive tree
- replace any temporary partial-tree frontend logic
- refine URL restoration and default selection behavior

## 14. Success Criteria

The redesign is successful when:

- the dashboard reads as a left-rail console instead of a top-nav dashboard
- `Tasks` supports real hierarchical investigation without leaving the page
- the user can move from root task to child task to file preview in a single surface
- the UI remains usable and readable in a bright theme
- implementation can proceed without ambiguity about scope or interaction behavior
