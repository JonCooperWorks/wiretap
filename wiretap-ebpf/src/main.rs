#![no_std]
#![no_main]

use aya_bpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::{HashMap, PerfMap},
    programs::XdpContext,
};

use core::mem;
use memoffset::offset_of;
use wiretap_common::{PacketLog, UDP_PROTOCOL, TCP_PROTOCOL};

mod bindings;
use bindings::{ethhdr, iphdr, tcphdr, udphdr, ipv6hdr};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unreachable!()
}

#[map(name = "PACKETS")]
static mut PACKETS: PerfMap<PacketLog> = PerfMap::<PacketLog>::with_max_entries(1024, 0);

#[map(name = "OUTBOUND_BLOCKLIST")]
static mut OUTBOUND_BLOCKLIST: HashMap<u32, u8> = HashMap::<u32, u8>::with_max_entries(1024, 0);

#[xdp(name="wiretap")]
pub fn xdp_firewall(ctx: XdpContext) -> u32 {
    match try_xdp_firewall(ctx) {
        Ok(ret) => xdp_action::XDP_PASS,
        Err(_) => xdp_action::XDP_ABORTED,
    }
}

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

fn try_xdp_firewall(ctx: XdpContext) -> Result<u32, ()> {
    let h_proto = u16::from_be(unsafe { *ptr_at(&ctx, offset_of!(ethhdr, h_proto))? });

    match h_proto {
        ETH_P_IPV6 => {
            let source = u128::from_be(unsafe { *ptr_at(&ctx, ETH_HDR_LEN + offset_of!(ipv6hdr, saddr))? });
            let dest = u128::from_be(unsafe { *ptr_at(&ctx, ETH_HDR_LEN + offset_of!(ipv6hdr, daddr))? });
            let l3_protocol = u8::from_be(unsafe { *ptr_at(&ctx, ETH_HDR_LEN + offset_of!(ipv6hdr, nexthdr))? });
        
            let (src_port, dst_port) = match l3_protocol {
                UDP_PROTOCOL => {
                    let src_port = u16::from_be(unsafe { *ptr_at(&ctx, IPV6_HDR_LEN + offset_of!(udphdr, source))? });
                    let dst_port = u16::from_be(unsafe { *ptr_at(&ctx, IPV6_HDR_LEN + offset_of!(udphdr, dest))? });
                    (src_port, dst_port)
                }
        
                TCP_PROTOCOL => {
                    let src_port = u16::from_be(unsafe { *ptr_at(&ctx, IPV6_HDR_LEN + offset_of!(tcphdr, source))? });
                    let dst_port = u16::from_be(unsafe { *ptr_at(&ctx, IPV6_HDR_LEN + offset_of!(tcphdr, dest))? });
                    (src_port, dst_port)
                }
        
                _ => {
                    (0x0000, 0x0000)
                }
            };
            let log_entry = PacketLog {
                src: source,
                dst: dest,
                src_port: src_port,
                dst_port: dst_port,
                l3_protocol: l3_protocol,
                action: xdp_action::XDP_PASS,
                is_ipv4: false,
            };
        
            unsafe {
                PACKETS.output(&ctx, &log_entry, 0);
            }

            Ok(xdp_action::XDP_PASS)
        }

        ETH_P_IPV4 => {
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
        
            let action = match unsafe { OUTBOUND_BLOCKLIST.get(&dest) } {
                Some(_) => xdp_action::XDP_ABORTED,
                None => xdp_action::XDP_PASS
            };

            let log_entry = PacketLog {
                src: source as u128,
                dst: dest as u128,
                src_port: src_port,
                dst_port: dst_port,
                l3_protocol: l3_protocol,
                action: action,
                is_ipv4: true,
            };
        
            unsafe {
                PACKETS.output(&ctx, &log_entry, 0);
            }

            if action == xdp_action::XDP_PASS {
                Ok(xdp_action::XDP_PASS)
            } else {
                Err(())
            }
        }
        _ => Ok(xdp_action::XDP_PASS)
    }
    

}

const ETH_P_IPV4: u16 = 0x0800;
const ETH_P_IPV6: u16 = 0x86DD;
const IPV6_HDR_LEN:usize = mem::size_of::<ipv6hdr>();
const ETH_HDR_LEN: usize = mem::size_of::<ethhdr>();
const IP_HDR_LEN: usize = mem::size_of::<iphdr>() + ETH_HDR_LEN;