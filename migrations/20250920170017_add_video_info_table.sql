-- =====================================
-- 视频信息表 video_info
-- =====================================
DROP TABLE IF EXISTS video_info;
CREATE TABLE IF NOT EXISTS video_info
(
    id                 UUID         NOT NULL DEFAULT gen_random_uuid(),
    title              VARCHAR(255) NOT NULL,
    rating             JSONB,
    pic                JSONB,
    is_new             BOOLEAN,
    uri                TEXT,
    episodes_info      TEXT NOT NULL DEFAULT '',
    card_subtitle      TEXT,
    type           VARCHAR(128),
    director           JSONB,
    screenwriter       JSONB,
    actors             JSONB,
    production_country JSONB,
    language           JSONB,
    release_year       SMALLINT,
    release_date       JSONB,
    duration           JSONB,
    aka                JSONB,
    original_title     VARCHAR(255),
    intro              TEXT,
    genres             JSONB,
    imdb               VARCHAR(64),
    created_at         TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at         TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT video_info_pkey PRIMARY KEY (id),
    CONSTRAINT uniq_video_info UNIQUE (title, episodes_info)
);

-- =====================================
-- 表注释
-- =====================================
COMMENT ON TABLE video_info IS '视频信息表，存储各类影片（电影、电视剧、动漫等）信息';
COMMENT ON COLUMN video_info.id IS '主键 UUID，唯一标识每条视频记录';
COMMENT ON COLUMN video_info.title IS '视频标题';
COMMENT ON COLUMN video_info.rating IS '评分信息，JSON 格式，可包含各平台评分';
COMMENT ON COLUMN video_info.pic IS '封面及相关图片信息，JSON 格式';
COMMENT ON COLUMN video_info.is_new IS '是否为最新视频，布尔值';
COMMENT ON COLUMN video_info.uri IS '视频播放链接或资源地址';
COMMENT ON COLUMN video_info.episodes_info IS '剧集信息，JSON 或文本格式，可存储集数、季数等';
COMMENT ON COLUMN video_info.card_subtitle IS '视频副标题或短描述';
COMMENT ON COLUMN video_info.type IS '视频分类，如 动漫、电视剧、电影 等';
COMMENT ON COLUMN video_info.director IS '导演信息，JSON 格式，支持多导演';
COMMENT ON COLUMN video_info.screenwriter IS '编剧信息，JSON 格式，支持多编剧';
COMMENT ON COLUMN video_info.actors IS '主演信息，JSON 格式，支持多演员';
COMMENT ON COLUMN video_info.production_country IS '制片地区/国家，JSON 格式支持多地区';
COMMENT ON COLUMN video_info.language IS '视频语言，如 中文、英语等';
COMMENT ON COLUMN video_info.release_year IS '上映年份，如 2025';
COMMENT ON COLUMN video_info.release_date IS '具体上映时间，如 2025-08-15(中国香港)';
COMMENT ON COLUMN video_info.duration IS '片长，单位为分钟';
COMMENT ON COLUMN video_info.aka IS '又名/别名，可存单个或多个备用名称';
COMMENT ON COLUMN video_info.original_title IS '原始标题';
COMMENT ON COLUMN video_info.intro IS '简介';
COMMENT ON COLUMN video_info.genres IS '所属类型/流派';
COMMENT ON COLUMN video_info.imdb IS 'imdb编号';
COMMENT ON COLUMN video_info.created_at IS '记录创建时间';
COMMENT ON COLUMN video_info.updated_at IS '记录最后更新时间';

-- =====================================
-- 索引
-- =====================================
CREATE INDEX IF NOT EXISTS idx_video_info_title
    ON video_info (title);

CREATE INDEX IF NOT EXISTS idx_video_info_type
    ON video_info (type);

CREATE INDEX IF NOT EXISTS idx_video_info_release_year
    ON video_info (release_year);
