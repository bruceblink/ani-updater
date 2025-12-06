-- Add migration script here
INSERT INTO scheduled_tasks (id, name, cron, params, is_enabled, retry_times, last_status)
VALUES (11, '提取news_info到news_item', '37 */10 6-23,0-1 * * * *', '{"arg": "", "cmd": "extract_transform_news_info_to_item"}', true, 0, '0')
ON CONFLICT (id) DO UPDATE SET
      name = EXCLUDED.name,
      cron = EXCLUDED.cron,
      params = EXCLUDED.params,
      is_enabled = EXCLUDED.is_enabled,
      retry_times = EXCLUDED.retry_times,
      last_status = EXCLUDED.last_status,
      updated_at = NOW();