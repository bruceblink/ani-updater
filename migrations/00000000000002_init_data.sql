-- ============================================================================
-- 初始化种子数据
-- 包含：定时任务配置、RBAC 角色/权限/关联、示例用户
-- ============================================================================

-- ============================================================
-- 定时任务（scheduled_tasks）
-- ============================================================
INSERT INTO scheduled_tasks (name, cron, params, is_enabled, retry_times, last_status)
VALUES ('render健康检测',
        '0 */10 * * * * ',
        '{"arg": "https://news-analytics-gw35.onrender.com/health", "cmd": "health_check"}',
        true, 0, '0')
ON CONFLICT (name) DO NOTHING;

INSERT INTO scheduled_tasks (name, cron, params, is_enabled, retry_times, last_status)
VALUES ('提取news_info到news_item',
        '37 */10 6-23,0-1 * * * *',
        '{"arg": "https://news-analytics-gw35.onrender.com/api/analysis/extract_news", "cmd": "extract_transform_news_info_to_item"}',
        true, 0, '0')
ON CONFLICT (name) DO NOTHING;

INSERT INTO scheduled_tasks (name, cron, params, is_enabled, retry_times, last_status)
VALUES ('提取新闻关键字到news_keywords',
        '27 */30 6-23,0-1 * * * *',
        '{"arg": "https://news-analytics-gw35.onrender.com/api/analysis/tfidf?limit=500&top_k=5", "cmd": "extract_keywords_to_news_keywords"}',
        true, 0, '0')
ON CONFLICT (name) DO NOTHING;

INSERT INTO scheduled_tasks (name, cron, params, is_enabled, retry_times, last_status)
VALUES ('提取新闻事件',
        '37 */30 6-23,0-1 * * * *',
        '{"arg": "https://news-analytics-gw35.onrender.com/api/analysis/extract_event", "cmd": "extract_news_event"}',
        true, 0, '0')
ON CONFLICT (name) DO NOTHING;

INSERT INTO scheduled_tasks (name, cron, params, is_enabled, retry_times, last_status)
VALUES ('合并新闻事件',
        '47 0 1 * * * *',
        '{"arg": "https://news-analytics-gw35.onrender.com/api/analysis/merge_event?days=2", "cmd": "merge_cross_day_news_events"}',
        true, 0, '0')
ON CONFLICT (name) DO NOTHING;

-- ============================================================
-- RBAC：角色
-- ============================================================
INSERT INTO roles (name, description)
VALUES ('admin', '系统管理员，拥有全部权限'),
       ('editor', '编辑用户，可以管理内容但不能管理系统配置'),
       ('user', '普通用户，只能访问自身相关资源')
ON CONFLICT (name) DO NOTHING;

-- ============================================================
-- RBAC：权限（JWT Scopes）
-- ============================================================
INSERT INTO permissions (name, description)
VALUES ('user:read', '读取用户信息'),
       ('user:write', '修改用户信息'),
       ('order:read', '读取订单信息'),
       ('order:write', '修改/创建订单'),
       ('admin:all', '系统管理权限，包括用户管理、角色管理等')
ON CONFLICT (name) DO NOTHING;

-- ============================================================
-- RBAC：角色 - 权限 关联
-- ============================================================
-- admin 拥有全部权限
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r,
     permissions p
WHERE r.name = 'admin'
ON CONFLICT DO NOTHING;

-- editor 拥有 user:read, order:read, order:write
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
         JOIN permissions p ON p.name IN ('user:read', 'order:read', 'order:write')
WHERE r.name = 'editor'
ON CONFLICT DO NOTHING;

-- user 拥有 user:read, order:read
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
         JOIN permissions p ON p.name IN ('user:read', 'order:read')
WHERE r.name = 'user'
ON CONFLICT DO NOTHING;

-- ============================================================
-- RBAC：套餐 - 权限 关联
-- ============================================================
-- free: 只允许 user:read
INSERT INTO plan_permissions (plan, permission_id)
SELECT 'free', p.id
FROM permissions p
WHERE p.name IN ('user:read')
ON CONFLICT DO NOTHING;

-- pro: user:read, order:read, order:write
INSERT INTO plan_permissions (plan, permission_id)
SELECT 'pro', p.id
FROM permissions p
WHERE p.name IN ('user:read', 'order:read', 'order:write')
ON CONFLICT DO NOTHING;

-- enterprise: 全部权限
INSERT INTO plan_permissions (plan, permission_id)
SELECT 'enterprise', p.id
FROM permissions p
ON CONFLICT DO NOTHING;

-- ============================================================
-- 初始用户（示例数据，生产环境请替换为真实哈希密码）
-- ============================================================
INSERT INTO user_info (email, username, password, display_name, avatar_url, tenant_id, org_id, plan)
VALUES ('admin@example.com', 'admin', 'hashed_password_admin', '系统管理员',
        'https://i.pravatar.cc/150?img=1', 'tenant_default', 'org_001', 'enterprise'),
       ('editor@example.com', 'editor', 'hashed_password_editor', '内容编辑',
        'https://i.pravatar.cc/150?img=2', 'tenant_default', 'org_001', 'pro'),
       ('user@example.com', 'user', 'hashed_password_user', '普通用户',
        'https://i.pravatar.cc/150?img=3', 'tenant_default', 'org_001', 'free'),
       ('gh_user@example.com', 'gh_user', NULL, 'GitHub 用户',
        'https://i.pravatar.cc/150?img=4', 'tenant_default', 'org_001', 'pro')
ON CONFLICT (email) DO NOTHING;

-- 第三方登录绑定（GitHub 示例）
INSERT INTO user_identities (user_id, provider, provider_uid, access_token, token_expires_at)
VALUES ((SELECT id FROM user_info WHERE username = 'gh_user'),
        'github', '123456', 'gh_access_token_sample', now() + INTERVAL '1 hour')
ON CONFLICT (provider, provider_uid) DO NOTHING;

-- ============================================================
-- 用户 - 角色 分配
-- ============================================================
INSERT INTO user_roles (user_id, role_id, assigned_at)
SELECT u.id, r.id, now()
FROM user_info u
         JOIN roles r ON r.name = 'admin'
WHERE u.username = 'admin'
ON CONFLICT DO NOTHING;

INSERT INTO user_roles (user_id, role_id, assigned_at)
SELECT u.id, r.id, now()
FROM user_info u
         JOIN roles r ON r.name = 'editor'
WHERE u.username = 'editor'
ON CONFLICT DO NOTHING;

INSERT INTO user_roles (user_id, role_id, assigned_at)
SELECT u.id, r.id, now()
FROM user_info u
         JOIN roles r ON r.name = 'user'
WHERE u.username = 'user'
ON CONFLICT DO NOTHING;

INSERT INTO user_roles (user_id, role_id, assigned_at)
SELECT u.id, r.id, now()
FROM user_info u
         JOIN roles r ON r.name = 'editor'
WHERE u.username = 'gh_user'
ON CONFLICT DO NOTHING;
