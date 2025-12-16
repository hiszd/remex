-- Add migration script here

CREATE TABLE logs (
    id TEXT PRIMARY KEY NOT NULL,
    client_id TEXT NOT NULL,
    client_name TEXT NOT NULL,
    log TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
