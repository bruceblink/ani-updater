-- Add migration script here
create table if not exists news_event_pipeline_run
(
    id            bigserial primary key,
    step_name     varchar(100)               not null,
    event_date    date,
    affected_rows integer,
    cost_ms       integer,
    status        smallint    default 0     not null,
    message       text,
    created_at    timestamptz default now() not null,
    updated_at    timestamptz default now() not null
);

comment on table news_event_pipeline_run is
    '新闻事件生成与演化流水线的运行记录表，用于记录每个 pipeline step 的执行情况、影响数据量和运行状态，支持调试、回溯和运维观测';

comment on column news_event_pipeline_run.id is
    '流水线运行记录唯一标识';

comment on column news_event_pipeline_run.step_name is
    '流水线步骤名称，如 step1_cluster, step2_generate_event, step7_merge_cross_day';

comment on column news_event_pipeline_run.event_date is
    '本次流水线运行关联的业务日期（事件日期），用于区分每日批处理或补跑';

comment on column news_event_pipeline_run.affected_rows is
    '本步骤影响的数据行数，如新增或更新的事件数量';

comment on column news_event_pipeline_run.cost_ms is
    '本步骤执行耗时，单位毫秒';

comment on column news_event_pipeline_run.status is
    '执行状态：0-成功，1-失败，2-部分成功，3-被跳过';

comment on column news_event_pipeline_run.message is
    '补充信息或错误信息，用于记录异常原因、阈值说明或调试信息';

comment on column news_event_pipeline_run.created_at is
    '流水线步骤实际执行时间';

create index if not exists idx_news_event_pipeline_run_step_date
    on news_event_pipeline_run (step_name, event_date);


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
        tables    text[] := ARRAY ['news_event_pipeline_run']; -- 这里列出所有目标表
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

ALTER TABLE news_event
    ADD COLUMN  IF NOT EXISTS merge_at TIMESTAMPTZ;

COMMENT ON COLUMN news_event.merge_at IS
    '最后一次参与跨事件合并的时间；NULL 表示尚未发生合并';