#![no_std]
#![no_main]

use aya_bpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::PerfMap,
    programs::XdpContext,
};

use core::mem;
use memoffset::offset_of;
use bpfwall_common::{IPv4PacketLog, UDP_PROTOCOL, TCP_PROTOCOL};

// ANCHOR: bindings
mod bindings;
use bindings::{ethhdr, iphdr, tcphdr, udphdr};
// ANCHOR_END: bindings

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unreachable!()
}

// ANCHOR: map
#[map(name = "EVENTS")]
static mut EVENTS: PerfMap<IPv4PacketLog> = PerfMap::<IPv4PacketLog>::with_max_entries(1024, 0);
// ANCHOR_END: map

#[xdp(name="bpfwall")]
pub fn xdp_firewall(ctx: XdpContext) -> u32 {
    match try_xdp_firewall(ctx) {
        Ok(ret) => ret,
        Err(_) => xdp_action::XDP_ABORTED,
    }
}

// ANCHOR: ptr_at
#[inline(always)]
unsafe fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *const T)
}
// ANCHOR_END: ptr_at

// ANCHOR: try
fn try_xdp_firewall(ctx: XdpContext) -> Result<u32, ()> {
    let h_proto = u16::from_be(unsafe { *ptr_at(&ctx, offset_of!(ethhdr, h_proto))? });
    if h_proto != ETH_P_IP {
        return Ok(xdp_action::XDP_PASS);
    }
    
    let source = u32::from_be(unsafe { *ptr_at(&ctx, ETH_HDR_LEN + offset_of!(iphdr, saddr))? });
    let dest = u32::from_be(unsafe { *ptr_at(&ctx, ETH_HDR_LEN + offset_of!(iphdr, daddr))? });
    let l3_protocol = u8::from_be(unsafe { *ptr_at(&ctx, ETH_HDR_LEN + offset_of!(iphdr, protocol))? });

    let (src_port, dst_port) = match l3_protocol {
        UDP_PROTOCOL => {
            let src_port = u16::from_be(unsafe { *ptr_at(&ctx, IP_HDR_LEN + offset_of!(udphdr, source))? });
            let dst_port = u16::from_be(unsafe { *ptr_at(&ctx, IP_HDR_LEN + offset_of!(udphdr, dest))? });
            (src_port, dst_port)
        }

        TCP_PROTOCOL => {
            let src_port = u16::from_be(unsafe { *ptr_at(&ctx, IP_HDR_LEN + offset_of!(tcphdr, source))? });
            let dst_port = u16::from_be(unsafe { *ptr_at(&ctx, IP_HDR_LEN + offset_of!(tcphdr, dest))? });
            (src_port, dst_port)
        }

        _ => {
            (0x0000, 0x0000)
        }
    };

    let log_entry = IPv4PacketLog {
        src: source,
        dst: dest,
        src_port: src_port,
        dst_port: dst_port,
        l3_protocol: l3_protocol,
        action: xdp_action::XDP_PASS,
    };

    unsafe {
        EVENTS.output(&ctx, &log_entry, 0);
    }
    Ok(xdp_action::XDP_PASS)
}
// ANCHOR_END: try

const ETH_P_IP: u16 = 0x0800;
const ETH_HDR_LEN: usize = mem::size_of::<ethhdr>();
const IP_HDR_LEN: usize = mem::size_of::<iphdr>() + ETH_HDR_LEN;