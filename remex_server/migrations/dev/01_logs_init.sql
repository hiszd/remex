CREATE TABLE IF NOT EXISTS logs (
	id INTEGER PRIMARY KEY,
  client TEXT NOT NULL UNIQUE,
	message TEXT NOT NULL,
  time_logged TIMESTAMP NOT NULL,
  created_at TIMESTAMP
        WITH
            TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP
        WITH
            TIME ZONE DEFAULT NOW()
);
