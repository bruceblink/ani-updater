-- ============================================================================
-- 全量初始化 Schema
-- 由原 35 个迁移文件合并精简而来，代表数据库最终状态
-- 适用于全新部署（需先清空 _sqlx_migrations 表）
-- ============================================================================

-- ============================================================
-- Extensions
-- ============================================================
CREATE EXTENSION IF NOT EXISTS vector;

-- ============================================================
-- 公共触发器函数：自动更新 updated_at
-- ============================================================
CREATE OR REPLACE FUNCTION update_updated_at()
    RETURNS TRIGGER AS
$$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================
-- Table: ani_info（番剧信息）
-- ============================================================
CREATE TABLE IF NOT EXISTS ani_info
(
    id           BIGSERIAL PRIMARY KEY,
    title        TEXT        NOT NULL,
    update_count TEXT,
    update_info  TEXT,
    image_url    TEXT        NOT NULL,
    detail_url   TEXT        NOT NULL,
    update_time  TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    platform     TEXT        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uniq_ani_info UNIQUE (title, platform, update_count)
);

COMMENT ON TABLE ani_info IS '番剧信息表';
COMMENT ON COLUMN ani_info.title IS '番剧标题';
COMMENT ON COLUMN ani_info.update_count IS '更新集数（如 第10集）';
COMMENT ON COLUMN ani_info.update_info IS '更新描述（如 已完结）';
COMMENT ON COLUMN ani_info.image_url IS '封面图片 URL';
COMMENT ON COLUMN ani_info.detail_url IS '详情页 URL';
COMMENT ON COLUMN ani_info.update_time IS '信息更新时间';
COMMENT ON COLUMN ani_info.platform IS '所属平台（如 bilibili、iqiyi 等）';

CREATE INDEX IF NOT EXISTS idx_ani_info_update_time ON ani_info (update_time);

-- ============================================================
-- Table: ani_collect（用户番剧收藏）
-- ============================================================
CREATE TABLE IF NOT EXISTS ani_collect
(
    id           BIGSERIAL PRIMARY KEY,
    user_id      TEXT        NOT NULL DEFAULT '',
    ani_item_id  BIGINT      NOT NULL,
    ani_title    TEXT        NOT NULL,
    collect_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_watched   BOOLEAN     NOT NULL DEFAULT FALSE,
    CONSTRAINT uniq_ani_collect UNIQUE (user_id, ani_item_id),
    CONSTRAINT fk_ani_item FOREIGN KEY (ani_item_id)
        REFERENCES ani_info (id) ON DELETE CASCADE
);

COMMENT ON TABLE ani_collect IS '用户番剧收藏表';
COMMENT ON COLUMN ani_collect.user_id IS '用户 ID';
COMMENT ON COLUMN ani_collect.ani_item_id IS '关联番剧 ID';
COMMENT ON COLUMN ani_collect.ani_title IS '收藏时的番剧标题';
COMMENT ON COLUMN ani_collect.is_watched IS '是否已观看';

CREATE INDEX IF NOT EXISTS idx_ani_collect_item_time ON ani_collect (ani_item_id, collect_time);
CREATE INDEX IF NOT EXISTS idx_ani_collect_title ON ani_collect (ani_title);

-- ============================================================
-- Table: ani_watch_history（番剧观看历史）
-- ============================================================
CREATE TABLE IF NOT EXISTS ani_watch_history
(
    id           BIGSERIAL PRIMARY KEY,
    user_id      TEXT        NOT NULL DEFAULT '',
    ani_item_id  BIGINT      NOT NULL,
    watched_time TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uniq_ani_watch UNIQUE (user_id, ani_item_id),
    CONSTRAINT fk_ani_watch FOREIGN KEY (ani_item_id)
        REFERENCES ani_info (id) ON DELETE CASCADE
);

COMMENT ON TABLE ani_watch_history IS '番剧观看历史表';
COMMENT ON COLUMN ani_watch_history.user_id IS '用户 ID';
COMMENT ON COLUMN ani_watch_history.ani_item_id IS '关联番剧 ID';
COMMENT ON COLUMN ani_watch_history.watched_time IS '观看时间';

