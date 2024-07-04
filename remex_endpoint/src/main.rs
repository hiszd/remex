//ENDPOINT
use std::io::Write;
use std::net::TcpStream;

const ADDRESS: &str = "127.0.0.1:4269";

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect(ADDRESS)?;

    stream.write(&[1])?;
    Ok(())
}
