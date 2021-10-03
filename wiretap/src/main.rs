use aya::{
    maps::perf::AsyncPerfEventArray,
    programs::{Xdp, XdpFlags},
    util::online_cpus,
    Bpf,
};
use bytes::BytesMut;
use std::{
    convert::{TryFrom, TryInto},
    fs, net,
};
use structopt::StructOpt;
use tokio::{signal, task};

use wiretap_common::IPv4PacketLog;


#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short, long)]
    path: String,
    #[structopt(short, long, default_value = "eth0")]
    iface: String,
}

fn l3_protocol(protocol: u8) -> String {
    match protocol {
        wiretap_common::ICMP_PROTOCOL => String::from("ICMP"),
        wiretap_common::UDP_PROTOCOL => String::from("UDP"),
        wiretap_common::TCP_PROTOCOL => String::from("TCP"),
        _ => format!("{:#04x}", protocol),
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::from_args();
    let data = fs::read(&opt.path)?;
    
    let mut bpf = Bpf::load(&data)?;
    let probe: &mut Xdp = bpf.program_mut("wiretap")?.try_into()?;
    probe.load()?;
    probe.attach(&opt.iface, XdpFlags::SKB_MODE)?;

    let mut perf_array = AsyncPerfEventArray::try_from(bpf.map_mut("IPv4_PACKETS")?)?;

    for cpu_id in online_cpus()? {
        let mut buf = perf_array.open(cpu_id, None)?;

        task::spawn(async move {
            let mut buffers = (0..10)
                .map(|_| BytesMut::with_capacity(1024))
                .collect::<Vec<_>>();

            loop {
                let events = buf.read_events(&mut buffers).await.unwrap();
                for i in 0..events.read {
                    let buf = &mut buffers[i];
                    let ptr = buf.as_ptr() as *const IPv4PacketLog;
                    let data = unsafe { ptr.read_unaligned() };
                    let src_addr = net::Ipv4Addr::from(data.src);
                    let dst_addr = net::Ipv4Addr::from(data.dst);
                    let protocol = l3_protocol(data.l3_protocol);

                    // IPv4PacketLog field accesses wrapped in {} to prevent warnings from unaligned fields
                    // See https://github.com/rust-lang/rust/issues/82523
                    println!("LOG: {} {}:{} -> {}:{}, ACTION {}", protocol, src_addr, {data.src_port}, dst_addr, {data.dst_port}, {data.action});

                    // TODO: send over storage channel.
                }
            }
        });
    }

    signal::ctrl_c().await.expect("failed to listen for event");
    Ok::<_, anyhow::Error>(())
}