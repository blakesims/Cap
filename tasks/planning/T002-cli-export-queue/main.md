# Task: [T002] - CLI Export Queue System

## 0. Task Summary
-  **Task Name:** CLI Export Queue System
-  **Priority:** 3
-  **Number of Stories:** 5
-  **Current Status:** PLANNING
-  **Dependencies:** `apps/cli/`, `crates/export/`, `cap-project`
-  **Rules Required:** CLAUDE.md
-  **Executor Ref:** Deliverable: S01-S05
-  **Acceptance Criteria:**
   - [ ] `cap queue add` command adds recordings to a JSON queue file
   - [ ] `cap queue run` processes queue sequentially with progress output
   - [ ] Machine stays awake during export via `caffeinate`
   - [ ] Successful exports can auto-delete originals after size validation
   - [ ] Queue state persists across CLI invocations

## 1. Goal / Objective
Build a queue-based CLI export system that allows batch exporting of Cap recordings without requiring the desktop UI, with support for keeping the machine awake, custom output naming, and optional cleanup of originals after successful export.

## 2. Overall Status
Planning phase - existing `apps/cli` has basic export command that can be extended.

## 3. Stories Breakdown

| Story ID | Story Name / Objective | Status | Deliverable | Link to Details |
| :--- | :--- | :--- | :--- | :--- |
| S01 | Queue data structure and persistence | Planned | Queue JSON file and types | Inline |
| S02 | `cap queue add` command | Planned | CLI subcommand | Inline |
| S03 | `cap queue run` with sequential processing | Planned | CLI subcommand | Inline |
| S04 | Caffeinate integration and progress output | Planned | Keep-awake wrapper | Inline |
| S05 | Post-export validation and cleanup | Planned | Size check + delete | Inline |

## 4. Story Details

### S01 - Queue data structure and persistence
-   **Acceptance Criteria:**
    -   [ ] Define `ExportJob` struct with: id, project_path, output_path, status, delete_original, added_at
    -   [ ] Define `ExportQueue` struct with jobs list and metadata
    -   [ ] Queue file location: `~/.cap/export-queue.json`
    -   [ ] Load/save functions with proper error handling
    -   [ ] Status enum: Pending, InProgress, Completed, Failed
-   **Tasks/Subtasks:**
    -   [ ] Create `apps/cli/src/queue.rs` module
    -   [ ] Define serde-compatible structs
    -   [ ] Implement `ExportQueue::load()` and `ExportQueue::save()`
    -   [ ] Handle missing/empty queue file gracefully

### S02 - `cap queue add` command
-   **Acceptance Criteria:**
    -   [ ] `cap queue add <project_path>` adds recording to queue
    -   [ ] `--output <path>` flag for custom output path
    -   [ ] `--delete-original` flag to mark for cleanup after export
    -   [ ] `--fps <n>` flag (default 60)
    -   [ ] `--resolution <WxH>` flag (default from recording)
    -   [ ] `--compression <level>` flag (Maximum/Social/Web/Potato)
    -   [ ] Validates project_path exists and is a .cap directory
    -   [ ] Prints confirmation with job ID
-   **Tasks/Subtasks:**
    -   [ ] Add `QueueAdd` args struct with clap
    -   [ ] Implement validation logic
    -   [ ] Generate UUID for job ID
    -   [ ] Append to queue and save

### S03 - `cap queue run` with sequential processing
-   **Acceptance Criteria:**
    -   [ ] Processes all Pending jobs in order
    -   [ ] Updates job status to InProgress before starting
    -   [ ] Updates job status to Completed/Failed after
    -   [ ] Saves queue state after each job (crash resilience)
    -   [ ] `--dry-run` flag shows what would be exported
    -   [ ] Progress output shows current job and frame count
-   **Tasks/Subtasks:**
    -   [ ] Add `QueueRun` args struct
    -   [ ] Loop through pending jobs
    -   [ ] Call existing export logic from `cap_export`
    -   [ ] Handle errors gracefully (mark Failed, continue to next)
    -   [ ] Print summary at end

### S04 - Caffeinate integration and progress output
-   **Acceptance Criteria:**
    -   [ ] `cap queue run` spawns `caffeinate -i` as child process
    -   [ ] Caffeinate is killed when queue processing completes
    -   [ ] Progress shows: `[2/5] Exporting "video name" - frame 1234/5678 (21%)`
    -   [ ] Estimated time remaining shown
    -   [ ] `--no-caffeinate` flag to disable
-   **Tasks/Subtasks:**
    -   [ ] Spawn caffeinate process on macOS
    -   [ ] Store process handle to kill on completion
    -   [ ] Implement progress callback with formatting
    -   [ ] Calculate ETA based on frames/second rate

### S05 - Post-export validation and cleanup
-   **Acceptance Criteria:**
    -   [ ] After export, verify output file exists
    -   [ ] Check output file size is reasonable (>1MB, or configurable minimum)
    -   [ ] If `delete_original` is set and validation passes, delete .cap folder
    -   [ ] Print warning before deletion with file sizes
    -   [ ] `--force` flag skips deletion confirmation
    -   [ ] Record deletion in queue (for audit trail)
-   **Tasks/Subtasks:**
    -   [ ] Add validation logic after export completes
    -   [ ] Implement size check (compare to estimates)
    -   [ ] Conditional deletion with confirmation prompt
    -   [ ] Update queue with deletion timestamp

## 5. Technical Considerations

### Existing Infrastructure
- `apps/cli/src/main.rs` already has working `export` command
- `cap_export::ExporterBase` handles all the heavy lifting
- `cap_export::mp4::Mp4ExportSettings` has all export options

### Queue File Format
```json
{
  "version": 1,
  "jobs": [
    {
      "id": "uuid-here",
      "project_path": "/path/to/recording.cap",
      "output_path": "/path/to/output.mp4",
      "status": "Pending",
      "delete_original": true,
      "settings": {
        "fps": 60,
        "resolution": [1920, 1080],
        "compression": "Maximum"
      },
      "added_at": "2026-01-24T12:00:00Z",
      "completed_at": null,
      "error": null
    }
  ]
}
```

### CLI Structure
```
cap queue add <project_path> [options]
cap queue list [--status <status>]
cap queue run [--dry-run] [--no-caffeinate] [--force]
cap queue remove <job_id>
cap queue clear [--status <status>]
```

### Platform Notes
- `caffeinate` is macOS-specific; skip on other platforms
- Queue file should use platform-appropriate config directory

## 6. Relevant Rules
- CLAUDE.md (no code comments, Rust clippy rules)
- Existing CLI patterns in `apps/cli/src/main.rs`
- Export patterns from `apps/desktop/src-tauri/src/export.rs`
