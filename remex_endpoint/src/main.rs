//ENDPOINT
use remex_core::{Message, Packet};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

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
pub enum Severity {
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
  tokio::spawn(async move {
    let stream = TcpStream::connect(ADDRESS).await.unwrap();
    let secret = Message::new(SECRET.to_string());
    process(&stream, secret).await;
  })
  .await
  .unwrap();
}

async fn process(socket: &TcpStream, secret: Message) {
  match await_secret(socket, secret).await {
    Ok(_) => {
      log(Severity::INFO, "secret received and verified".to_string()).await;
      await_messages(socket).await;
    }
    Err(e) => {
      log(Severity::ERROR, format!("Secret verification failed. Reason: {}", String::from(e)))
        .await;
      return;
    }
  }
}

async fn await_messages(socket: &TcpStream) {
  loop {
    let mut buf = [0; 128];
    let mut packets = Vec::new();
    loop {
      socket.readable().await.unwrap();
      match socket.try_read(&mut buf) {
        Ok(x) => {
          if x != 0 {
            let packet: Packet = buf.into();
            // println!("got packet: {:?}", packet);
            packets.push(packet.clone());
            buf = [0; 128];
            if packet.number == packet.total {
              log(Severity::INFO, "got all packets".to_string()).await;
              break;
            }
          }
        }
        Err(_) => continue,
      }
    }

    // packets.iter().for_each(|x| println!("{}, {}", x.number, x.total));
    let received = Message::from(packets);
    log(Severity::INFO, format!("got {:?}", received.get_msg())).await;
  }
}

async fn await_secret(socket: &TcpStream, secret: Message) -> Result<(), ERROR> {
  {
    let mut buf = [0; 128];
    let mut packets = Vec::new();
    loop {
      socket.readable().await.unwrap();
      socket.try_read(&mut buf).unwrap();
      let packet: Packet = buf.into();
      packets.push(packet.clone());
      buf = [0; 128];
      if packet.number == packet.total {
        break;
      }
    }
    let receivedsecret = Message::from(packets);
    log(Severity::INFO, format!("got secret: {:?}", receivedsecret.get_msg())).await;
    match secret.get_msg() == receivedsecret.get_msg() {
      false => return Err(ERROR::InvalidSecret),
      _ => {}
    }
  }

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
    }
  }
  Ok(())
}
