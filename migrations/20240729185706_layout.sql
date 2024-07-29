-- Add migration script here
ALTER TABLE programs
ADD COLUMN layout TEXT NOT NULL DEFAULT '';