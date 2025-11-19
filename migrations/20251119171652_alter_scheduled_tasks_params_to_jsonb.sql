-- Add migration script here
-- 1. 执行字段类型修改
ALTER TABLE scheduled_tasks
    ALTER COLUMN params TYPE jsonb USING params::jsonb;
-- 2. 创建索引
CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_params
    ON scheduled_tasks USING gin (params);