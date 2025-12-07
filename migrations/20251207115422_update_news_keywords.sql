-- Add migration script here
DROP TABLE IF EXISTS news_keywords;
CREATE TABLE IF NOT EXISTS news_keywords (
   id BIGSERIAL PRIMARY KEY,
   news_id TEXT NOT NULL
       REFERENCES news_item(id)
           ON DELETE CASCADE,
   keyword TEXT NOT NULL,
   weight REAL,
   method TEXT NOT NULL,
   created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
   updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
   CONSTRAINT uq_news_keywords UNIQUE(news_id, keyword, method)
);

COMMENT ON TABLE news_keywords IS '存储每条新闻对应的关键词及权重信息';
COMMENT ON COLUMN news_keywords.news_id IS '关联新闻条目 news_item';
COMMENT ON COLUMN news_keywords.keyword IS '关键词';
COMMENT ON COLUMN news_keywords.weight IS 'TF-IDF 或 TextRank 权重';
COMMENT ON COLUMN news_keywords.method IS '关键词提取方法，例如 textrank 或 tfidf';
