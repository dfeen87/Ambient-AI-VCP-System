-- Track per-assignment execution state so the system can observe which nodes
-- have started/completed their portion of a task.
--
-- execution_status lifecycle:
--   'assigned'    → node has been allocated to the task but has not started yet
--   'in_progress' → node has acknowledged the task (reported via heartbeat or explicit start)
--   'completed'   → node submitted a successful result via POST /tasks/{id}/result
--   'failed'      → node reported an error or was swept offline while executing

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
