-- Add migration script here
ALTER TABLE programs
ADD COLUMN builtins TEXT[] NOT NULL DEFAULT array[]::TEXT[];