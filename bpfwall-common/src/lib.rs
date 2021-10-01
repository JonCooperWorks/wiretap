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

#[repr(C)]
pub struct IPv6PacketLog {
    pub src: u64,
    pub dst: u64,
    pub src_port: u16,
    pub dst_port: u16,
    pub action: u32,
}

#[repr(C)]
pub enum PacketLog {
    V4(IPv4PacketLog),
    V6(IPv6PacketLog),
}

pub const UDP_PROTOCOL: u8 = 0x11;
pub const TCP_PROTOCOL: u8 = 0x06;

pub fn l3_protocol(protocol: u8) -> &'static str {
    match protocol {
        UDP_PROTOCOL => "UDP",
        TCP_PROTOCOL => "TCP",
        _ => "TODO",
    }
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for PacketLog {}

