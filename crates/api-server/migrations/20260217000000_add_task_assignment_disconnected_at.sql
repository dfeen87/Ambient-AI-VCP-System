-- Preserve task assignment history while allowing assignments to be marked disconnected.
ALTER TABLE task_assignments
ADD COLUMN IF NOT EXISTS disconnected_at TIMESTAMP WITH TIME ZONE;

-- Fast lookup for active (still connected) task assignments.
CREATE INDEX IF NOT EXISTS idx_task_assignments_active
ON task_assignments(task_id, node_id)
WHERE disconnected_at IS NULL;
