-- This file should undo anything in `up.sql`

DROP TRIGGER update_jobs_clients_updated_at ON jobs_clients;
DROP TABLE jobs_clients;
