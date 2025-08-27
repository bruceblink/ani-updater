-- Add migration script here
create table subscription_token (
    subscription_token TEXT NOT NULL,
    subscriber_id uuid NOT NULL
      references subscriptions (id),
    primary key (subscription_token)
);
