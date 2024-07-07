//ENDPOINT
use remex_core::{Message, Packet};
use std::io::{Read, Write};
use std::net::TcpStream;

const ADDRESS: &str = "127.0.0.1:4269";

const SECRET: &str = "tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%2$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B";

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect(ADDRESS)?;

    let mut buf = [0; 128];

    println!("reading");
    let mut packets = Vec::new();
    loop {
        stream.read_exact(&mut buf).unwrap();
        let packet: Packet = buf.into();
        println!("got packet: {:?}", packet);
        packets.push(packet.clone());
        buf = [0; 128];
        if packet.number == packet.total {
            break;
        }
    }
    println!("exited loop");

    let str = std::str::from_utf8(&buf).unwrap();
    if str != SECRET {
        println!("Wrong secret: {}", str);
        return Ok(());
    }

    stream.write(&[1])?;
    Ok(())
}
