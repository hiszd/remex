-- Add migration script here

CREATE TABLE jobs_clients (
    job_id UUID REFERENCES jobs (id) ON DELETE CASCADE,
    client_id UUID REFERENCES clients (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (job_id, client_id)
);

CREATE OR REPLACE FUNCTION update_jobs_clients_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE jobs
    SET updated_at = NOW()
    WHERE id = NEW.job_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trigger_update_jobs_clients_time
AFTER INSERT ON jobs_clients
FOR EACH ROW
EXECUTE FUNCTION update_jobs_clients_timestamp();
