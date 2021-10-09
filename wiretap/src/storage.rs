use std::net::IpAddr;
use std::time::Duration;

pub struct Config {
    pub max_packets_per_log: usize,
    pub packet_log_interval: Duration,
}

pub struct IPv4FlowLog {
    pub src: IpAddr,
    pub dst: IpAddr,
    pub src_port: u16,
    pub dst_port: u16,
    pub l3_protocol: u8,
    pub action: u32,
    pub timestamp: u64,
}

unsafe impl Send for IPv4FlowLog {}