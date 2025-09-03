-- Add migration script here
-- 用户表：系统内的唯一用户信息
CREATE TABLE IF NOT EXISTS user_info
(
    id           BIGSERIAL PRIMARY KEY,
    email        VARCHAR(255) UNIQUE,
    username     VARCHAR(100) UNIQUE,
    password     TEXT, -- 本地登录密码哈希（可为空）
    display_name VARCHAR(255),
    avatar_url   TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE user_info IS '系统用户表，存储应用内的用户主体信息';
COMMENT ON COLUMN user_info.id IS '系统内唯一用户 ID';
COMMENT ON COLUMN user_info.email IS '邮箱（可选，主要用于邮箱登录或通知）';
COMMENT ON COLUMN user_info.username IS '用户名（可选，可用于自定义登录名）';
COMMENT ON COLUMN user_info.password IS '本地登录密码哈希（可为空，如果用户只用第三方登录）';
COMMENT ON COLUMN user_info.display_name IS '显示名（可以来自第三方平台或用户自定义）';
COMMENT ON COLUMN user_info.avatar_url IS '头像 URL';
COMMENT ON COLUMN user_info.created_at IS '用户创建时间';
COMMENT ON COLUMN user_info.updated_at IS '用户信息最后更新时间';


-- 用户身份表：第三方账号绑定信息
CREATE TABLE IF NOT EXISTS user_identities
(
    id               BIGSERIAL PRIMARY KEY,
    user_id          BIGINT       NOT NULL REFERENCES user_info (id) ON DELETE CASCADE,
    provider         VARCHAR(50)  NOT NULL, -- 第三方提供商，例如 'github'
    provider_uid     VARCHAR(255) NOT NULL, -- 第三方用户唯一标识（如 GitHub id）
    access_token     TEXT,                  -- 第三方 access_token（可选）
    token_expires_at TIMESTAMPTZ,           -- token 过期时间（如果第三方有）
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ  NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_uid)
);

COMMENT ON TABLE user_identities IS '用户身份表，存储第三方登录账号与系统用户的映射关系';
COMMENT ON COLUMN user_identities.id IS '主键 ID';
COMMENT ON COLUMN user_identities.user_id IS '关联的系统用户 ID（外键，指向 user_info.id）';
COMMENT ON COLUMN user_identities.provider IS '第三方登录提供商，例如 github、google、wechat';
COMMENT ON COLUMN user_identities.provider_uid IS '第三方平台的用户唯一 ID';
COMMENT ON COLUMN user_identities.access_token IS '第三方 OAuth access token（可选，用于调用 API）';
COMMENT ON COLUMN user_identities.token_expires_at IS '第三方 access_token 过期时间';
COMMENT ON COLUMN user_identities.created_at IS '绑定记录的创建时间';
COMMENT ON COLUMN user_identities.updated_at IS '绑定记录的最后更新时间';


-- 刷新 Token 表：用于长时间会话管理
CREATE TABLE IF NOT EXISTS refresh_tokens
(
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT      NOT NULL REFERENCES user_info (id) ON DELETE CASCADE,
    token      TEXT        NOT NULL,              -- 存储 refresh_token（建议哈希存储）
    expires_at TIMESTAMPTZ NOT NULL,              -- 过期时间
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked    BOOLEAN     NOT NULL DEFAULT FALSE -- 是否已撤销
);

COMMENT ON TABLE refresh_tokens IS '刷新 Token 表，用于长时间会话管理';
COMMENT ON COLUMN refresh_tokens.id IS '主键 ID';
COMMENT ON COLUMN refresh_tokens.user_id IS '关联的系统用户 ID（外键，指向 user_info.id）';
COMMENT ON COLUMN refresh_tokens.token IS '刷新 Token，建议存储哈希值而不是明文';
COMMENT ON COLUMN refresh_tokens.expires_at IS '刷新 Token 过期时间';
COMMENT ON COLUMN refresh_tokens.created_at IS '刷新 Token 创建时间';
COMMENT ON COLUMN refresh_tokens.revoked IS '刷新 Token 是否已撤销';