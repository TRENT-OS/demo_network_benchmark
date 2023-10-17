use std::{
    io::Write,
    net::ToSocketAddrs,
    time::{Duration, Instant},
};

use argh::FromArgs;
use color_eyre::{eyre::eyre, Result};
use rand::{RngCore, SeedableRng};
use socket2::{Domain, Protocol, Socket, Type};

fn parse_duration(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s).map_err(|err| err.to_string())
}

/// A tool to benchmark network applications.
#[derive(FromArgs)]
struct Args {
    /// target address and port, delimited by a colon
    #[argh(positional, default = "String::from(\"10.0.0.10:5560\")")]
    address: String,
    /// the targeted per-transmission duration
    #[argh(
        option,
        default = "Duration::from_secs(10)",
        from_str_fn(parse_duration)
    )]
    duration: Duration,
    /// the number of transmissions
    #[argh(option, default = "5")]
    sample_size: u32,
    /// how large each block should be
    #[argh(option, default = "131072")]
    block_size: usize,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args: Args = argh::from_env();

    let mut total_bytes_transmitted = 0;
    let mut total_overall_duration = Duration::ZERO;
    let mut total_pure_duration = Duration::ZERO;

    let mut rng = rand_xoshiro::Xoroshiro128Plus::from_entropy();

    let address = args
        .address
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| eyre!("Could not resolve address"))?
        .into();

    println!("Sample | Overall Throughput | Pure Throughput");
    println!("-------|--------------------|----------------");
    for i in 0..args.sample_size {
        let mut write_buf = vec![0; args.block_size];
        rng.fill_bytes(&mut write_buf);

        let time_connect = Instant::now();
        let mut socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        socket.connect(&address)?;
        socket.set_nonblocking(true)?;
        let time_send = Instant::now();
        let mut bytes_transmitted = 0;
        while time_send.elapsed() < args.duration {
            let mut i = 0;
            while i < 10 {
                match socket.write(&write_buf) {
                    Ok(n) => {
                        bytes_transmitted += n as u64;
                        i += 1;
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(err) => return Err(err.into()),
                };
            }
        }
        let time_send_end = Instant::now();

        drop(socket);
        let time_close = Instant::now();

        let overall_duration = time_close.duration_since(time_connect);
        let pure_duration = time_send_end.duration_since(time_send);
        total_bytes_transmitted += bytes_transmitted;
        total_overall_duration += overall_duration;
        total_pure_duration += pure_duration;

        let overall_throughput = format_throughput(overall_duration, bytes_transmitted);
        let pure_throughput = format_throughput(pure_duration, bytes_transmitted);
        println!("{i:>6} | {overall_throughput:>18} | {pure_throughput:>15}");
    }

    let mean_overall_throughput =
        format_throughput(total_overall_duration, total_bytes_transmitted);
    let mean_pure_throughput = format_throughput(total_pure_duration, total_bytes_transmitted);
    println!("-------|--------------------|----------------");
    println!("  Mean | {mean_overall_throughput:>18} | {mean_pure_throughput:>15}");

    Ok(())
}

fn format_throughput(elapsed: Duration, bytes: u64) -> String {
    let bits_per_second = (((bytes * 8) as f64) / elapsed.as_secs_f64()) as u64;
    let mut s = bytesize::to_string(bits_per_second, false);
    s.pop();
    format!("{}b/s", s)
}
