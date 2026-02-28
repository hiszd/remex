-- Your SQL goes here

CREATE TABLE jobs (
    id TEXT PRIMARY KEY NOT NULL,
    job_name TEXT NOT NULL,
    job_type TEXT NOT NULL,
    job_status TEXT NOT NULL,
    job_shell TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_jobs_updated_at
BEFORE UPDATE ON jobs
FOR EACH ROW
EXECUTE FUNCTION update_modified_column();
