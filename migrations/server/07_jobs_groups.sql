-- Add migration script here

CREATE TABLE jobs_groups (
    job_id UUID REFERENCES jobs (id) ON DELETE CASCADE,
    group_id UUID REFERENCES groups (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_jobs_groups_updated_at
BEFORE UPDATE ON jobs_groups
FOR EACH ROW
EXECUTE FUNCTION update_modified_column();
