-- Add migration script here
-- ------------------------
-- 创建定时任务表
-- ------------------------
CREATE TABLE IF NOT EXISTS scheduled_tasks
(
    id          BIGSERIAL PRIMARY KEY,              -- 任务唯一 ID
    name        VARCHAR(128) NOT NULL,              -- 任务名称
    cron        VARCHAR(256) NOT NULL,              -- cron 表达式
    params      JSON,
    is_enabled  BOOLEAN               DEFAULT TRUE, -- 是否启用
    retry_times SMALLINT              DEFAULT 0,    -- 重试次数
    last_run    TIMESTAMP,                          -- 上次运行时间
    next_run    TIMESTAMP,                          -- 下次运行时间（可选，调度器算）
    last_status VARCHAR(16),                        -- 上次运行结果（SUCCESS/FAILED）

    created_at  TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP
);
