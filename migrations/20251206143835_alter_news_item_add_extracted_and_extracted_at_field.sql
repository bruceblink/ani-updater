-- Add migration script here
ALTER TABLE news_item
    ADD COLUMN IF NOT EXISTS extracted boolean DEFAULT false,
    ADD COLUMN IF NOT EXISTS extracted_at TIMESTAMPTZ;

COMMENT ON COLUMN news_item.extracted IS '标记新闻批次是否已抽取到 news_keywords 表';
COMMENT ON COLUMN news_item.extracted_at IS '新闻批次抽取完成时间';