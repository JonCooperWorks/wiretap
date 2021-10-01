#![no_std]

#[repr(C)]
pub struct IPv4PacketLog {
    pub src: u32,
    pub dst: u32,
    pub action: u32,
}

#[repr(C)]
pub struct IPv6PacketLog {
    pub src: u64,
    pub dst: u64,
    pub action: u32,
}

#[repr(C)]
pub enum PacketLog {
    V4(IPv4PacketLog),
    V6(IPv6PacketLog),
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for PacketLog {}

