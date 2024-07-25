-- Add migration script here
CREATE TABLE programs (
    id UUID PRIMARY KEY,
    hash TEXT NOT NULL,
    code BYTEA NOT NULL
);