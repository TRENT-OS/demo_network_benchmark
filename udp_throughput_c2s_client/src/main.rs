use std::{
    collections::VecDeque,
    net::UdpSocket,
    time::{Duration, Instant},
};

use argh::FromArgs;
use bytesize::ByteSize;
use color_eyre::Result;
use rand::{RngCore, SeedableRng};

fn parse_duration(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s).map_err(|err| err.to_string())
}

/// A tool to benchmark network applications.
#[derive(FromArgs)]
struct Args {
    /// target address and port, delimited by a colon
    #[argh(positional, default = "String::from(\"10.0.0.10:5560\")")]
    address: String,
    /// the targeted overall duration
    #[argh(
        option,
        default = "Duration::from_secs(10)",
        from_str_fn(parse_duration)
    )]
    duration: Duration,
    /// how large each packet should be
    #[argh(option, short = 'n', default = "ByteSize::b(1472)")]
    packet_size: ByteSize,
    /// approximately how many bytes should be sent per second
    #[argh(option, short = 'b', default = "ByteSize::kb(100)")]
    bytes_per_second: ByteSize,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args: Args = argh::from_env();

    let mut rng = rand_xoshiro::Xoroshiro128Plus::from_entropy();
    let mut buf = vec![0; args.packet_size.0 as usize];

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

    // Perform throughput measurements.
    let mut bytes_transmitted = 0;
    let mut sends = VecDeque::new();
    let mut total_send_duration = Duration::ZERO;
    let mut total_sends = 0;
    let time_start = Instant::now();
    loop {
        let transmit_start = Instant::now();
        let n = socket.send(&buf)? as u64;
        let now = Instant::now();
        bytes_transmitted += n;
        total_send_duration += now.duration_since(transmit_start);
        total_sends += 1;

        sends.push_back((transmit_start, n, now));
        while let Some(&(transmit_start, _, _)) = sends.front() {
            if now.duration_since(transmit_start) > Duration::from_secs(1) {
                sends.pop_front();
            } else {
                break;
            }
        }
        let time_per_send = total_send_duration / total_sends;
        let target_sends_per_second = args.bytes_per_second.0 / args.packet_size.0;
        let sleep_time = (Duration::from_secs(1) / (target_sends_per_second as u32))
            .saturating_sub(time_per_send);
        if (now + sleep_time).duration_since(time_start) >= args.duration {
            break;
        }
        spin_sleep::sleep(sleep_time);
    }
    let time_end = Instant::now();

    while socket.send(&[1])? != 1 {
        // Repeat until actually sent.
    }

    println!("Waiting for bytes-received response...");
    let n = socket.recv(&mut buf)?;
    let bytes_received = u64::from_le_bytes(buf[..n].try_into()?);

    let loss = 1.0 - (bytes_received as f64) / (bytes_transmitted as f64);
    println!(
        "Sent: {}, Received: {}, Loss: {:.1}%",
        bytesize::to_string(bytes_transmitted, true),
        bytesize::to_string(bytes_received, true),
        loss * 100.0,
    );
    let duration = time_end.duration_since(time_start);
    println!(
        "Sending Throughput: {}, Receiving Throughput: {}",
        format_throughput(duration, bytes_transmitted),
        format_throughput(duration, bytes_received),
    );

    Ok(())
}

fn format_throughput(elapsed: Duration, bytes: u64) -> String {
    let bytes_per_second = ((bytes as f64) / elapsed.as_secs_f64()) as u64;
    format!("{}/s", bytesize::to_string(bytes_per_second, true))
}
