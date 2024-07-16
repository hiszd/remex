//ENDPOINT
use remex_core::{Message, Packet};
use tokio::net::TcpStream;

const ADDRESS: &str = "127.0.0.1:4269";

const SECRET: &str = "tzs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%2$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B";

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
  await_secret(socket, secret).await;
  println!("exited loop");
  println!("secret received and verified");
  await_messages(socket).await;
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
              println!("got all packets");
              break;
            }
          }
        }
        Err(_) => continue,
      }
    }

    // packets.iter().for_each(|x| println!("{}, {}", x.number, x.total));
    let received = Message::from(packets);
    println!("got {:?}", received.get_msg());
  }
}

async fn await_secret(socket: &TcpStream, secret: Message) {
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
    println!("got secret: {:?}", receivedsecret.get_msg());
    assert_eq!(secret.get_msg(), receivedsecret.get_msg());
  }

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
    }
  }
}
