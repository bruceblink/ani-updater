-- Add migration script here
BEGIN;
ALTER TABLE news_info ADD COLUMN name VARCHAR(50);  --新闻平台的中文名称
ALTER TABLE news_info DROP CONSTRAINT uniq_news_info;
ALTER TABLE news_info ADD CONSTRAINT uniq_news_info UNIQUE (news_from, name, news_date);
COMMIT;
