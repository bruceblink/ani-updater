-- 用户表：系统内的唯一用户信息
CREATE TABLE IF NOT EXISTS users (
     id              BIGSERIAL PRIMARY KEY,
     email           VARCHAR(255) UNIQUE,
     username        VARCHAR(100) UNIQUE,
     password        TEXT,                       -- 本地登录密码哈希（可为空）
     display_name    VARCHAR(255),
     avatar_url      TEXT,
     created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
     updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE users IS '系统用户表，存储应用内的用户主体信息';
COMMENT ON COLUMN users.id IS '系统内唯一用户 ID';
COMMENT ON COLUMN users.email IS '邮箱（可选，主要用于邮箱登录或通知）';
COMMENT ON COLUMN users.username IS '用户名（可选，可用于自定义登录名）';
COMMENT ON COLUMN users.password IS '本地登录密码哈希（可为空，如果用户只用第三方登录）';
COMMENT ON COLUMN users.display_name IS '显示名（可以来自第三方平台或用户自定义）';
COMMENT ON COLUMN users.avatar_url IS '头像 URL';
COMMENT ON COLUMN users.created_at IS '用户创建时间';
COMMENT ON COLUMN users.updated_at IS '用户信息最后更新时间';


-- 用户身份表：第三方账号绑定信息
CREATE TABLE IF NOT EXISTS user_identities (
       id               BIGSERIAL PRIMARY KEY,
       user_id          BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
       provider         VARCHAR(50) NOT NULL,
       provider_user_id VARCHAR(255) NOT NULL,
       access_token     TEXT,
       refresh_token    TEXT,
       expires_at       TIMESTAMPTZ,
       created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
       updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
       UNIQUE(provider, provider_user_id)
);

COMMENT ON TABLE user_identities IS '用户身份表，存储第三方登录账号与系统用户的映射关系';
COMMENT ON COLUMN user_identities.id IS '主键 ID';
COMMENT ON COLUMN user_identities.user_id IS '关联的系统用户 ID（外键，指向 users.id）';
COMMENT ON COLUMN user_identities.provider IS '第三方登录提供商，例如 github、google、wechat';
COMMENT ON COLUMN user_identities.provider_user_id IS '第三方平台的用户唯一 ID';
COMMENT ON COLUMN user_identities.access_token IS 'OAuth access token（可选，若需调用第三方 API）';
COMMENT ON COLUMN user_identities.refresh_token IS 'OAuth refresh token（可选，用于刷新 access token）';
COMMENT ON COLUMN user_identities.expires_at IS 'access_token 过期时间（可选）';
COMMENT ON COLUMN user_identities.created_at IS '绑定记录的创建时间';
COMMENT ON COLUMN user_identities.updated_at IS '绑定记录的最后更新时间';
