create table pastes (
    id serial primary key,
    uuid text not null,
    content text not null,
    expires_at timestamp with time zone not null
);
