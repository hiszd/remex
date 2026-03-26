use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct Client {
  pub id: String,
  pub secret: String,
  pub client_name: String,
  pub hardware_hash: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl super::DbOperator for Client {
  fn create_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::clients::{
      ClientSRV,
      NewClientSRV,
    };
    use schema::server::clients;
    match diesel::insert_into(clients::table)
      .values(NewClientSRV::from(self.clone()))
      .get_result::<ClientSRV>(conn)
    {
      Ok(client) => Ok(client.into()),
      Err(e) => Err(e),
    }
  }
  fn update_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::clients::{
      ClientSRV,
      UpdateClientSRV,
    };
    use schema::server::clients;
    match diesel::update(clients::table.find(self.id.clone()))
      .set(UpdateClientSRV::from(self.clone()))
      .get_result::<ClientSRV>(conn)
    {
      Ok(client) => Ok(client.into()),
      Err(e) => Err(e),
    }
  }
  fn delete_srv(&self, conn: &mut diesel::PgConnection) -> Result<(), diesel::result::Error> {
    use schema::server::clients;
    match diesel::delete(clients::table.find(self.id.clone())).execute(conn) {
      Ok(_) => Ok(()),
      Err(e) => Err(e),
    }
  }
  fn read_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::clients::ClientSRV;
    use schema::server::clients;
    match clients::table
      .find(self.id.clone())
      .get_result::<ClientSRV>(conn)
    {
      Ok(client) => Ok(client.into()),
      Err(e) => Err(e),
    }
  }
  fn upsert_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::clients::{
      ClientSRV,
      NewClientSRV,
      UpsertClientSRV,
    };
    use schema::server::clients;
    diesel::insert_into(clients::table)
      .values(NewClientSRV::from(self.clone()))
      .on_conflict(clients::id)
      .do_update()
      .set(UpsertClientSRV::from(self.clone()))
      .execute(conn)?;
    clients::table
      .find(self.id.clone())
      .get_result::<ClientSRV>(conn)
      .map(|client| client.into())
  }
}
