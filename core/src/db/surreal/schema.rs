pub const SCHEMA_DEFINITIONS: &str = r#"
-- Define namespace and database
DEFINE NAMESPACE IF NOT EXISTS remex;
DEFINE DATABASE IF NOT EXISTS remex;

-- Use the remex database
USE NS remex DB remex;

-- JWT secret for scope authentication (HS256)
DEFINE SECRET IF NOT EXISTS jwt_secret VALUE "remex_jwt_secret_key_change_in_production";

-- Define scope for endpoints
DEFINE SCOPE IF NOT EXISTS endpoint
  SESSION 7d
  SIGNUP ( CREATE user SET username = $username, password = $password )
  SIGNIN ( SELECT * FROM user WHERE username = $username AND crypto::argon2::compare(password, $password) );

-- Clients table
DEFINE TABLE IF NOT EXISTS clients SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON clients TYPE string;
DEFINE FIELD IF NOT EXISTS secret ON clients TYPE string;
DEFINE FIELD IF NOT EXISTS client_name ON clients TYPE string;
DEFINE FIELD IF NOT EXISTS hardware_hash ON clients TYPE string;
DEFINE FIELD IF NOT EXISTS created_at ON clients TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON clients TYPE datetime;
DEFINE INDEX IF NOT EXISTS idx_clients_id ON clients COLUMNS id UNIQUE;

-- Jobs table
DEFINE TABLE IF NOT EXISTS jobs SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON jobs TYPE string;
DEFINE FIELD IF NOT EXISTS job_name ON jobs TYPE string;
DEFINE FIELD IF NOT EXISTS job_type ON jobs TYPE string;
DEFINE FIELD IF NOT EXISTS job_status ON jobs TYPE string;
DEFINE FIELD IF NOT EXISTS job_shell ON jobs TYPE string;
DEFINE FIELD IF NOT EXISTS job_command ON jobs TYPE string;
DEFINE FIELD IF NOT EXISTS created_at ON jobs TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON jobs TYPE datetime;
DEFINE INDEX IF NOT EXISTS idx_jobs_id ON jobs COLUMNS id UNIQUE;

-- Executions table
DEFINE TABLE IF NOT EXISTS executions SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON executions TYPE string;
DEFINE FIELD IF NOT EXISTS job_id ON executions TYPE option<string>;
DEFINE FIELD IF NOT EXISTS client_id ON executions TYPE string;
DEFINE FIELD IF NOT EXISTS executed_at ON executions TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS execution_result ON executions TYPE option<string>;
DEFINE FIELD IF NOT EXISTS created_at ON executions TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON executions TYPE datetime;
DEFINE INDEX IF NOT EXISTS idx_executions_id ON executions COLUMNS id UNIQUE;
DEFINE INDEX IF NOT EXISTS idx_executions_job_id ON executions COLUMNS job_id;
DEFINE INDEX IF NOT EXISTS idx_executions_client_id ON executions COLUMNS client_id;

-- Logs table
DEFINE TABLE IF NOT EXISTS logs SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON logs TYPE string;
DEFINE FIELD IF NOT EXISTS client_id ON logs TYPE string;
DEFINE FIELD IF NOT EXISTS execution_id ON logs TYPE string;
DEFINE FIELD IF NOT EXISTS output ON logs TYPE string;
DEFINE FIELD IF NOT EXISTS command ON logs TYPE string;
DEFINE FIELD IF NOT EXISTS exit_code ON logs TYPE string;
DEFINE FIELD IF NOT EXISTS start_time ON logs TYPE datetime;
DEFINE FIELD IF NOT EXISTS end_time ON logs TYPE datetime;
DEFINE FIELD IF NOT EXISTS created_at ON logs TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON logs TYPE datetime;
DEFINE INDEX IF NOT EXISTS idx_logs_id ON logs COLUMNS id UNIQUE;
DEFINE INDEX IF NOT EXISTS idx_logs_execution_id ON logs COLUMNS execution_id;
DEFINE INDEX IF NOT EXISTS idx_logs_client_id ON logs COLUMNS client_id;

-- Groups table
DEFINE TABLE IF NOT EXISTS groups SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON groups TYPE string;
DEFINE FIELD IF NOT EXISTS group_name ON groups TYPE string;
DEFINE FIELD IF NOT EXISTS created_at ON groups TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON groups TYPE datetime;
DEFINE INDEX IF NOT EXISTS idx_groups_id ON groups COLUMNS id UNIQUE;

-- Groups-Clients relationship table
DEFINE TABLE IF NOT EXISTS groups_clients SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON groups_clients TYPE int;
DEFINE FIELD IF NOT EXISTS group_id ON groups_clients TYPE option<string>;
DEFINE FIELD IF NOT EXISTS client_id ON groups_clients TYPE option<string>;
DEFINE FIELD IF NOT EXISTS created_at ON groups_clients TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON groups_clients TYPE datetime;
DEFINE INDEX IF NOT EXISTS idx_groups_clients_id ON groups_clients COLUMNS id UNIQUE;

-- Jobs-Groups relationship table
DEFINE TABLE IF NOT EXISTS jobs_groups SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON jobs_groups TYPE int;
DEFINE FIELD IF NOT EXISTS job_id ON jobs_groups TYPE option<string>;
DEFINE FIELD IF NOT EXISTS group_id ON jobs_groups TYPE option<string>;
DEFINE FIELD IF NOT EXISTS created_at ON jobs_groups TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON jobs_groups TYPE datetime;
DEFINE INDEX IF NOT EXISTS idx_jobs_groups_id ON jobs_groups COLUMNS id UNIQUE;
"#;
