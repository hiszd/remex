pub struct Endpoint {
  clientname: String,
  uid: String,
}

impl Endpoint {
  pub fn new(clientname: String, uid: String) -> Self {
    Self { clientname, uid }
  }
}
