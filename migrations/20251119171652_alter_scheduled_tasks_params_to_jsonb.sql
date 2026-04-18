-- Add migration script here
-- 1. 执行字段类型修改（仅当字段类型不是 jsonb 时执行）
DO $$
BEGIN
    IF (SELECT data_type FROM information_schema.columns
        WHERE table_name = 'scheduled_tasks' AND column_name = 'params') <> 'jsonb' THEN
        ALTER TABLE scheduled_tasks
            ALTER COLUMN params TYPE jsonb USING params::jsonb;
    END IF;
END $$;
-- 2. 创建索引
CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_params
    ON scheduled_tasks USING gin (params);