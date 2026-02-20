-- Track per-assignment execution state so the system can observe which nodes
-- have started/completed their portion of a task.
--
-- execution_status lifecycle:
--   'assigned'    → node has been allocated to the task but has not confirmed activity yet
--   'in_progress' → node confirmed active via heartbeat (first heartbeat after assignment)
--   'completed'   → assignment ended successfully; this occurs through three paths:
--                   (a) node submitted a result via POST /tasks/{id}/result,
--                   (b) synthetic fallback completion after max_execution_time_sec,
--                   (c) connect_only session lifecycle ended normally
--   'failed'      → node was swept offline, deleted, or rejected while in_progress

ALTER TABLE task_assignments
    ADD COLUMN IF NOT EXISTS execution_status VARCHAR(32) NOT NULL DEFAULT 'assigned';

ALTER TABLE task_assignments
    ADD COLUMN IF NOT EXISTS execution_started_at TIMESTAMP WITH TIME ZONE;

ALTER TABLE task_assignments
    ADD COLUMN IF NOT EXISTS execution_completed_at TIMESTAMP WITH TIME ZONE;

-- Index to quickly find assignments by execution state within a task.
CREATE INDEX IF NOT EXISTS idx_task_assignments_exec_status
    ON task_assignments(task_id, execution_status)
    WHERE disconnected_at IS NULL;
