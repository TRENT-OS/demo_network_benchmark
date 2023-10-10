use std::{
    io::{Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};

use color_eyre::Result;
use rand::{RngCore, SeedableRng};

const DURATION: Duration = Duration::from_secs(10);
const BLOCK_SIZE: usize = 128 * 1024;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut rng = rand_xoshiro::Xoroshiro128Plus::from_entropy();
    let mut write_buf = vec![0; BLOCK_SIZE];
    rng.fill_bytes(&mut write_buf);
    let mut read_buf = vec![0; BLOCK_SIZE];

    let start_time = Instant::now();
    let mut stream = TcpStream::connect("127.0.0.1:8888")?;
    stream.set_nodelay(true)?;
    let mut bytes_transmitted = 0;
    while start_time.elapsed() < DURATION {
        stream.write(&write_buf)?;
        // stream.read_exact(&mut read_buf)?;
        bytes_transmitted += BLOCK_SIZE;
    }

    let elapsed = start_time.elapsed();
    let bytes_per_second = ((bytes_transmitted as f64) / elapsed.as_secs_f64()) as u64;
    println!("{}/s", bytesize::to_string(bytes_per_second, true));

    Ok(())
}
