#![no_std]

#[repr(C)]
pub struct IPv4PacketLog {
    pub address: u32,
    pub action: u32,
}

#[repr(C)]
pub struct IPv6PacketLog {
    pub address: u64,
    pub action: u32,
}

#[repr(C)]
pub enum PacketLog {
    V4(IPv4PacketLog),
    V6(IPv6PacketLog),
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for PacketLog {}

