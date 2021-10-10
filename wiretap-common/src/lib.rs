#![no_std]

#[repr(packed)]
pub struct PacketLog {
    pub src: u128,
    pub dst: u128,
    pub src_port: u16,
    pub dst_port: u16,
    pub l3_protocol: u8,
    pub action: u32,
    pub is_ipv4: bool,
}

pub struct PacketLogWrapper {
    pub data: *const PacketLog,
}
unsafe impl Send for PacketLogWrapper {}

#[cfg(feature = "user")]
unsafe impl aya::Pod for PacketLog {}

pub const ICMP_PROTOCOL: u8 = 0x01;
pub const UDP_PROTOCOL: u8 = 0x11;
pub const TCP_PROTOCOL: u8 = 0x06;
