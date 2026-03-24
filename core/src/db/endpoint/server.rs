// @generated automatically by Diesel CLI.

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
diesel::joinable!(logs -> executions (execution_id));

diesel::allow_tables_to_appear_in_same_query!(executions, jobs, logs,);
