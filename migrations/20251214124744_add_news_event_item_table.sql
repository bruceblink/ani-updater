-- Add migration script here
CREATE TABLE IF NOT EXISTS news_event_item
(
    event_id   bigint NOT NULL,
    news_id    bigint NOT NULL,

    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),

    PRIMARY KEY (event_id, news_id),

    FOREIGN KEY (event_id)
        REFERENCES news_event (id)
        ON DELETE CASCADE,

    FOREIGN KEY (news_id)
        REFERENCES news_item (id)
        ON DELETE CASCADE
);

COMMENT ON TABLE news_event_item IS
    '新闻事件与新闻条目的关联表，支持一个事件包含多条新闻，也支持新闻在未来被重新归类';

COMMENT ON COLUMN news_event_item.event_id IS
    '关联的新闻事件ID';

COMMENT ON COLUMN news_event_item.news_id IS
    '关联的新闻ID';

-- 用于查询新闻事件对应的新闻
CREATE INDEX if not exists idx_event_item_news
    ON news_event_item (news_id);
