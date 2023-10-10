use std::{
    io::{Read, Write},
    net::TcpListener,
};

use color_eyre::Result;

const COOKIE_SIZE: usize = 37;
const TCP_BLKSIZE: usize = 128 * 1024;

fn main() -> Result<()> {
    color_eyre::install()?;

    let listener = TcpListener::bind("127.0.0.1:8888")?;
    let (mut stream, _) = listener.accept()?;

    {
        let mut cookie_buf = [0; COOKIE_SIZE];
        stream.read_exact(&mut cookie_buf)?;
        println!("{}", std::str::from_utf8(&cookie_buf)?);
    }

    {
        let mut buf = vec![0; TCP_BLKSIZE];
        while let Ok(n) = stream.read(&mut buf) {
            stream.write(&buf[..n])?;
        }
    }

    Ok(())
}
