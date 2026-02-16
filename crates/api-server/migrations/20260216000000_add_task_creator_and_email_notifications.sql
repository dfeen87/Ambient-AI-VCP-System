-- Add user email contact and task ownership/notification tracking

ALTER TABLE users
ADD COLUMN IF NOT EXISTS email VARCHAR(255);

ALTER TABLE tasks
ADD COLUMN IF NOT EXISTS creator_id UUID REFERENCES users(user_id) ON DELETE SET NULL,
ADD COLUMN IF NOT EXISTS completion_email_sent_at TIMESTAMP WITH TIME ZONE;

CREATE INDEX IF NOT EXISTS idx_tasks_creator_id_created_at
ON tasks(creator_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_tasks_completion_email_pending
ON tasks(creator_id, status)
WHERE status = 'completed' AND completion_email_sent_at IS NULL;
