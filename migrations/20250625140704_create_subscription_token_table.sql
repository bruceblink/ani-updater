-- Add migration script here
CREATE TABLE IF NOT EXISTS subscription_token (
    subscription_token TEXT NOT NULL,
    subscriber_id uuid NOT NULL
      references subscriptions (id),
    primary key (subscription_token)
);
