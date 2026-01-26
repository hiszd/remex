-- Add migration script here

CREATE TABLE groups_clients (
    group_id UUID NOT NULL REFERENCES groups (id) ON DELETE CASCADE,
    client_id UUID NOT NULL REFERENCES clients (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (group_id, client_id)
);

CREATE OR REPLACE FUNCTION update_group_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE groups
    SET updated_at = NOW()
    WHERE id = NEW.group_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trigger_update_group_time
AFTER INSERT ON groups_clients
FOR EACH ROW
EXECUTE FUNCTION update_group_timestamp();
