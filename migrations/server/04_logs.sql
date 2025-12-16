-- Add migration script here

CREATE TABLE logs (
    id UUID PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    client_id UUID NOT NULL,
    FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
    execution_id UUID NOT NULL,
    FOREIGN KEY (execution_id) REFERENCES executions (id) ON DELETE CASCADE,
    log TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_logs_updated_at
BEFORE UPDATE ON logs
FOR EACH ROW
EXECUTE FUNCTION update_modified_column();
