#![no_std]

#[repr(packed)]
pub struct IPv4PacketLog {
    pub src: u32,
    pub dst: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub l3_protocol: u8,
    pub action: u32,
}

pub const ICMP_PROTOCOL: u8 = 0x01;
pub const UDP_PROTOCOL: u8 = 0x11;
pub const TCP_PROTOCOL: u8 = 0x06;

#[cfg(feature = "user")]
unsafe impl aya::Pod for IPv4PacketLog {}

