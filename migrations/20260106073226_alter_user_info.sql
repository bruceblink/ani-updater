-- Add migration script here
-- -----------------------------
-- 扩展 user_info 表
-- -----------------------------
alter table user_info
    add column if not exists tenant_id varchar(50) not null default 'default';

alter table user_info
    add column if not exists org_id varchar(50);

alter table user_info
    add column if not exists plan varchar(20) default 'free';

comment on column user_info.tenant_id is 'SaaS 租户 ID';
comment on column user_info.org_id is '组织或项目 ID';
comment on column user_info.plan is '用户套餐类型';
