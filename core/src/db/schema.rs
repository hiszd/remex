// @generated automatically by Diesel CLI.

diesel::table! {
    clients (id) {
        id -> Text,
        secret -> Text,
        client_name -> Text,
        hardware_hash -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    executions (id) {
        id -> Text,
        job_id -> Nullable<Text>,
        client_id -> Text,
        executed_at -> Nullable<Timestamptz>,
        execution_result -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    groups (id) {
        id -> Text,
        group_name -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    groups_clients (group_id, client_id) {
        group_id -> Text,
        client_id -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    jobs (id) {
        id -> Text,
        job_name -> Text,
        job_type -> Text,
        job_status -> Text,
        job_shell -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    jobs_clients (job_id, client_id) {
        job_id -> Text,
        client_id -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    jobs_groups (job_id, group_id) {
        job_id -> Text,
        group_id -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    logs (id) {
        id -> Text,
        client_id -> Text,
        execution_id -> Text,
        log -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(executions -> clients (client_id));
diesel::joinable!(executions -> jobs (job_id));
diesel::joinable!(groups_clients -> clients (client_id));
diesel::joinable!(groups_clients -> groups (group_id));
diesel::joinable!(jobs_clients -> clients (client_id));
diesel::joinable!(jobs_clients -> jobs (job_id));
diesel::joinable!(jobs_groups -> groups (group_id));
diesel::joinable!(jobs_groups -> jobs (job_id));
diesel::joinable!(logs -> clients (client_id));
diesel::joinable!(logs -> executions (execution_id));

diesel::allow_tables_to_appear_in_same_query!(
  clients,
  executions,
  groups,
  groups_clients,
  jobs,
  jobs_clients,
  jobs_groups,
  logs,
);
