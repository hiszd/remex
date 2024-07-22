//SERVER
use remex_core::{Message, Packet};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use tracing_subscriber;

mod args;

#[derive(Debug, Clone)]
pub enum ERROR {
  InvalidSecret,
  InvalidPacket,
  InvalidLength,
  NotConnected,
  NotEnoughPackets,
}

impl From<ERROR> for String {
  fn from(value: ERROR) -> Self {
    match value {
      ERROR::InvalidSecret => "invalid secret".to_string(),
      ERROR::InvalidPacket => "invalid packet".to_string(),
      ERROR::InvalidLength => "invalid length".to_string(),
      ERROR::NotConnected => "not connected".to_string(),
      ERROR::NotEnoughPackets => "not enough packets".to_string(),
    }
  }
}

const ADDRESS: &str = "127.0.0.1:4269";

const SECRET: &str = "tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%2$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B";

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt::init();
  let listener = TcpListener::bind(ADDRESS).await.unwrap();

  loop {
    let (stream, _) = listener.accept().await.unwrap();
    tokio::spawn(async move {
      let secret = Message::new(SECRET.to_string());
      process(stream, secret).await;
    });
  }
}

async fn process(socket: TcpStream, secret: Message) {
  match await_secret(&socket, secret).await {
    Ok(_) => {
      info!("secret received and verified");
      write_message(&socket, "hello".to_string()).await;
      sleep(Duration::from_millis(100)).await;
      write_message(&socket, "zion".to_string()).await;
    }
    Err(e) => {
      error!("Secret verification failed. Reason: {}", String::from(e));
      return;
    }
  }
}

async fn write_message(socket: &TcpStream, mg: String) {
  let msg = Message::new(mg);
  for packet in msg.get_packets().into_iter() {
    socket.writable().await.unwrap();
    match socket.try_write(&packet.clone().to_vec()) {
      Ok(_) => {}
      Err(e) => error!("failed to send message: {:?}", e),
    }
  }
}

async fn await_secret(socket: &TcpStream, secret: Message) -> Result<(), ERROR> {
  loop {
    socket.writable().await.unwrap();

    let mut sent_packets = 0;

    for packet in secret.get_packets().into_iter() {
      match socket.try_write(&packet.clone().to_vec()) {
        Ok(_) => {
          sent_packets = sent_packets + 1;
        }
        Err(e) => error!("failed to send secret: {:?}", e),
      }
    }

    if sent_packets == secret.get_packets().len() as u8 {
      break;
    } else {
      warn!("failed to send all packets");
    }
  }

  let mut read_buf = [0; 128];

  let mut packets = Vec::new();
  loop {
    socket.readable().await.unwrap();
    socket.try_read(&mut read_buf).unwrap();

    let packet: Packet = read_buf.into();
    // println!("got packet: {:?}", packet);
    packets.push(packet.clone());
    read_buf = [0; 128];
    if packet.number == packet.total {
      break;
    }
  }
  let receivedsecret = Message::from(packets);
  info!("got secret: {:?}", receivedsecret.get_msg());
  match secret.get_msg() == receivedsecret.get_msg() {
    true => Ok(()),
    false => Err(ERROR::InvalidSecret),
  }
}
