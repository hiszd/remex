pub fn cast_uuid(uuid: sqlx::types::Uuid) -> sea_query::SimpleExpr {
  sea_query::Expr::cust_with_values("$1::uuid", vec![uuid.to_string()])
}
