use sea_query::{Alias, Expr, Func, JoinType, PostgresQueryBuilder, Query};
use uuid::Uuid;

pub fn build_job_complete_query(job_id: Uuid) -> sea_query::SelectStatement {
  let job_id_str = job_id.to_string();

  // Helper for building client JSON object matching ClientsComplete struct
  // ClientsComplete (ClientsModel) expects "createdAt" and "updatedAt" (camelCase)
  // and requires "secret".
  let client_obj_build = |alias_name: &str| {
    let alias = Alias::new(alias_name);
    Func::cust(Alias::new("jsonb_build_object"))
      .arg(Expr::val("id"))
      .arg(Expr::col((alias.clone(), Alias::new("id"))))
      .arg(Expr::val("client_name"))
      .arg(Expr::col((alias.clone(), Alias::new("client_name"))))
      .arg(Expr::val("secret"))
      .arg(Expr::col((alias.clone(), Alias::new("secret"))))
      .arg(Expr::val("createdAt"))
      .arg(Expr::col((alias.clone(), Alias::new("created_at"))))
      .arg(Expr::val("updatedAt"))
      .arg(Expr::col((alias.clone(), Alias::new("updated_at"))))
  };

  // 1. Clients Subquery (Directly assigned to Job)
  let clients_sub = Query::select()
    .expr(
      Func::cust(Alias::new("COALESCE"))
        .arg(Func::cust(Alias::new("jsonb_agg")).arg(client_obj_build("c")))
        .arg(Expr::cust("'[]'::jsonb")),
    )
    .from_as(Alias::new("clients"), Alias::new("c"))
    .join(
      JoinType::InnerJoin,
      Alias::new("jobs_clients"),
      Expr::col((Alias::new("jobs_clients"), Alias::new("client_id")))
        .eq(Expr::col((Alias::new("c"), Alias::new("id")))),
    )
    .and_where(
      Expr::col((Alias::new("jobs_clients"), Alias::new("job_id")))
        .eq(Expr::col((Alias::new("j"), Alias::new("id")))),
    )
    .to_string(PostgresQueryBuilder);

  // 2. Nested Groups Clients Subquery (Assigned to Groups)
  let group_clients_sub = Query::select()
    .expr(
      Func::cust(Alias::new("COALESCE"))
        .arg(Func::cust(Alias::new("jsonb_agg")).arg(client_obj_build("gc_c")))
        .arg(Expr::cust("'[]'::jsonb")),
    )
    .from_as(Alias::new("clients"), Alias::new("gc_c"))
    .join(
      JoinType::InnerJoin,
      Alias::new("groups_clients"),
      Expr::col((Alias::new("groups_clients"), Alias::new("client_id")))
        .eq(Expr::col((Alias::new("gc_c"), Alias::new("id")))),
    )
    .and_where(
      Expr::col((Alias::new("groups_clients"), Alias::new("group_id")))
        .eq(Expr::col((Alias::new("g"), Alias::new("id")))),
    )
    .to_string(PostgresQueryBuilder);

  // 3. Groups Subquery
  // GroupsComplete expects snake_case keys (no renames in struct), so to_jsonb(g) works for base fields.
  // We inject 'clients' array.
  let groups_sub = Query::select()
    .expr(
      Func::cust(Alias::new("COALESCE"))
        .arg(Func::cust(Alias::new("jsonb_agg")).arg(Expr::cust(format!(
          "to_jsonb(g) || jsonb_build_object('clients', ({}))",
          group_clients_sub
        ))))
        .arg(Expr::cust("'[]'::jsonb")),
    )
    .from_as(Alias::new("groups"), Alias::new("g"))
    .join(
      JoinType::InnerJoin,
      Alias::new("jobs_groups"),
      Expr::col((Alias::new("jobs_groups"), Alias::new("group_id")))
        .eq(Expr::col((Alias::new("g"), Alias::new("id")))),
    )
    .and_where(
      Expr::col((Alias::new("jobs_groups"), Alias::new("job_id")))
        .eq(Expr::col((Alias::new("j"), Alias::new("id")))),
    )
    .to_string(PostgresQueryBuilder);

  // 4. Main Query
  // JobsComplete expects snake_case keys (no renames in struct), so to_jsonb(j) works.
  // We inject 'clients' and 'groups' arrays.
  Query::select()
    .expr_as(
      Expr::cust(format!(
        "to_jsonb(j) || jsonb_build_object('clients', ({}), 'groups', ({}))",
        clients_sub, groups_sub
      )),
      Alias::new("job_blob"),
    )
    .from_as(Alias::new("jobs"), Alias::new("j"))
    .and_where(
      Expr::col((Alias::new("j"), Alias::new("id")))
        .eq(Expr::cust_with_values("$1::uuid", [job_id_str])),
    )
    .to_owned()
}
