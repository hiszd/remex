// @generated automatically by Diesel CLI.

diesel::table! {
    clients (id) {
        id -> Text,
        secret -> Text,
        client_name -> Text,
        hardware_hash -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    executions (id) {
        id -> Text,
        job_id -> Nullable<Text>,
        client_id -> Text,
        executed_at -> Nullable<Timestamp>,
        execution_result -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    groups (id) {
        id -> Text,
        group_name -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    groups_clients (id) {
        id -> Int4,
        group_id -> Nullable<Text>,
        client_id -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    jobs (id) {
        id -> Text,
        job_name -> Text,
        job_type -> Text,
        job_status -> Text,
        job_shell -> Text,
        job_command -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    jobs_groups (id) {
        id -> Int4,
        job_id -> Nullable<Text>,
        group_id -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    logs (id) {
        id -> Text,
        client_id -> Text,
        execution_id -> Text,
        output -> Text,
        command -> Text,
        exit_code -> Text,
        start_time -> Timestamp,
        end_time -> Timestamp,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(executions -> jobs (job_id));
diesel::joinable!(groups_clients -> clients (client_id));
diesel::joinable!(groups_clients -> groups (group_id));
diesel::joinable!(jobs_groups -> groups (group_id));
diesel::joinable!(jobs_groups -> jobs (job_id));
diesel::joinable!(logs -> executions (execution_id));

diesel::allow_tables_to_appear_in_same_query!(
  clients,
  executions,
  groups,
  groups_clients,
  jobs,
  jobs_groups,
  logs,
);
