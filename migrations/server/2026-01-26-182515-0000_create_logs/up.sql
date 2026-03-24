-- Your SQL goes here

CREATE TABLE logs (
    id TEXT PRIMARY KEY NOT NULL,
    client_id TEXT NOT NULL,
    execution_id TEXT NOT NULL,
    FOREIGN KEY (execution_id) REFERENCES executions (id) ON DELETE CASCADE,
    output TEXT NOT NULL,
    command TEXT NOT NULL,
    exit_code TEXT NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_logs_updated_at
BEFORE UPDATE ON logs
FOR EACH ROW
EXECUTE FUNCTION update_modified_column();
