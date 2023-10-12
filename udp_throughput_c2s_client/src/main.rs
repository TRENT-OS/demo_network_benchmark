use std::{
    io::Write,
    net::TcpStream,
    time::{Duration, Instant},
};

use argh::FromArgs;
use color_eyre::Result;
use rand::{RngCore, SeedableRng};

const DURATION: Duration = Duration::from_secs(10);
const BLOCK_SIZE: usize = 128 * 1024;

/// A tool to benchmark network applications.
#[derive(FromArgs)]
struct Args {
    /// target address and port, delimited by a colon
    #[argh(option, default = "String::from(\"10.0.0.10:5560\")")]
    address: String,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args: Args = argh::from_env();

    let mut rng = rand_xoshiro::Xoroshiro128Plus::from_entropy();
    let mut write_buf = vec![0; BLOCK_SIZE];
    rng.fill_bytes(&mut write_buf);

    let start_time = Instant::now();
    let mut stream = TcpStream::connect(&args.address)?;
    stream.set_nodelay(true)?;
    let mut bytes_transmitted = 0;
    while start_time.elapsed() < DURATION {
        stream.write(&write_buf)?;
        bytes_transmitted += BLOCK_SIZE;
    }

    let elapsed = start_time.elapsed();
    let bytes_per_second = ((bytes_transmitted as f64) / elapsed.as_secs_f64()) as u64;
    println!("{}/s", bytesize::to_string(bytes_per_second, true));

    Ok(())
}