CREATE INDEX IF NOT EXISTS idx_ani_watch_history_item_time ON ani_watch_history (ani_item_id, watched_time);
CREATE INDEX IF NOT EXISTS idx_ani_watch_history_time ON ani_watch_history (watched_time);

-- ============================================================
-- Table: user_info（系统用户，包含 SaaS 和安全字段）
-- ============================================================
CREATE TABLE IF NOT EXISTS user_info
(
    id                    BIGSERIAL PRIMARY KEY,
    email                 VARCHAR(255) UNIQUE,
    username              VARCHAR(100) UNIQUE,
    password              TEXT,
    display_name          VARCHAR(255),
    avatar_url            TEXT,
    tenant_id             VARCHAR(50)  NOT NULL DEFAULT 'default',
    org_id                VARCHAR(50),
    plan                  VARCHAR(20)           DEFAULT 'free',
    token_version         BIGINT       NOT NULL DEFAULT 0,
    status                VARCHAR(50)  NOT NULL DEFAULT 'active',
    locked_until          TIMESTAMPTZ,
    failed_login_attempts INT                   DEFAULT 0,
    created_at            TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ  NOT NULL DEFAULT now()
);

COMMENT ON TABLE user_info IS '系统用户表，存储应用内的用户主体信息';
COMMENT ON COLUMN user_info.tenant_id IS 'SaaS 租户 ID';
COMMENT ON COLUMN user_info.org_id IS '组织或项目 ID';
COMMENT ON COLUMN user_info.plan IS '用户套餐类型';
COMMENT ON COLUMN user_info.token_version IS '用户 JWT 版本，版本变更时旧 JWT 自动失效';
COMMENT ON COLUMN user_info.status IS '账户状态：active、inactive、suspended、frozen 等';
COMMENT ON COLUMN user_info.locked_until IS '账户锁定到期时间，NULL 表示未锁定';
COMMENT ON COLUMN user_info.failed_login_attempts IS '登录失败次数';

CREATE INDEX IF NOT EXISTS idx_user_info_status ON user_info (status);

-- ============================================================
-- Table: user_identities（第三方登录绑定）
-- ============================================================
CREATE TABLE IF NOT EXISTS user_identities
(
    id               BIGSERIAL PRIMARY KEY,
    user_id          BIGINT       NOT NULL REFERENCES user_info (id) ON DELETE CASCADE,
    provider         VARCHAR(50)  NOT NULL,
    provider_uid     VARCHAR(255) NOT NULL,
    access_token     TEXT,
    token_expires_at TIMESTAMPTZ,
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ  NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_uid)
);

COMMENT ON TABLE user_identities IS '用户身份表，存储第三方登录账号与系统用户的映射关系';
COMMENT ON COLUMN user_identities.provider IS '第三方登录提供商，例如 github、google';
COMMENT ON COLUMN user_identities.provider_uid IS '第三方平台的用户唯一 ID';

