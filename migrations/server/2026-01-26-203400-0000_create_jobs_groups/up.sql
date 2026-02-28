-- Your SQL goes here

CREATE TABLE jobs_groups (
    id SERIAL PRIMARY KEY,
    job_id TEXT REFERENCES jobs (id) ON DELETE CASCADE,
    group_id TEXT REFERENCES groups (id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_jobs_groups_updated_at
BEFORE UPDATE ON jobs_groups
FOR EACH ROW
EXECUTE FUNCTION update_modified_column();
