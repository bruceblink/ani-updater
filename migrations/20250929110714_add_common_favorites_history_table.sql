-- Add migration script here
CREATE TABLE favorites   -- 收藏表
(
    id           BIGSERIAL PRIMARY KEY,
    user_id      BIGINT      NOT NULL,
    content_id   BIGINT      NOT NULL,
    content_type VARCHAR(50) NOT NULL,               -- video / article / url / audio
    added_at     TIMESTAMP   NOT NULL DEFAULT NOW(), -- 记录收藏时间
    note         TEXT,                               -- 可选：收藏备注
    tags         TEXT[],                             -- 可选：标签
    created_at   TIMESTAMP   NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMP   NOT NULL DEFAULT NOW(),
    CONSTRAINT uniq_user_favorite UNIQUE (user_id, content_type, content_id)
);

-- UPSERT 示例：重复收藏时更新备注和标签
/*INSERT INTO favorites (user_id, content_id, content_type, note, tags)
VALUES (?, ?, ?, ?, ?)
ON CONFLICT (user_id, content_type, content_id)
    DO UPDATE SET
                  note = EXCLUDED.note,
                  tags = EXCLUDED.tags,
                  updated_at = NOW();*/

CREATE TABLE watch_history  -- 观看历史表
(
    id               BIGSERIAL PRIMARY KEY,
    user_id          BIGINT      NOT NULL,
    content_id       BIGINT      NOT NULL,
    content_type     VARCHAR(50) NOT NULL,
    watched_at       TIMESTAMP   NOT NULL DEFAULT NOW(),
    progress_seconds INT                  DEFAULT 0,     -- 对视频有效
    duration_seconds INT                  DEFAULT 0,     -- 对视频有效
    is_finished      BOOLEAN              DEFAULT FALSE, -- 对视频有效
    extra            JSONB,                              -- 存储类型特定数据
    created_at       TIMESTAMP   NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMP   NOT NULL DEFAULT NOW(),
    CONSTRAINT uniq_user_content UNIQUE (user_id, content_type, content_id)
);

-- UPSERT 示例：更新播放进度或其他历史信息
/*INSERT INTO watch_history (user_id, content_id, content_type, watched_at, progress_seconds, duration_seconds, is_finished, extra)
VALUES (?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT (user_id, content_type, content_id)
    DO UPDATE SET
                  watched_at = EXCLUDED.watched_at,
                  progress_seconds = EXCLUDED.progress_seconds,
                  duration_seconds = EXCLUDED.duration_seconds,
                  is_finished = EXCLUDED.is_finished,
                  extra = EXCLUDED.extra,
                  updated_at = NOW();*/

-- 1. 创建/替换触发器函数（所有表共用）
CREATE OR REPLACE FUNCTION update_updated_at()
    RETURNS TRIGGER AS
$$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 2. 批量处理表名
DO
$$
    DECLARE
        tbl_name  text;
        trig_name text;
        tables    text[] := ARRAY ['favorites', 'watch_history']; -- 这里列出所有目标表
    BEGIN
        FOREACH tbl_name IN ARRAY tables
            LOOP
                trig_name := tbl_name || '_updated_at_trg';

                -- 检查表是否存在
                IF EXISTS (SELECT 1 FROM pg_class WHERE relname = tbl_name AND relkind = 'r') THEN

                    -- 删除已有触发器（如果存在）
                    IF EXISTS (SELECT 1
                               FROM pg_trigger t
                                        JOIN pg_class c ON t.tgrelid = c.oid
                               WHERE t.tgname = trig_name
                                 AND c.relname = tbl_name) THEN
                        EXECUTE format('DROP TRIGGER %I ON %I;', trig_name, tbl_name);
                    END IF;

                    -- 创建新的触发器
                    EXECUTE format(
                            'CREATE TRIGGER %I BEFORE UPDATE ON %I
                             FOR EACH ROW EXECUTE FUNCTION update_updated_at();',
                            trig_name, tbl_name
                            );

                    RAISE NOTICE 'Trigger % created on table %', trig_name, tbl_name;

                ELSE
                    RAISE NOTICE 'Table % does not exist, trigger not created.', tbl_name;
                END IF;
            END LOOP;
    END
$$;