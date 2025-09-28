-- Add migration script here
-- ========================
-- 批量添加 updated_at 自动更新时间触发器
-- ========================

-- 1. 创建/替换触发器函数（所有表共用）
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
        tables    text[] := ARRAY ['ani_info', 'user_info', 'user_identities','video_info', 'scheduled_tasks', 'another_table']; -- 这里列出所有目标表
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
