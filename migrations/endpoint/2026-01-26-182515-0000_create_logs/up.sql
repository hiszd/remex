-- Your SQL goes here

CREATE TABLE logs (
    id TEXT PRIMARY KEY NOT NULL,
    client_id TEXT NOT NULL,
    execution_id TEXT NOT NULL,
    output TEXT NOT NULL,
    command TEXT NOT NULL,
    exit_code TEXT NOT NULL,
    start_time DATETIME NOT NULL,
    end_time DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (execution_id) REFERENCES executions (id) ON DELETE CASCADE
);

CREATE TRIGGER update_logs_timestamp 
AFTER UPDATE ON logs
BEGIN
    UPDATE logs SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
