-- Add migration script here
-- 更新published_at的类型为date
DO $$
    BEGIN
        IF pg_typeof((SELECT published_at FROM news_item LIMIT 1))::text != 'date' THEN
            ALTER TABLE news_item
                ALTER COLUMN published_at TYPE date
                    USING published_at::date;
        END IF;
    END$$;