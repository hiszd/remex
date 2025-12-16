-- Add migration script here

CREATE TABLE clients (
    id UUID PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    secret TEXT NOT NULL,
    client_name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_clients_updated_at
BEFORE UPDATE ON clients
FOR EACH ROW
EXECUTE FUNCTION update_modified_column();
