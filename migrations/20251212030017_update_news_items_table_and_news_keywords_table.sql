-- Add migration script here
-- 删除表
DROP TABLE IF EXISTS news_keywords;
DROP TABLE IF EXISTS news_item;
-- 更新 news_info 中数据的提取状态
UPDATE newsletter.public.news_info set extracted=false;


-- 重新创建表
-- pgvector 扩展（本地和生产都要执行一次）
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS news_item
(
    id       BIGSERIAL PRIMARY KEY,              -- 真实唯一主键
    item_id            TEXT NOT NULL,                      -- 新闻来源ID
    news_info_id  BIGINT REFERENCES news_info ON DELETE CASCADE,

    title         TEXT NOT NULL,
    url           TEXT NOT NULL,
    published_at  DATE NOT NULL,                      -- 你需要用于唯一约束的字段
    source        VARCHAR(50),

    content       TEXT,                                -- 新闻正文（大多为空，后续可扩展）

    -- NLP / 分析扩展字段（可以留空）
    embedding     VECTOR(768),                         -- 语义向量
    cluster_id    BIGINT,                              -- 聚类事件ID

    created_at    TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    extracted     BOOLEAN DEFAULT FALSE,
    extracted_at  TIMESTAMPTZ,

    -- 关键：同一个新闻 ID 在同一天只能出现一次
    CONSTRAINT uq_news_date UNIQUE (item_id, published_at)
);

COMMENT ON TABLE news_item IS '存储新闻条目；item_id 为内部主键，(id, published_at) 为业务唯一约束';
COMMENT ON COLUMN news_item.id IS '数据库内部主键，供外键引用';
COMMENT ON COLUMN news_item.item_id IS '新闻原始ID（例如URL中的数字，可能跨天重复）';
COMMENT ON COLUMN news_item.embedding IS '新闻标题或正文的语义向量（如 BGE / m3e）';
COMMENT ON COLUMN news_item.cluster_id IS '语义聚类后的事件ID';



CREATE TABLE IF NOT EXISTS news_keywords
(
    id         BIGSERIAL PRIMARY KEY,
    news_id    BIGINT NOT NULL REFERENCES news_item(id) ON DELETE CASCADE,
    keyword    TEXT NOT NULL,
    weight     REAL,
    method     TEXT NOT NULL,   -- tfidf / textrank / embedding / hybrid

    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT uq_news_keywords UNIQUE (news_id, keyword, method)
);

COMMENT ON TABLE news_keywords IS '保存新闻关键词，用 news_item.item_id 关联';
COMMENT ON COLUMN news_keywords.method IS 'tfidf/textrank/embedding等关键词提取方法';
