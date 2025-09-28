-- Add migration script here
-- 给 ani_info 表添加 created_at 和 updated_at 字段
ALTER TABLE ani_info
    ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP;

