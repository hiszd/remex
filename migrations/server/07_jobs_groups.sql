-- Add migration script here

CREATE TABLE jobs_groups (
    job_id UUID REFERENCES jobs (id) ON DELETE CASCADE,
    group_id UUID REFERENCES groups (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (job_id, group_id)
);

CREATE OR REPLACE FUNCTION update_jobs_groups_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE jobs
    SET updated_at = NOW()
    WHERE id = NEW.job_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trigger_update_jobs_groups_time
AFTER INSERT ON jobs_groups
FOR EACH ROW
EXECUTE FUNCTION update_jobs_groups_timestamp();
