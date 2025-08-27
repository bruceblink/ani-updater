-- 1. 创建表
--------------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS ani_info
(
    id           BIGSERIAL PRIMARY KEY,
    title        TEXT        NOT NULL,
    update_count TEXT,
    update_info  TEXT,
    image_url    TEXT        NOT NULL,
    detail_url   TEXT        NOT NULL,
    update_time  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP, -- 仅插入时写入
    platform     TEXT        NOT NULL,
    CONSTRAINT uniq_ani_info UNIQUE (title, platform, update_count)
);

CREATE TABLE IF NOT EXISTS ani_collect
(
    id           BIGSERIAL PRIMARY KEY,
    user_id      TEXT                 DEFAULT '',
    ani_item_id  BIGINT     NOT NULL,
    ani_title    TEXT        NOT NULL,
    collect_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP, -- 收藏时的时间
    is_watched   BOOLEAN     NOT NULL DEFAULT FALSE,             -- 是否已观看
    CONSTRAINT uniq_ani_collect UNIQUE (user_id, ani_item_id),
    CONSTRAINT fk_ani_item FOREIGN KEY (ani_item_id)
        REFERENCES ani_info (id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ani_watch_history
(
    id           BIGSERIAL PRIMARY KEY,
    user_id      TEXT                 DEFAULT '',
    ani_item_id  BIGINT     NOT NULL,
    watched_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP, -- 观看时的时间
    CONSTRAINT uniq_ani_watch UNIQUE (user_id, ani_item_id),
    CONSTRAINT fk_ani_watch FOREIGN KEY (ani_item_id)
        REFERENCES ani_info (id)
        ON DELETE CASCADE
);

--------------------------------------------------------------------------------
-- 1.1 字段备注
--------------------------------------------------------------------------------

-- ani_info 表
COMMENT ON TABLE ani_info IS '番剧信息表';
COMMENT ON COLUMN ani_info.id IS '主键 ID';
COMMENT ON COLUMN ani_info.title IS '番剧标题';
COMMENT ON COLUMN ani_info.update_count IS '更新集数（如 第10集）';
COMMENT ON COLUMN ani_info.update_info IS '更新描述（如 已完结）';
COMMENT ON COLUMN ani_info.image_url IS '封面图片 URL';
COMMENT ON COLUMN ani_info.detail_url IS '详情页 URL';
COMMENT ON COLUMN ani_info.update_time IS '信息更新时间（插入时自动写入）';
COMMENT ON COLUMN ani_info.platform IS '所属平台（如 bilibili、iqiyi 等）';

-- ani_collect 表
COMMENT ON TABLE ani_collect IS '用户收藏表';
COMMENT ON COLUMN ani_collect.id IS '主键 ID';
COMMENT ON COLUMN ani_collect.user_id IS '用户 ID';
COMMENT ON COLUMN ani_collect.ani_item_id IS '关联番剧 ID';
COMMENT ON COLUMN ani_collect.ani_title IS '收藏时的番剧标题';
COMMENT ON COLUMN ani_collect.collect_time IS '收藏时间';
COMMENT ON COLUMN ani_collect.is_watched IS '是否已观看';

-- ani_watch_history 表
COMMENT ON TABLE ani_watch_history IS '观看历史表';
COMMENT ON COLUMN ani_watch_history.id IS '主键 ID';
COMMENT ON COLUMN ani_watch_history.user_id IS '用户 ID';
COMMENT ON COLUMN ani_watch_history.ani_item_id IS '关联番剧 ID';
COMMENT ON COLUMN ani_watch_history.watched_time IS '观看时间';


--------------------------------------------------------------------------------
-- 2. 创建索引
--------------------------------------------------------------------------------

CREATE INDEX IF NOT EXISTS idx_ani_info_update_time
    ON ani_info (update_time);

CREATE INDEX IF NOT EXISTS idx_ani_collect_item_time
    ON ani_collect (ani_item_id, collect_time);

CREATE INDEX IF NOT EXISTS idx_ani_collect_title
    ON ani_collect (ani_title);

CREATE INDEX IF NOT EXISTS idx_ani_watch_history_item_time
    ON ani_watch_history (ani_item_id, watched_time);

CREATE INDEX IF NOT EXISTS idx_ani_watch_history_time
    ON ani_watch_history (watched_time);


--------------------------------------------------------------------------------
-- 3. 创建触发器（更新 is_watched 状态）
--------------------------------------------------------------------------------

-- 先删除旧触发器和函数，避免重复执行报错
DO
$$
    BEGIN
        IF EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'trg_after_insert_watch') THEN
            DROP TRIGGER trg_after_insert_watch ON ani_watch_history;
        END IF;

        IF EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'trg_after_insert_watch_func') THEN
            DROP FUNCTION trg_after_insert_watch_func() CASCADE;
        END IF;
    END;
$$;

-- 创建触发器函数
CREATE OR REPLACE FUNCTION trg_after_insert_watch_func()
    RETURNS TRIGGER AS
$$
BEGIN
    UPDATE ani_collect
    SET is_watched = TRUE
    WHERE user_id = NEW.user_id
      AND ani_item_id = NEW.ani_item_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 创建触发器
CREATE TRIGGER trg_after_insert_watch
    AFTER INSERT
    ON ani_watch_history
    FOR EACH ROW
EXECUTE FUNCTION trg_after_insert_watch_func();
