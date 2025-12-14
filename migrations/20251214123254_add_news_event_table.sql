-- Add migration script here
CREATE TABLE if not exists news_event
(
    id         bigserial PRIMARY KEY,
    event_date date        NOT NULL,
    cluster_id bigint      NOT NULL,

    title      text,
    summary    text,

    news_count integer     NOT NULL DEFAULT 0,

    score      real,
    status     smallint    NOT NULL DEFAULT 0,

    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);


-- 同一天同一聚类只能生成一个事件
CREATE UNIQUE INDEX uniq_news_event_date_cluster
    ON news_event (event_date, cluster_id);

-- 热点查询核心索引
CREATE INDEX idx_event_day_heat
    ON news_event (event_date, score DESC);

COMMENT ON TABLE news_event IS
    '新闻事件表，表示在某一天内，由相似新闻聚合而成的一个热点事件，是系统对外消费的核心对象';

COMMENT ON COLUMN news_event.id IS
    '事件ID，系统内部主键';

COMMENT ON COLUMN news_event.event_date IS
    '事件归属日期（通常按新闻发布时间或抓取日期聚合）';

COMMENT ON COLUMN news_event.cluster_id IS
    '关联的聚类ID，来源于算法结果，用于事件生成的实现逻辑，不保证跨天稳定';

COMMENT ON COLUMN news_event.title IS
    '事件标题，通常由新闻标题统计或后处理生成';

COMMENT ON COLUMN news_event.summary IS
    '事件摘要，可由算法生成或人工编辑';

COMMENT ON COLUMN news_event.news_count IS
    '当前事件包含的新闻数量，用于衡量事件规模';

COMMENT ON COLUMN news_event.score IS
    '事件热度或重要性评分（如新闻数量、传播速度等综合计算）';

COMMENT ON COLUMN news_event.status IS
    '事件状态：0=自动生成，1=人工确认，2=已归档，3=已合并等';

COMMENT ON COLUMN news_event.created_at IS
    '事件创建时间';

COMMENT ON COLUMN news_event.updated_at IS
    '事件更新时间';


