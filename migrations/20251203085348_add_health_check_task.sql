-- Add migration script here
INSERT INTO scheduled_tasks (id, name, cron, params, is_enabled, retry_times, last_status) VALUES (10,'render健康检测', '0 */10 * * * * ', '{"arg": "https://news-analytics-gw35.onrender.com/health", "cmd": "health_check"}', true, 0,  '0');
