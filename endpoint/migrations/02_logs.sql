-- Add migration script here

CREATE TABLE logs (
  id TEXT PRIMARY KEY DEFAULT sequence(),
  uuid TEXT DEFAULT gen_random_uuid(),
  type TEXT NOT NULL,
  message TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

