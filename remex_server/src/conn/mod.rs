use remex_core::{db, Message, Packet};
use tokio::net::TcpStream;
use tracing::{error, info, warn};

use crate::endpoint::Endpoint;

// the purpose of this struct is to encapuslate the connection to the endpoint
pub struct Conn {
  socket: TcpStream,
  secret: String,
  endpoint: Option<Endpoint>,
  db: db::Db,
}

impl Conn {
  pub async fn new(socket: TcpStream, secret: String) -> Self {
    let path_str = if !cfg!(debug_assertions) {
      "db/prod.db"
    } else {
      "db/dev.db"
    };
    let db = db::Db::new(path_str.to_string()).await;
    Self {
      socket,
      secret,
      endpoint: None,
      db,
    }
  }

  // steps to connection:
  // 1. send secret
  // 2. receive secret
  // 3. receive clientname
  // etc...
  pub async fn process(&mut self) {
    match self.await_secret().await {
      Ok(_) => {
        let clientname = self.await_clientname().await;
        info!("secret received and verified for client {}", clientname);
        self.endpoint = Some(Endpoint::new(clientname));
      }
      Err(e) => {
        error!("Secret verification failed. Reason: {}", String::from(e));
        return;
      }
    }
  }

  async fn await_secret(&self) -> Result<(), crate::ERROR> {
    self.await_send(self.secret.clone()).await;

    let receivedsecret = self.await_message().await;
    info!("got secret: {:?}", receivedsecret);
    match self.secret == receivedsecret {
      true => Ok(()),
      false => Err(crate::ERROR::InvalidSecret),
    }
  }

  async fn await_clientname(&self) -> String {
    let receivedclientname = self.await_message().await;
    info!("got clientname: {:?}", receivedclientname);
    receivedclientname
  }

  // this function is just to await receiving a message from the client
  async fn await_message(&self) -> String {
    let mut buf = [0; 128];
    let mut packets = Vec::new();
    loop {
      self.socket.readable().await.unwrap();
      match self.socket.try_read(&mut buf) {
        Err(e) => match e.kind() {
          tokio::io::ErrorKind::WouldBlock => continue,
          _ => error!("failed to send message: {:?}", e),
        },
        _ => {}
      }

      let packet: Packet = buf.into();
      // println!("got packet: {:?}", packet);
      packets.push(packet.clone());
      buf = [0; 128];
      if packet.number == packet.total {
        break;
      }
    }
    let received = Message::from(packets);
    info!("got message: {:?}", received.get_msg());
    received.get_msg().to_string()
  }

  // this function is just to await sending a message to the client
  async fn await_send(&self, message: String) {
    let message = Message::new(message);
    loop {
      self.socket.writable().await.unwrap();
      let mut sent_packets = 0;
      for packet in message.get_packets().into_iter() {
        match self.socket.try_write(&packet.clone().to_vec()) {
          Ok(_) => {
            sent_packets = sent_packets + 1;
          }
          Err(e) => match e.kind() {
            tokio::io::ErrorKind::WouldBlock => continue,
            _ => error!("failed to send message: {:?}", e),
          },
        }
      }
      if sent_packets == message.get_packets().len() as u8 {
        break;
      } else {
        warn!("failed to send all packets");
      }
    }
  }
}
