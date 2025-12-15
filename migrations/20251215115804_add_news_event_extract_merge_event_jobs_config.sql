-- Add migration script here
INSERT INTO scheduled_tasks (id, name, cron, params, is_enabled, retry_times, last_status)
VALUES (13, '提取新闻事件', '37 */30 6-23,0-1 * * * *', '{"arg": "https://news-analytics-gw35.onrender.com/api/analysis/extract_event", "cmd": "extract_news_event"}', true, 0, '0')
ON CONFLICT (id) DO UPDATE SET
                               name = EXCLUDED.name,
                               cron = EXCLUDED.cron,
                               params = EXCLUDED.params,
                               is_enabled = EXCLUDED.is_enabled,
                               retry_times = EXCLUDED.retry_times,
                               last_status = EXCLUDED.last_status,
                               updated_at = NOW();

INSERT INTO scheduled_tasks (id, name, cron, params, is_enabled, retry_times, last_status)
VALUES (14, '合并新闻事件', '47 0 1 * * * *', '{"arg": "https://news-analytics-gw35.onrender.com/api/analysis/merge_event?days=2", "cmd": "merge_cross_day_news_events"}', true, 0, '0')
ON CONFLICT (id) DO UPDATE SET
                               name = EXCLUDED.name,
                               cron = EXCLUDED.cron,
                               params = EXCLUDED.params,
                               is_enabled = EXCLUDED.is_enabled,
                               retry_times = EXCLUDED.retry_times,
                               last_status = EXCLUDED.last_status,
                               updated_at = NOW();