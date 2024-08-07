pub struct Endpoint {
  clientname: String,
}

impl Endpoint {
  pub fn new(clientname: String) -> Self { Self { clientname } }
}
