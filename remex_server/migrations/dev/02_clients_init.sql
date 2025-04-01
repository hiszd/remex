CREATE TABLE IF NOT EXISTS clients (
	id BIGINT PRIMARY KEY,
  client_id TEXT NOT NULL UNIQUE,
  clientname TEXT NOT NULL UNIQUE,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE OR REPLACE FUNCTION generate_unique_client_id() RETURNS BIGINT AS $$
DECLARE
    new_id BIGINT;
    attempts INT := 0;
BEGIN
    LOOP
        new_id := floor(random() * 9223372036854775807);
        EXIT WHEN NOT EXISTS (SELECT 1 FROM clients WHERE id = new_id);
        attempts := attempts + 1;
        IF attempts > 1000 THEN
            RAISE EXCEPTION 'Could not generate unique ID after 1000 attempts.';
        END IF;
    END LOOP;
    RETURN new_id;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION set_client_id() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.id IS NULL THEN
        NEW.id := generate_unique_client_id();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_client_id_on_insert
BEFORE INSERT ON clients
FOR EACH ROW
EXECUTE FUNCTION set_client_id();
