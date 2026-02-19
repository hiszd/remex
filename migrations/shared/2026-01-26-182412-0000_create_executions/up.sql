-- Your SQL goes here

CREATE TABLE executions (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT,
    FOREIGN KEY (job_id) REFERENCES jobs (id),
    client_id TEXT NOT NULL,
    executed_at TIMESTAMPTZ,
    execution_result TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_executions_updated_at
BEFORE UPDATE ON executions
FOR EACH ROW
EXECUTE FUNCTION update_modified_column();
