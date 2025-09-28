-- Add migration script here
--- 给 scheduled_tasks 添加唯一约束
DO $$
    BEGIN
        IF NOT EXISTS (
            SELECT 1
            FROM pg_constraint
            WHERE conname = 'uq_scheduled_tasks_name'
        ) THEN
            ALTER TABLE scheduled_tasks
                ADD CONSTRAINT uq_scheduled_tasks_name UNIQUE (name);
        END IF;
    END$$;
