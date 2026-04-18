-- 添加从 latestnews 抓取全量新闻源数据的定时任务
-- 步骤：先调用 /api/s/ids 获取所有 sourceId，再并发抓取每个源
INSERT INTO scheduled_tasks (name, cron, params, is_enabled, retry_times, last_status)
VALUES ('抓取全量新闻数据',
        '0 0 * * * * *',
        '{"arg": "https://news.likanug.top", "cmd": "fetch_all_news"}',
        true, 0, '0')
ON CONFLICT (name) DO NOTHING;
