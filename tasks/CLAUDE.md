---
description: Task documentation procedures for AI agents.
---

## AI Task Documentation Procedures

### Core Task Creation Process

Definitions:
-    **Task** = epic-like container tracked in `tasks/global-task-manager.md`
-    **Story** = execution unit (story-like "phase") inside a Task

For new tasks:
1. Get next Task ID from `tasks/global-task-manager.md`.
2. Create folder: `tasks/planning/TXXX-task-slug/`
3. Copy template: `tasks/main-template.md` → `tasks/planning/TXXX-task-slug/main.md`
4. In the task folder, create `stories/` only if needed (see "Story Detail Rule" below).
5. Update GTM: add a row linking to the task and increment "Next available task id".
6. Fill out `main.md` with objective, scope, and a story list. Then update GTM fields (priority, status, stories done/total).

### Story Detail Rule (keep it lightweight)

Default: story-like phases live in `main.md` (inline).

Create a separate Story doc at `stories/SXX-story-slug.md` only if the story exceeds a small threshold:
-    Likely > 1-2 dev days, OR
-    More than ~5 acceptance criteria items, OR
-    Touches more than ~2 major components/directories, OR
-    High risk / easy to misunderstand / requires design decisions.

When a Story doc exists:
-    Keep the story row in the `main.md` story table
-    Put detailed AC + tasks/subtasks in the story doc
-    Link the row to the story doc

### Status / Directory Rules

-    PLANNING → `tasks/planning/`
-    ACTIVE → `tasks/active/`
-    ONGOING → `tasks/ongoing/`
-    PAUSED → `tasks/paused/`
-    COMPLETED → `tasks/completed/`
-    ARCHIVED → `tasks/archived/`

When status changes, move the task folder to the matching directory and update the GTM row (status + link).

Important: Tasks must be continuously updated in their directory.
Eg. create the task in `./tasks/planning/` and move to `./tasks/active/` when started working on it.
Once completed move into `./tasks/completed/`.
Make sure the `./tasks/global-task-manager.md` is kept up-to-date.
