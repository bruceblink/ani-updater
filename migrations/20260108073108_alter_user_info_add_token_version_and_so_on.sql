-- Add migration script here
-- 增加 token_version 字段，表示用户的jwt_token版本,用于撤销token验证
ALTER TABLE user_info
    ADD COLUMN IF NOT EXISTS token_version BIGINT DEFAULT 0 NOT NULL;

-- 增加 status 字段，表示用户的账户状态
ALTER TABLE user_info
    ADD COLUMN IF NOT EXISTS status VARCHAR(50) DEFAULT 'active' NOT NULL;

-- 增加索引（可选，优化基于 status 字段的查询）
CREATE INDEX IF NOT EXISTS idx_status ON user_info (status);

-- 增加一个字段，用于标记账户的锁定到期时间（如果有锁定需求）
ALTER TABLE user_info
    ADD COLUMN IF NOT EXISTS locked_until TIMESTAMPTZ;

-- 增加一个字段，用于记录失败的登录尝试次数（如果有防暴力破解需求）
ALTER TABLE user_info
    ADD COLUMN IF NOT EXISTS failed_login_attempts INT DEFAULT 0;
-- 为 user_info 表添加 token_version 字段


-- 更新表注释
COMMENT ON COLUMN user_info.token_version IS '用户的 JWT 版本，版本变更时更新此字段，以防止旧版本的 JWT 继续有效';
COMMENT ON COLUMN user_info.status IS '用户账户状态，取值：active、inactive、suspended、frozen 等';
COMMENT ON COLUMN user_info.locked_until IS '账户锁定的到期时间，若为 null 表示未锁定';
COMMENT ON COLUMN user_info.failed_login_attempts IS '用户登录失败的尝试次数';
