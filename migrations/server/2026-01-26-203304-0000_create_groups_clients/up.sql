-- Your SQL goes here

CREATE TABLE groups_clients (
    group_id TEXT REFERENCES groups (id) ON DELETE CASCADE,
    client_id TEXT REFERENCES clients (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (group_id, client_id)
);

CREATE TRIGGER update_groups_clients_updated_at
BEFORE UPDATE ON groups_clients
FOR EACH ROW
EXECUTE FUNCTION update_modified_column();
