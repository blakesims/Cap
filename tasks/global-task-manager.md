# Global Task Manager

Tracks all non-archived work (PLANNING, ACTIVE, PAUSED, ONGOING). Completed items may be moved out of this table.

## Current Tasks

| ID | Task Name | Priority(1-5) | Stories (Done/Total) | Status | Dependencies | Rules Required | Link |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| T001 | Keyboard-Driven Editor | 2 | 6/6 | ACTIVE | context.ts, useEditorShortcuts.ts, Timeline, Player | CLAUDE.md | [main.md](./active/T001-keyboard-driven-editor/main.md) |
| T002 | CLI Export Queue System | 3 | 0/5 | PLANNING | apps/cli/, crates/export/ | CLAUDE.md | [main.md](./planning/T002-cli-export-queue/main.md) |
| T003 | Segment Deletion UX Improvements | 2 | 3/3 | COMPLETED | context.ts, Player.tsx, ClipTrack.tsx, ConfigSidebar.tsx | CLAUDE.md | [main.md](./completed/T003-segment-deletion-ux/main.md) |
| T004 | Editor Playback Improvements | 2 | 2/2 | COMPLETED | crates/editor/, apps/desktop/src/routes/editor/ | CLAUDE.md | [main.md](./completed/T004-editor-playback-improvements/main.md) |
| T005 | Split-Screen with Animated Text Overlays | 2 | 5/5 | COMPLETE | crates/rendering/, crates/project/, apps/desktop/src/routes/editor/ | CLAUDE.md | [main.md](./active/T005-split-screen-text-overlays/main.md) |
| T006 | Overlay Track System | 1 | 0/6 | READY | T005, crates/rendering/, apps/desktop/src/routes/editor/ | CLAUDE.md | [main.md](./active/T006-overlay-track/main.md) |

Next available task id: T007
(When a new task is created, increment this in the same change.)
Important: When creating tasks they be given a directory and continuously updated.
Eg. create the task in `./tasks/planning/` and move to `./tasks/active/` when started working on it.
Once completed move into `./tasks/completed/`.
Make sure this tracker is kept up to date

---

**Task Status Legend:**
-  **PLANNING**: Defined but not started.
-  **ACTIVE**: Currently being worked on.
-  **ONGOING**: Recurring maintenance/workstream.
-  **PAUSED**: Blocked or intentionally stopped; include reason in task doc.
-  **COMPLETED**: Finished and reviewed.
-  **ARCHIVED**: Stored for reference only.
