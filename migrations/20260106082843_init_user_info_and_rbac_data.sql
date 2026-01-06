-- Add migration script here
-- ======================================
-- 1️⃣ 初始化角色
-- ======================================
insert into roles (name, description, created_at, updated_at)
values ('admin', '系统管理员，拥有全部权限', now(), now()),
       ('editor', '编辑用户，可以管理内容但不能管理系统配置', now(), now()),
       ('user', '普通用户，只能访问自身相关资源', now(), now());

-- ======================================
-- 2️⃣ 初始化权限 (scopes)
-- ======================================
insert into permissions (name, description, created_at, updated_at)
values ('user:read', '读取用户信息', now(), now()),
       ('user:write', '修改用户信息', now(), now()),
       ('order:read', '读取订单信息', now(), now()),
       ('order:write', '修改/创建订单', now(), now()),
       ('admin:all', '系统管理权限，包括用户管理、角色管理等', now(), now());

-- ======================================
-- 3️⃣ 初始化角色-权限关联 (role_permissions)
-- ======================================
-- admin 拥有全部权限
insert into role_permissions (role_id, permission_id)
select r.id, p.id
from roles r,
     permissions p
where r.name = 'admin';

-- editor 拥有 user:read, order:read, order:write
insert into role_permissions (role_id, permission_id)
select r.id, p.id
from roles r
         join permissions p on p.name in ('user:read', 'order:read', 'order:write')
where r.name = 'editor';

-- user 拥有 user:read, order:read
insert into role_permissions (role_id, permission_id)
select r.id, p.id
from roles r
         join permissions p on p.name in ('user:read', 'order:read')
where r.name = 'user';

-- ======================================
-- 4️⃣ 初始化套餐-权限关联 (plan_permissions)
-- ======================================
-- free: 只允许 user:read
insert into plan_permissions (plan, permission_id)
select 'free', p.id
from permissions p
where p.name in ('user:read');

-- pro: user:read, order:read, order:write
insert into plan_permissions (plan, permission_id)
select 'pro', p.id
from permissions p
where p.name in ('user:read', 'order:read', 'order:write');

-- enterprise: 全部权限
insert into plan_permissions (plan, permission_id)
select 'enterprise', p.id
from permissions p;

-- ======================================
-- 5️⃣ 初始化用户 (user_info)
-- ======================================
insert into user_info (email, username, password, display_name, avatar_url, tenant_id, org_id, plan, created_at,
                       updated_at)
values ('admin@example.com', 'admin', 'hashed_password_admin', '系统管理员', 'https://i.pravatar.cc/150?img=1',
        'tenant_default', 'org_001', 'enterprise', now(), now()),
       ('editor@example.com', 'editor', 'hashed_password_editor', '内容编辑', 'https://i.pravatar.cc/150?img=2',
        'tenant_default', 'org_001', 'pro', now(), now()),
       ('user@example.com', 'user', 'hashed_password_user', '普通用户', 'https://i.pravatar.cc/150?img=3',
        'tenant_default', 'org_001', 'free', now(), now()),
       ('gh_user@example.com', 'gh_user', NULL, 'GitHub 用户', 'https://i.pravatar.cc/150?img=4', 'tenant_default',
        'org_001', 'pro', now(), now());

-- ======================================
-- 6️⃣ 初始化第三方登录绑定 (user_identities)
-- ======================================
-- GitHub 用户 id 为 '123456'
insert into user_identities (user_id, provider, provider_uid, access_token , token_expires_at, created_at, updated_at)
values ((select id from user_info where username = 'gh_user'), 'github', '123456', 'gh_access_token_sample',
        now() + interval '1 hour', now(), now());

-- ======================================
-- 7️⃣ 用户角色分配 (user_roles)
-- ======================================
-- admin -> admin
insert into user_roles (user_id, role_id, assigned_at)
select u.id, r.id, now()
from user_info u
         join roles r on r.name = 'admin'
where u.username = 'admin';

-- editor -> editor
insert into user_roles (user_id, role_id, assigned_at)
select u.id, r.id, now()
from user_info u
         join roles r on r.name = 'editor'
where u.username = 'editor';

-- user -> user
insert into user_roles (user_id, role_id, assigned_at)
select u.id, r.id, now()
from user_info u
         join roles r on r.name = 'user'
where u.username = 'user';

-- GitHub 用户 -> editor
insert into user_roles (user_id, role_id, assigned_at)
select u.id, r.id, now()
from user_info u
         join roles r on r.name = 'editor'
where u.username = 'gh_user';
