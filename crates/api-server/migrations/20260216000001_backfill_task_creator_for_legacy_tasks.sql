-- Backfill creator ownership for legacy tasks created before creator_id existed.
-- This preserves visibility after ownership-scoped task filtering was introduced.

UPDATE tasks
SET creator_id = (
    SELECT user_id
    FROM users
    WHERE role = 'admin'
    ORDER BY created_at ASC
    LIMIT 1
)
WHERE creator_id IS NULL
  AND EXISTS (
      SELECT 1
      FROM users
      WHERE role = 'admin'
  );
