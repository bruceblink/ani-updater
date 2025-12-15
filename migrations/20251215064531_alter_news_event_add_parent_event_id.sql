-- Add migration script here
ALTER TABLE news_event
    ADD COLUMN IF NOT EXISTS parent_event_id bigint
        REFERENCES news_event(id);
