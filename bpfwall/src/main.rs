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

use bpfwall_common::{IPv4PacketLog, l3_protocol};


#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short, long)]
    path: String,
    #[structopt(short, long, default_value = "eth0")]
    iface: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::from_args();
    let data = fs::read(&opt.path)?;
    
    let mut bpf = Bpf::load(&data)?;
    let probe: &mut Xdp = bpf.program_mut("bpfwall")?.try_into()?;
    probe.load()?;
    probe.attach(&opt.iface, XdpFlags::SKB_MODE)?;

    let mut perf_array = AsyncPerfEventArray::try_from(bpf.map_mut("EVENTS")?)?;

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
                    println!("LOG: {} {}:{} -> {}:{}, ACTION {}", protocol, src_addr, data.src_port, dst_addr, data.dst_port, data.action);
                }
            }
        });
    }

    signal::ctrl_c().await.expect("failed to listen for event");
    Ok::<_, anyhow::Error>(())
}