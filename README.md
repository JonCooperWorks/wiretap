# bpfwall
`bpfwall` is a simple eBPF firewall that uses [`aya`](https://crates.io/crates/aya) to create the eBPF program and loader in Rust.
It is meant to help me learn eBPF and Rust and should not be used in a production environment.

## Prerequisites
This was done on Ubuntu 21.04 on a DigitalOcean droplet with 2GB RAM.
Building `cargo-generate` consistently crashed `cargo` with an OOM on smaller machines.

### Setup
First, install dependencies with the following commands:

```
# First update package lists and packages.
sudo apt update
sudo apt upgrade

# Then install the packages needed to compile bpf modules
sudo apt install -y build-essential llvm-12-dev libclang-12-dev zlib1g-dev libssl-dev pkg-config linux-cloud-tools-generic linux-tools-generic

# Install Rust
curl https://sh.rustup.rs -sSf | sh

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
The generated bindings will be written out to `bpfwall-ebpf/bindings.rs` by default. 

```bash
cargo xtask codegen --names iphdr ethhdr
```

You can specify a eBPF directory and vmlinux path with the `--bpf-directory` and `--vmlinux-path`  flags.

```bash
cargo xtask codegen --names iphdr ethhdr --bpf-directory bpfwall-ebpf --vmlinux-path /sys/kernel/btf/vmlinux
```

You can change 



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

```bash
cargo run --package bpfwall --bin bpfwall
```


