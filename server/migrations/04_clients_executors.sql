-- Add migration script here

CREATE TABLE clients_executors (
    machineid TEXT REFERENCES clients(machineid) ON DELETE CASCADE,
    executor_id TEXT REFERENCES executors(id) ON DELETE CASCADE,
    PRIMARY KEY (machineid, executor_id) -- Composite primary key for unique student-course pairings
);
