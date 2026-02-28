-- Your SQL goes here

CREATE TABLE executions (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT,
    client_id TEXT NOT NULL,
    executed_at DATETIME,
    execution_result TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (job_id) REFERENCES jobs (id)
);

CREATE TRIGGER update_executions_timestamp 
AFTER UPDATE ON executions
BEGIN
    UPDATE executions SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
