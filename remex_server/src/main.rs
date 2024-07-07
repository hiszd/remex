//SERVER
use remex_core::Message;
use tokio::io::AsyncReadExt;
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

async fn process(mut socket: TcpStream, secret: Message) {
    println!("streaming");
    loop {
        socket.writable().await.unwrap();

        let mut sent_packets = 0;

        for packet in secret.packets.clone().into_iter() {
            match socket.try_write(&packet.clone().to_vec()) {
                Ok(_) => {
                    sent_packets = sent_packets + 1;
                    println!("sent {} packets", sent_packets);
                }
                Err(e) => println!("failed to send secret: {:?}", e),
            }
        }

        if sent_packets == secret.packets.len() as u8 {
            break;
        }
    }

    let mut read_buf: Vec<u8> = Vec::new();

    socket.read_to_end(&mut read_buf).await.unwrap();
    println!("{:?}", read_buf);
}
