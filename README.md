# Demo Network Benchmark
Tools for benchmarking TRENTOS networking.

## Requirements
The host system needs a recent version of the cargo package manager installed.

## Usage
First build and run the correct TRENTOS component.
```sh
# One of
./build.sh tcp_throughput_c2s_server
./build.sh tcp_throughput_s2c_server
./build.sh udp_throughput_c2s_server
```

Then, run the corresponding benchmark:
```sh
cargo run --release -p tcp_throughput_c2s_client -- --help
```
