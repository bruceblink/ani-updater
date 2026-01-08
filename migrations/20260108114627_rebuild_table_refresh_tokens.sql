-- Add migration script here
DROP TABLE IF EXISTS refresh_tokens;
CREATE TABLE IF NOT EXISTS refresh_tokens
(
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT NOT NULL,
    token      TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    revoked    BOOLEAN DEFAULT FALSE NOT NULL,
    CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES user_info(id) ON DELETE CASCADE
);
CREATE INDEX idx_user_id ON refresh_tokens (user_id);
CREATE INDEX idx_expires_at ON refresh_tokens (expires_at);
