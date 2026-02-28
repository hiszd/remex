-- Your SQL goes here

CREATE TABLE clients (
    id TEXT PRIMARY KEY NOT NULL,
    secret TEXT NOT NULL,
    client_name TEXT NOT NULL,
    hardware_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_clients_updated_at
BEFORE UPDATE ON clients
FOR EACH ROW
EXECUTE FUNCTION update_modified_column();
