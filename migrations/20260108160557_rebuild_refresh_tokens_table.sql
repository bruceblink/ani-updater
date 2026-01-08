-- Add migration script here
DROP TABLE IF EXISTS refresh_tokens;
CREATE TABLE if not exists refresh_tokens
(
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL CONSTRAINT fk_refresh_tokens_user_info REFERENCES user_info(id),  -- 关联用户表
    token TEXT NOT NULL,  -- refresh_token 字符串
    expires_at TIMESTAMPTZ NOT NULL,
    -- 当前 refresh_token 的滑动窗口到期时间，每次刷新都会更新
    -- 例如 now() + 30 days，但不会超过 session_expires_at
    session_expires_at TIMESTAMPTZ NOT NULL,
    -- 登录会话的绝对最晚到期时间，固定值
    -- 用于强制用户重新登录，即使 refresh_token 还没过期
    revoked BOOLEAN DEFAULT false NOT NULL,
    -- 标记 token 是否被主动作废，例如用户登出或被管理员撤销
    created_at TIMESTAMPTZ DEFAULT now() NOT NULL -- token 生成时间
);

-- 为快速查询有效 token 建立索引
CREATE UNIQUE INDEX idx_refresh_tokens_user_token ON refresh_tokens (user_id, token);
-- 可选：为查询所有有效 token 提供索引
CREATE INDEX idx_refresh_tokens_valid ON refresh_tokens (expires_at, revoked);

COMMENT ON COLUMN refresh_tokens.user_id IS '关联用户表 user_info 的 ID';
COMMENT ON COLUMN refresh_tokens.token IS 'refresh_token 字符串，用于刷新 access_token';
COMMENT ON COLUMN refresh_tokens.expires_at IS '当前 refresh_token 到期时间（滑动窗口）';
COMMENT ON COLUMN refresh_tokens.session_expires_at IS '会话最晚到期时间，用于强制重新登录';
COMMENT ON COLUMN refresh_tokens.revoked IS '标记 token 是否被撤销';
COMMENT ON COLUMN refresh_tokens.created_at IS 'token 创建时间';
