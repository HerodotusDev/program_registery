-- Add migration script here
ALTER TABLE programs
ADD CONSTRAINT unique_hash UNIQUE (hash);
