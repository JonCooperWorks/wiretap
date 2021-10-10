# wiretap
`wiretap` is a simple eBPF packet flow logger that uses [`aya`](https://crates.io/crates/aya) to create the eBPF program and loader in Rust.
`wiretap` will take flow logs from an interface and store them to AWS S3 compatible cloud storage.
The S3 credentials should be set as environment variables with the following names:

```
AWS_ACCESS_KEY_ID
AWS_SECRET_ACCESS_KEY
```

`wiretap` is meant to help me learn eBPF and Rust and should not be used in a production environment.

## Prerequisites
This was done on Ubuntu 21.04 on a DigitalOcean droplet with 2GB RAM.
Building `cargo-generate` consistently crashed `cargo` with an OOM on smaller machines.

### Setup

#### Ubuntu 21.04
First, install dependencies with the following commands:

```
# First update package lists and packages.
sudo apt-get update
sudo apt-get upgrade

# Then install the packages needed to compile bpf modules
sudo apt-get install -y sudo build-essential llvm-12-dev libclang-12-dev zlib1g-dev libssl-dev pkg-config linux-cloud-tools-generic linux-tools-$(uname -r) linux-cloud-tools-$(uname -r) linux-tools-generic curl


# Install Rust
curl https://sh.rustup.rs -sSf | sh  -s -- -y

# Use rustup to install stable and nightly rust toolchains
rustup install stable
rustup toolchain install nightly --component rust-src

# Install bpf-linker to to link eBPF programs
cargo +nightly install bpf-linker

# Install cargo-generate to generate aya project skeleton.
# There are multiple binaries in this crate so specify cargo-generate
cargo install --git https://github.com/cargo-generate/cargo-generate cargo-generate
```

### Generating `vmlinux` Bindings
This package requires Rust `vmlinux.h` bindings.
Use the `codegen` build task to do this.
It will look in `/sys/kernel/btf/vmlinux` by default for the vmlinux file.
The generated bindings will be written out to `wiretap-ebpf/bindings.rs` by default.
`wiretap` needs the bindings for `iphdr`, `ethhdr`, `tcphdr` and `udphdr` to inspect Ethernet and IP packets before they go on the wire.

```bash
cargo xtask codegen --names iphdr ethhdr tcphdr udphdr
```

You can specify a eBPF directory and vmlinux path with the `--bpf-directory` and `--vmlinux-path`  flags.

```bash
cargo xtask codegen --names iphdr ethhdr tcphdr udphdr --bpf-directory wiretap-ebpf --vmlinux-path /sys/kernel/btf/vmlinux
```

## Build eBPF

```bash
cargo xtask build-ebpf
```

To perform a release build you can use the `--release` flag.
You may also change the target architecture with the `--target` flag.

## Build Userspace

```bash
cargo build
```

## Run

`wiretap` can be configured to send flow logs for a particular interface to [S3](https://aws.amazon.com/s3/) compatible storage.
By default, it will log from `eth0`, but can be made to listen to any interface with the `--iface` flag.
Pass the XDP `wiretap` program using the `--path` flag.

### S3 Storage
`wiretap` expects AWS credentials to be passed the environment variables:

- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`

Pass the bucket, endpoint and region for S3 compatible storage using the `--storage-bucket`, `--storage-endpoint` and `--storage-region` flags.


```bash
AWS_ACCESS_KEY_ID=AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY
=AWS_SECRET_KEY cargo run --bin wiretap -- --iface eth0 --path target/bpfel-unknown-none/debug/wiretap --storage-bucket bucket-name --storage-endpoint https://s3-storage-endpoint --storage-region s3-region
```
