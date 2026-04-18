-- Add migration script here
-- 仅删除旧表，不重建；news_item 与 news_keywords 的最终结构由
-- 20251212030017_update_news_items_table_and_news_keywords_table.sql 统一重建
DROP TABLE IF EXISTS news_keywords;
