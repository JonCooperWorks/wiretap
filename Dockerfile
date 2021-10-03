FROM ubuntu:21.04
ARG TIMEZONE
RUN apt-get update && apt-get upgrade -y
RUN DEBIAN_FRONTEND="noninteractive" TZ=${TIMZEONE} apt-get install -y sudo build-essential llvm-12-dev libclang-12-dev zlib1g-dev libssl-dev pkg-config linux-cloud-tools-generic linux-tools-generic curl
RUN useradd -m docker && echo "docker:docker" | chpasswd && adduser docker sudo
USER cargo
RUN curl https://sh.rustup.rs -sSf | sh  -s -- -y
ENV PATH="/home/cargo/.cargo/bin:${PATH}"
RUN rustup install stable
RUN rustup toolchain install nightly --component rust-src
RUN cargo +nightly install bpf-linker
RUN cargo install --git https://github.com/cargo-generate/cargo-generate cargo-generate
CMD ["bash"]