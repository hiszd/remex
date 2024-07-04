//SERVER
use remex_core::Message;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

const ADDRESS: &str = "127.0.0.1:4269";

static mut CLIENTS: Vec<String> = Vec::new();

const SECRET: &[u8] = "tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%2$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B".as_bytes();

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind(ADDRESS).await.unwrap();
    println!("listening on {}...", ADDRESS);

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            let secret = Message::new("tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%2$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B".to_string());
            process(stream, secret).await;
        });
    }
}

async fn process(mut socket: TcpStream, secret: Message) {
    println!("streaming");
    loop {
        socket.writable().await.unwrap();

        match socket.try_write(secret.packets[0].into()) {
            Ok(_) => {
                println!("sent secret");
                break;
            }
            Err(e) => println!("failed to send secret: {:?}", e),
        }
    }
    let mut read_buf: Vec<u8> = Vec::new();

    socket.read_to_end(&mut read_buf).await.unwrap();
    println!("{:?}", read_buf);
}
