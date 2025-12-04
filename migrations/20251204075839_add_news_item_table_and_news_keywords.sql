-- Add migration script here
-- 存储具体的新闻条目，每条记录对应一篇新闻
CREATE TABLE IF NOT EXISTS news_item
(
    id            TEXT PRIMARY KEY,                       -- 新闻唯一 ID
    news_info_id  BIGINT REFERENCES news_info(id) ON DELETE CASCADE, -- 所属原始批次
    title         TEXT NOT NULL,                          -- 新闻标题
    url           TEXT NOT NULL,                          -- 新闻链接
    published_at  TIMESTAMP,                              -- 新闻发布时间
    source        VARCHAR(50),                            -- 新闻来源平台
    content       TEXT,                                   -- 新闻正文
    created_at    TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE news_item IS '存储具体的新闻条目，每条记录对应一篇新闻';
COMMENT ON COLUMN news_item.news_info_id IS '关联原始新闻批次 news_info';
COMMENT ON COLUMN news_item.content IS '新闻正文';

-- 存储每条新闻对应的关键词及权重信息
CREATE TABLE IF NOT EXISTS news_keywords
(
    id        BIGSERIAL PRIMARY KEY,
    news_id   TEXT REFERENCES news_item(id) ON DELETE CASCADE, -- 对应新闻条目
    keyword   TEXT NOT NULL,                                   -- 关键词
    weight    REAL,                                            -- TF-IDF 或 TextRank 权重
    method    TEXT NOT NULL,                                   -- 提取方法，例如 textrank / tfidf
    created_at    TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE news_keywords IS '存储每条新闻对应的关键词及权重信息';
COMMENT ON COLUMN news_keywords.news_id IS '关联新闻条目 news_item';
COMMENT ON COLUMN news_keywords.method IS '关键词提取方法，例如 textrank 或 tfidf';


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
        tables    text[] := ARRAY ['news_item', 'news_keywords']; -- 这里列出所有目标表
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