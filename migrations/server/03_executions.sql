-- Add migration script here

CREATE TABLE executions (
    id UUID PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    job_id UUID,
    FOREIGN KEY (job_id) REFERENCES jobs (id),
    client_id UUID NOT NULL,
    FOREIGN KEY (client_id) REFERENCES clients (id) ON DELETE CASCADE,
    executed_at TIMESTAMPTZ,
    execution_result TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_executions_updated_at
BEFORE UPDATE ON executions
FOR EACH ROW
EXECUTE FUNCTION update_modified_column();