-- ============================================================
-- Table: refresh_tokens（刷新 Token，支持滑动窗口会话）
-- ============================================================
CREATE TABLE IF NOT EXISTS refresh_tokens
(
    id                 BIGSERIAL PRIMARY KEY,
    user_id            BIGINT      NOT NULL CONSTRAINT fk_refresh_tokens_user_info
        REFERENCES user_info (id),
    token              TEXT        NOT NULL,
    expires_at         TIMESTAMPTZ NOT NULL,
    session_expires_at TIMESTAMPTZ NOT NULL,
    revoked            BOOLEAN     NOT NULL DEFAULT false,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON COLUMN refresh_tokens.user_id IS '关联用户表 user_info 的 ID';
COMMENT ON COLUMN refresh_tokens.token IS 'refresh_token 字符串，用于刷新 access_token';
COMMENT ON COLUMN refresh_tokens.expires_at IS '当前 refresh_token 到期时间（滑动窗口）';
COMMENT ON COLUMN refresh_tokens.session_expires_at IS '会话最晚到期时间，用于强制重新登录';
COMMENT ON COLUMN refresh_tokens.revoked IS '标记 token 是否被撤销';

CREATE UNIQUE INDEX IF NOT EXISTS idx_refresh_tokens_user_token ON refresh_tokens (user_id, token);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_valid ON refresh_tokens (expires_at, revoked);

-- ============================================================
-- Table: scheduled_tasks（定时任务）
-- ============================================================
CREATE TABLE IF NOT EXISTS scheduled_tasks
(
    id          BIGSERIAL PRIMARY KEY,
    name        VARCHAR(128) NOT NULL,
    cron        VARCHAR(256) NOT NULL,
    params      JSONB,
    is_enabled  BOOLEAN               DEFAULT TRUE,
    retry_times SMALLINT              DEFAULT 0,
    last_run    TIMESTAMP,
    next_run    TIMESTAMP,
    last_status VARCHAR(16),
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_scheduled_tasks_name UNIQUE (name)
);

-- ============================================================
-- Table: favorites（通用收藏）
-- ============================================================
CREATE TABLE IF NOT EXISTS favorites
(
    id           BIGSERIAL PRIMARY KEY,
    user_id      BIGINT      NOT NULL,
    content_type VARCHAR(50) NOT NULL,
    added_at     TIMESTAMP   NOT NULL DEFAULT NOW(),
    note         TEXT,
    tags         TEXT[],
    created_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uniq_user_favorite UNIQUE (user_id, content_type)
);

-- ============================================================
-- Table: watch_history（通用观看历史）
-- ============================================================
CREATE TABLE IF NOT EXISTS watch_history
(
    id               BIGSERIAL PRIMARY KEY,
    user_id          BIGINT      NOT NULL,
    content_id       BIGINT      NOT NULL,
    content_type     VARCHAR(50) NOT NULL,
    watched_at       TIMESTAMP   NOT NULL DEFAULT NOW(),
    progress_seconds INT                  DEFAULT 0,
    duration_seconds INT                  DEFAULT 0,
    is_finished      BOOLEAN              DEFAULT FALSE,
    extra            JSONB,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uniq_user_content UNIQUE (user_id, content_type, content_id)
);

-- ============================================================
-- Table: user_setting（用户设置）
-- ============================================================
CREATE TABLE IF NOT EXISTS user_setting
(
    id           BIGSERIAL PRIMARY KEY,
    user_id      BIGINT      NOT NULL,
    setting_type VARCHAR(50) NOT NULL,
    data         JSONB,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uniq_user_setting UNIQUE (user_id, setting_type)
);

-- ============================================================
-- Table: news_info（新闻批次，每条 = 一个来源×一天的原始抓取）
-- ============================================================
CREATE TABLE IF NOT EXISTS news_info
(
    id           BIGSERIAL PRIMARY KEY,
    news_from    VARCHAR(50) NOT NULL,
    news_date    DATE        NOT NULL DEFAULT CURRENT_DATE,
    data         JSONB,
    name         VARCHAR(50),
    extracted    BOOLEAN              DEFAULT false,
    extracted_at TIMESTAMPTZ,
    error        TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uniq_news_info UNIQUE (news_from, news_date)
);

COMMENT ON TABLE news_info IS '存储每次抓取的原始新闻数据，每条记录对应一个来源和日期的新闻批次';
COMMENT ON COLUMN news_info.data IS '原始抓取 JSON 数据';
COMMENT ON COLUMN news_info.extracted IS '标记新闻批次是否已抽取到 news_item 表';
COMMENT ON COLUMN news_info.extracted_at IS '新闻批次抽取完成时间';
COMMENT ON COLUMN news_info.error IS '抽取失败时记录的错误信息';

-- ============================================================
-- Table: news_item（新闻条目）
-- ============================================================
CREATE TABLE IF NOT EXISTS news_item
(
    id             BIGSERIAL PRIMARY KEY,
    item_id        TEXT   NOT NULL,
    news_info_id   BIGINT REFERENCES news_info (id) ON DELETE CASCADE,
    title          TEXT   NOT NULL,
    url            TEXT   NOT NULL,
    published_at   DATE   NOT NULL,
    source         VARCHAR(50),
    content        TEXT,
    cluster_method TEXT,
    cluster_id     BIGINT,
    extracted      BOOLEAN     NOT NULL DEFAULT FALSE,
    extracted_at   TIMESTAMPTZ,
    created_at     TIMESTAMPTZ          DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMPTZ          DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_news_date UNIQUE (item_id, published_at)
);

COMMENT ON TABLE news_item IS '存储新闻条目；item_id 为来源ID，(item_id, published_at) 为业务唯一约束';
COMMENT ON COLUMN news_item.cluster_method IS '聚类算法';
COMMENT ON COLUMN news_item.cluster_id IS '语义聚类事件ID';

CREATE INDEX IF NOT EXISTS idx_news_item_cluster ON news_item (cluster_id);
CREATE INDEX IF NOT EXISTS idx_news_item_published_at ON news_item (published_at);
CREATE INDEX IF NOT EXISTS idx_news_item_id ON news_item (item_id);

-- ============================================================
-- Table: news_keywords（新闻关键词）
-- ============================================================
CREATE TABLE IF NOT EXISTS news_keywords
(
    id         BIGSERIAL PRIMARY KEY,
    news_id    BIGINT NOT NULL REFERENCES news_item (id) ON DELETE CASCADE,
    keyword    TEXT   NOT NULL,
    weight     REAL,
    method     TEXT   NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_news_keywords UNIQUE (news_id, keyword, method)
);

COMMENT ON TABLE news_keywords IS '保存新闻关键词';
COMMENT ON COLUMN news_keywords.method IS 'tfidf / textrank / embedding 等关键词提取方法';

-- ============================================================
-- Table: news_event（新闻事件，热点聚合）
-- ============================================================
CREATE TABLE IF NOT EXISTS news_event
(
    id              BIGSERIAL PRIMARY KEY,
    event_date      DATE     NOT NULL,
    cluster_id      BIGINT   NOT NULL,
    title           TEXT,
    summary         TEXT,
    news_count      INTEGER  NOT NULL DEFAULT 0,
    score           REAL,
    status          SMALLINT NOT NULL DEFAULT 0,
    parent_event_id BIGINT REFERENCES news_event (id),
    merge_at        TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE news_event IS '新闻事件表，表示在某一天内由相似新闻聚合而成的热点事件';
COMMENT ON COLUMN news_event.parent_event_id IS '父事件 ID，用于跨天合并';
COMMENT ON COLUMN news_event.merge_at IS '最后一次参与跨事件合并的时间，NULL 表示尚未合并';
COMMENT ON COLUMN news_event.status IS '0=自动生成，1=人工确认，2=已归档，3=已合并';

CREATE UNIQUE INDEX IF NOT EXISTS uniq_news_event_date_cluster ON news_event (event_date, cluster_id);
CREATE INDEX IF NOT EXISTS idx_event_day_heat ON news_event (event_date, score DESC);

-- ============================================================
-- Table: news_event_item（事件 - 新闻条目 关联）
-- ============================================================
CREATE TABLE IF NOT EXISTS news_event_item
(
    event_id   BIGINT      NOT NULL REFERENCES news_event (id) ON DELETE CASCADE,
    news_id    BIGINT      NOT NULL REFERENCES news_item (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (event_id, news_id)
);

COMMENT ON TABLE news_event_item IS '新闻事件与新闻条目的关联表';

CREATE INDEX IF NOT EXISTS idx_event_item_news ON news_event_item (news_id);

-- ============================================================
-- Table: news_event_pipeline_run（事件流水线运行记录）
-- ============================================================
CREATE TABLE IF NOT EXISTS news_event_pipeline_run
(
    id            BIGSERIAL PRIMARY KEY,
    step_name     VARCHAR(100) NOT NULL,
    event_date    DATE,
    affected_rows INTEGER,
    cost_ms       INTEGER,
    status        SMALLINT     NOT NULL DEFAULT 0,
    message       TEXT,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT now()
);

COMMENT ON TABLE news_event_pipeline_run IS '新闻事件生成与演化流水线的运行记录表';
COMMENT ON COLUMN news_event_pipeline_run.status IS '0=成功，1=失败，2=部分成功，3=被跳过';

CREATE INDEX IF NOT EXISTS idx_news_event_pipeline_run_step_date
    ON news_event_pipeline_run (step_name, event_date);

-- ============================================================
-- Table: roles（RBAC 角色）
-- ============================================================
CREATE TABLE IF NOT EXISTS roles
(
    id          BIGSERIAL PRIMARY KEY,
    name        VARCHAR(50) NOT NULL UNIQUE,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE roles IS '系统角色表';

-- ============================================================
-- Table: user_roles（用户 - 角色 关联）
-- ============================================================
CREATE TABLE IF NOT EXISTS user_roles
(
    user_id     BIGINT      NOT NULL REFERENCES user_info (id) ON DELETE CASCADE,
    role_id     BIGINT      NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, role_id)
);

COMMENT ON TABLE user_roles IS '用户与角色的关联表';

-- ============================================================
-- Table: permissions（权限 / JWT Scope）
-- ============================================================
CREATE TABLE IF NOT EXISTS permissions
(
    id          BIGSERIAL PRIMARY KEY,
    name        VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT now()
);

COMMENT ON TABLE permissions IS '系统权限表，对应 JWT scopes';

-- ============================================================
-- Table: role_permissions（角色 - 权限 关联）
-- ============================================================
CREATE TABLE IF NOT EXISTS role_permissions
(
    role_id       BIGINT      NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    permission_id BIGINT      NOT NULL REFERENCES permissions (id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (role_id, permission_id)
);

COMMENT ON TABLE role_permissions IS '角色与权限的关联表';

-- ============================================================
-- Table: plan_permissions（套餐 - 权限 关联）
-- ============================================================
CREATE TABLE IF NOT EXISTS plan_permissions
(
    plan          VARCHAR(20) NOT NULL,
    permission_id BIGINT      NOT NULL REFERENCES permissions (id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (plan, permission_id)
);

COMMENT ON TABLE plan_permissions IS 'SaaS 套餐权限表，控制不同套餐可用 scope';

-- ============================================================
-- 批量创建 updated_at 触发器
-- ============================================================
DO
$$
    DECLARE
        tbl_name  TEXT;
        trig_name TEXT;
        tables    TEXT[] := ARRAY [
            'ani_info', 'user_info', 'user_identities', 'scheduled_tasks',
            'favorites', 'watch_history', 'user_setting',
            'news_info', 'news_item', 'news_keywords',
            'news_event', 'news_event_item', 'news_event_pipeline_run',
            'roles', 'user_roles', 'permissions', 'role_permissions', 'plan_permissions'
            ];
    BEGIN
        FOREACH tbl_name IN ARRAY tables
            LOOP
                trig_name := tbl_name || '_updated_at_trg';
                IF EXISTS (SELECT 1 FROM pg_class WHERE relname = tbl_name AND relkind = 'r') THEN
                    IF EXISTS (SELECT 1
                               FROM pg_trigger t
                                        JOIN pg_class c ON t.tgrelid = c.oid
                               WHERE t.tgname = trig_name
                                 AND c.relname = tbl_name) THEN
                        EXECUTE format('DROP TRIGGER %I ON %I;', trig_name, tbl_name);
                    END IF;
                    EXECUTE format(
                            'CREATE TRIGGER %I BEFORE UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION update_updated_at();',
                            trig_name, tbl_name);
                END IF;
            END LOOP;
    END
$$;

-- ============================================================
-- 触发器：观看历史写入后自动标记收藏为已观看
-- ============================================================
CREATE OR REPLACE FUNCTION trg_after_insert_watch_func()
    RETURNS TRIGGER AS
$$
BEGIN
    UPDATE ani_collect
    SET is_watched = TRUE
    WHERE user_id = NEW.user_id
      AND ani_item_id = NEW.ani_item_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DO
$$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'trg_after_insert_watch') THEN
        CREATE TRIGGER trg_after_insert_watch
            AFTER INSERT ON ani_watch_history
            FOR EACH ROW
        EXECUTE FUNCTION trg_after_insert_watch_func();
    END IF;
END
$$;
