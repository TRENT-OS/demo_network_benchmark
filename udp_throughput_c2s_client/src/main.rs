use std::{
    net::UdpSocket,
    time::{Duration, Instant},
};

use argh::FromArgs;
use color_eyre::Result;
use rand::{RngCore, SeedableRng};

fn parse_duration(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s).map_err(|err| err.to_string())
}

/// A tool to benchmark network applications.
#[derive(FromArgs)]
struct Args {
    /// target address and port, delimited by a colon
    #[argh(option, default = "String::from(\"10.0.0.10:5560\")")]
    address: String,
    /// the targeted overall duration
    #[argh(
        option,
        default = "Duration::from_secs(10)",
        from_str_fn(parse_duration)
    )]
    duration: Duration,
    /// how large each packet should be
    #[argh(option, default = "1472")]
    packet_size: usize,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args: Args = argh::from_env();

    let mut rng = rand_xoshiro::Xoroshiro128Plus::from_entropy();
    let mut buf = vec![0; args.packet_size];

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(args.address)?;

    loop {
        // Reset received byte counter.
        while socket.send(&[2])? != 1 {
            // Repeat until actually sent.
        }
        let n = socket.recv(&mut buf)?;
        let bytes_received = u64::from_le_bytes(buf[..n].try_into()?);
        if bytes_received == 0 {
            break;
        }
    }

    rng.fill_bytes(&mut buf);
    buf[0] = 0;

    let time_start = Instant::now();
    let mut bytes_transmitted = 0;
    while time_start.elapsed() < args.duration {
        bytes_transmitted += socket.send(&buf)? as u64;
    }
    let time_end = Instant::now();

    while socket.send(&[1])? != 1 {
        // Repeat until actually sent.
    }

    let n = socket.recv(&mut buf)?;
    let bytes_received = u64::from_le_bytes(buf[..n].try_into()?);

    let loss = 1.0 - (bytes_received as f64) / (bytes_transmitted as f64);
    println!(
        "Sent: {}, Received: {}, Loss: {:.1}%",
        bytesize::to_string(bytes_transmitted, true),
        bytesize::to_string(bytes_received, true),
        loss * 100.0
    );
    println!(
        "Throughput: {}",
        format_throughput(time_end.duration_since(time_start), bytes_received)
    );

    Ok(())
}

fn format_throughput(elapsed: Duration, bytes: u64) -> String {
    let bytes_per_second = ((bytes as f64) / elapsed.as_secs_f64()) as u64;
    format!("{}/s", bytesize::to_string(bytes_per_second, true))
}
