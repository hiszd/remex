-- Add migration script here

CREATE TABLE logs_executors (
    log_id TEXT REFERENCES logs(id) ON DELETE CASCADE,
    executor_id TEXT REFERENCES executors(id) ON DELETE CASCADE,
    PRIMARY KEY (log_id, executor_id)
);
