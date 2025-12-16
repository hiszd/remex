-- Add migration script here

CREATE TABLE jobs_clients (
    job_id UUID REFERENCES jobs (id) ON DELETE CASCADE,
    client_id UUID REFERENCES clients (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_jobs_clients_updated_at
BEFORE UPDATE ON jobs_clients
FOR EACH ROW
EXECUTE FUNCTION update_modified_column();
