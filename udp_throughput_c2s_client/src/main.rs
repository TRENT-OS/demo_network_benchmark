/*
 * Copyright (C) 2023-2024, HENSOLDT Cyber GmbH
 * 
 * SPDX-License-Identifier: GPL-2.0-or-later
 *
 * For commercial licensing, contact: info.cyber@hensoldt.net
 */

use std::{
    io::Read,
    net::ToSocketAddrs,
    str::FromStr,
    time::{Duration, Instant},
};

use argh::FromArgs;
use bytesize::ByteSize;
use color_eyre::{eyre::eyre, Result};
use rand::{RngCore, SeedableRng};
use socket2::{Domain, Protocol, Socket, Type};

fn parse_duration(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s).map_err(|err| err.to_string())
}

fn parse_rate(full: &str) -> Result<u64, String> {
    let without_second = full.strip_suffix("/s").unwrap_or(full);
    if let Some(bits) = without_second.strip_suffix("bit") {
        Ok(ByteSize::from_str(bits)?.0 / 8)
    } else {
        Ok(ByteSize::from_str(without_second)?.0)
    }
}

/// A tool to benchmark network applications.
#[derive(FromArgs)]
struct Args {
    /// target address and port, delimited by a colon
    #[argh(positional)]
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
    /// approximately how many bits (`bit` suffix) or bytes (`B` suffix) should be sent per second
    #[argh(option, short = 'b', default = "1024 * 1024", from_str_fn(parse_rate))]
    rate: u64,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args: Args = argh::from_env();

    let mut rng = rand_xoshiro::Xoroshiro128Plus::from_entropy();
    let mut buf = vec![0; args.packet_size.0 as usize];

    let address = args
        .address
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| eyre!("Could not resolve address"))?
        .into();

    let mut socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.connect(&address)?;
    socket.set_read_timeout(Some(Duration::from_secs(10)))?;

    loop {
        // Reset received byte counter.
        while socket.send(&[2])? != 1 {
            // Repeat until actually sent.
        }
        let n = socket.read(&mut buf)?;
        let bytes_received = u64::from_le_bytes(buf[..n].try_into()?);
        if bytes_received == 0 {
            break;
        }
    }

    rng.fill_bytes(&mut buf);
    buf[0] = 0;

    socket.set_nonblocking(true)?;
    println!("Starting benchmark.");

    // Perform throughput measurements.
    let mut bytes_transmitted = 0;
    let mut total_send_duration = Duration::ZERO;
    let mut total_sends = 0;
    let time_start = Instant::now();
    loop {
        let transmit_start = Instant::now();
        match socket.send(&buf) {
            Ok(n) => {
                bytes_transmitted += n as u64;
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(err) => return Err(err.into()),
        };
        let now = Instant::now();
        total_send_duration += now.duration_since(transmit_start);
        total_sends += 1;

        // Calculating sleep duration.
        let time_per_send = total_send_duration / total_sends;
        let mut target_sends_per_second = args.rate / args.packet_size.0;
        if target_sends_per_second == 0 {
            target_sends_per_second = 1;
        }
        let sleep_time = (Duration::from_secs(1) / (target_sends_per_second as u32))
            .saturating_sub(time_per_send);
        if (now + sleep_time).duration_since(time_start) >= args.duration {
            break;
        }
        spin_sleep::sleep(sleep_time);
    }
    let time_end = Instant::now();

    socket.set_nonblocking(false)?;

    while socket.send(&[1])? != 1 {
        // Repeat until actually sent.
    }

    println!("Benchmark finished. Waiting for bytes-received response...");
    let n = socket.read(&mut buf)?;
    let bytes_received = u64::from_le_bytes(buf[..n].try_into()?);

    let loss_absolute = if bytes_transmitted >= bytes_received {
        bytesize::to_string(bytes_transmitted - bytes_received, true)
    } else {
        format!(
            "-{}",
            bytesize::to_string(bytes_received - bytes_transmitted, true)
        )
    };
    let loss_relative = 1.0 - (bytes_received as f64) / (bytes_transmitted as f64);
    println!(
        "Sent: {}, Received: {}, Loss: {} ({:.1}%)",
        bytesize::to_string(bytes_transmitted, true),
        bytesize::to_string(bytes_received, true),
        loss_absolute,
        loss_relative * 100.0,
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
    let bits_per_second = (((bytes * 8) as f64) / elapsed.as_secs_f64()) as u64;
    let mut s = bytesize::to_string(bits_per_second, false);
    s.pop();
    format!("{}bit/s", s)
}
