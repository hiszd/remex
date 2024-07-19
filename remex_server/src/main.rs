//SERVER
use remex_core::{Message, Packet};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, Duration};

mod args;

static mut LOGLEVEL: u8 = 0;

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

#[derive(Debug, Clone)]
enum Severity {
  INFO,
  WARNING,
  ERROR,
}

async fn log(severity: Severity, msg: String) {
  let mut file = OpenOptions::new().create(true).append(true).open("log.log").await.unwrap();
  let date = chrono::Local::now().format("%m-%d-%y %H:%M:%S");
  let mut log = String::new();
  match severity {
    Severity::WARNING => log.push_str("[WARNING] "),
    Severity::ERROR => log.push_str("  [ERROR] "),
    Severity::INFO => log.push_str("   [INFO] "),
  }
  log.push_str(date.to_string().as_str());
  log.push_str(" - ");
  log.push_str(msg.as_str());
  log.push_str("\n");
  file.write_all(log.as_bytes()).await.unwrap();
  print!("{}", log);
}

const ADDRESS: &str = "127.0.0.1:4269";

const SECRET: &str = "tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%2$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B";

#[tokio::main]
async fn main() {
  let args = args::cli().get_matches();
  let listener = TcpListener::bind(ADDRESS).await.unwrap();
  log(Severity::INFO, format!("listening on {}...", ADDRESS)).await;

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
      log(Severity::INFO, format!("secret received and verified")).await;
      write_message(&socket, "hello".to_string()).await;
      sleep(Duration::from_millis(100)).await;
      write_message(&socket, "zion".to_string()).await;
    }
    Err(e) => {
      log(Severity::ERROR, format!("Secret verification failed. Reason: {}", String::from(e)))
        .await;
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
      Err(e) => log(Severity::ERROR, format!("failed to send message: {:?}", e)).await,
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
        Err(e) => log(Severity::ERROR, format!("failed to send secret: {:?}", e)).await,
      }
    }

    if sent_packets == secret.get_packets().len() as u8 {
      break;
    } else {
      log(Severity::WARNING, format!("failed to send all packets")).await;
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
  log(Severity::INFO, format!("got secret: {:?}", receivedsecret.get_msg())).await;
  match secret.get_msg() == receivedsecret.get_msg() {
    true => Ok(()),
    false => Err(ERROR::InvalidSecret),
  }
}
