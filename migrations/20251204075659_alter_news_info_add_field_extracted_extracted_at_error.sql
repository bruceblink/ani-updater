-- Add migration script here
ALTER TABLE news_info
    ADD COLUMN IF NOT EXISTS extracted boolean DEFAULT false,
    ADD COLUMN IF NOT EXISTS extracted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS error text;

COMMENT ON TABLE news_info IS '存储每次抓取的原始新闻数据，每条记录对应一个来源和日期的新闻批次';
COMMENT ON COLUMN news_info.data IS '原始抓取 JSON 数据';
COMMENT ON COLUMN news_info.extracted IS '标记新闻批次是否已抽取到 news_item 表';
COMMENT ON COLUMN news_info.extracted_at IS '新闻批次抽取完成时间';
COMMENT ON COLUMN news_info.error IS '抽取新闻失败时记录的错误信息';