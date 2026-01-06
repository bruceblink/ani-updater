-- Add migration script here
-- -----------------------------
-- 角色表
-- -----------------------------
create table if not exists roles
(
    id          bigserial primary key,
    name        varchar(50) not null unique,
    description text,
    created_at  timestamp with time zone default now() not null,
    updated_at  timestamp with time zone default now() not null
);

comment on table roles is '系统角色表';
comment on column roles.name is '角色名，例如 admin/editor';
comment on column roles.description is '角色描述';

-- -----------------------------
-- 用户角色关联表
-- -----------------------------
create table if not exists user_roles
(
    user_id bigint not null references user_info(id) on delete cascade,
    role_id bigint not null references roles(id) on delete cascade,
    assigned_at timestamp with time zone default now() not null,
    created_at  timestamp with time zone default now() not null,
    updated_at  timestamp with time zone default now() not null,
    primary key (user_id, role_id)
);

comment on table user_roles is '用户与角色的关联表';

-- -----------------------------
-- 权限表（Scope）
-- -----------------------------
create table if not exists permissions
(
    id          bigserial primary key,
    name        varchar(100) not null unique,
    description text,
    created_at  timestamp with time zone default now() not null,
    updated_at  timestamp with time zone default now() not null
);

comment on table permissions is '系统权限表，对应 JWT scopes';
comment on column permissions.name is '权限名，例如 user:read/order:write';

-- -----------------------------
-- 角色权限关联表
-- -----------------------------
create table if not exists role_permissions
(
    role_id       bigint not null references roles(id) on delete cascade,
    permission_id bigint not null references permissions(id) on delete cascade,
    created_at  timestamp with time zone default now() not null,
    updated_at  timestamp with time zone default now() not null,
    primary key(role_id, permission_id)
);

comment on table role_permissions is '角色与权限的关联表';

-- -----------------------------
-- 套餐权限表
-- -----------------------------
create table if not exists plan_permissions
(
    plan           varchar(20) not null,     -- free/pro/enterprise
    permission_id  bigint not null references permissions(id) on delete cascade,
    created_at  timestamp with time zone default now() not null,
    updated_at  timestamp with time zone default now() not null,
    primary key(plan, permission_id)
);

comment on table plan_permissions is 'SaaS 套餐权限表，控制不同套餐可用 scope';


-- 创建/替换触发器函数（所有表共用）
CREATE OR REPLACE FUNCTION update_updated_at()
    RETURNS TRIGGER AS
$$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 2. 批量处理表名
DO
$$
    DECLARE
        tbl_name  text;
        trig_name text;
        tables    text[] := ARRAY ['roles', 'user_roles', 'permissions', 'role_permissions', 'plan_permissions']; -- 这里列出所有目标表
    BEGIN
        FOREACH tbl_name IN ARRAY tables
            LOOP
                trig_name := tbl_name || '_updated_at_trg';

                -- 检查表是否存在
                IF EXISTS (SELECT 1 FROM pg_class WHERE relname = tbl_name AND relkind = 'r') THEN

                    -- 删除已有触发器（如果存在）
                    IF EXISTS (SELECT 1
                               FROM pg_trigger t
                                        JOIN pg_class c ON t.tgrelid = c.oid
                               WHERE t.tgname = trig_name
                                 AND c.relname = tbl_name) THEN
                        EXECUTE format('DROP TRIGGER %I ON %I;', trig_name, tbl_name);
                    END IF;

                    -- 创建新的触发器
                    EXECUTE format(
                            'CREATE TRIGGER %I BEFORE UPDATE ON %I
                             FOR EACH ROW EXECUTE FUNCTION update_updated_at();',
                            trig_name, tbl_name
                            );

                    RAISE NOTICE 'Trigger % created on table %', trig_name, tbl_name;

                ELSE
                    RAISE NOTICE 'Table % does not exist, trigger not created.', tbl_name;
                END IF;
            END LOOP;
    END
$$;