-- Add migration script here
INSERT INTO scheduled_tasks (id, name, cron, params, is_enabled, retry_times, last_status)
VALUES (12, '提取新闻关键字到news_keywords', '27 */30 6-23,0-1 * * * *', '{"arg": "https://news-analytics-gw35.onrender.com/api/analysis/tfidf?limit=500&top_k=5", "cmd": "extract_keywords_to_news_keywords"}', true, 0, '0')
ON CONFLICT (id) DO UPDATE SET
       name = EXCLUDED.name,
       cron = EXCLUDED.cron,
       params = EXCLUDED.params,
       is_enabled = EXCLUDED.is_enabled,
       retry_times = EXCLUDED.retry_times,
       last_status = EXCLUDED.last_status,
       updated_at = NOW();