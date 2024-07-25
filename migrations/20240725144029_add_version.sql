-- Add migration script here
ALTER TABLE programs
ADD COLUMN version INTEGER NOT NULL DEFAULT 0;
