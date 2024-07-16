//SERVER
use remex_core::{Message, Packet};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const ADDRESS: &str = "127.0.0.1:4269";

static mut CLIENTS: Vec<String> = Vec::new();

const SECRET: &str = "tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%2$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B";

#[tokio::main]
async fn main() {
  let listener = TcpListener::bind(ADDRESS).await.unwrap();
  println!("listening on {}...", ADDRESS);

  loop {
    let (stream, _) = listener.accept().await.unwrap();
    tokio::spawn(async move {
      let secret = Message::new(SECRET.to_string());
      process(stream, secret).await;
    });
  }
}

async fn process(socket: TcpStream, secret: Message) {
  await_secret(&socket, secret).await;
  println!("secret received and verified");
  write_message(&socket, "hello".to_string()).await;
  write_message(&socket, "zion".to_string()).await;
}

async fn write_message(socket: &TcpStream, mg: String) {
  let msg = Message::new(mg);
  for packet in msg.get_packets().into_iter() {
    socket.writable().await.unwrap();
    match socket.try_write(&packet.clone().to_vec()) {
      Ok(_) => unsafe {
        MSG_SENT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        println!("sent message {}", MSG_SENT.load(std::sync::atomic::Ordering::Relaxed));
      },
      Err(e) => println!("failed to send message: {:?}", e),
    }
  }
}

async fn await_secret(socket: &TcpStream, secret: Message) {
  loop {
    socket.writable().await.unwrap();

    let mut sent_packets = 0;

    for packet in secret.get_packets().into_iter() {
      match socket.try_write(&packet.clone().to_vec()) {
        Ok(_) => {
          sent_packets = sent_packets + 1;
        }
        Err(e) => println!("failed to send secret: {:?}", e),
      }
    }

    if sent_packets == secret.get_packets().len() as u8 {
      break;
    } else {
      println!("failed to send all packets");
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
  println!("got secret: {:?}", receivedsecret.get_msg());
  assert_eq!(secret.get_msg(), receivedsecret.get_msg());
}
