-- Your SQL goes here

CREATE TABLE jobs (
    id INT PRIMARY KEY NOT NULL,
    job_name TEXT NOT NULL,
    job_type TEXT NOT NULL,
    job_status TEXT NOT NULL,
    job_shell TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_jobs_timestamp 
AFTER UPDATE ON jobs
BEGIN
    UPDATE jobs SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
