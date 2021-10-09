use aya::{
    maps::perf::AsyncPerfEventArray,
    programs::{Xdp, XdpFlags},
    util::online_cpus,
    Bpf,
};
use bytes::BytesMut;
use futures_batch::ChunksTimeoutStreamExt;
use s3::{
    bucket::Bucket,
    creds::Credentials,
    Region,
};
use std::{
    convert::{TryFrom, TryInto},
    fs, net,
};
use std::net::IpAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use structopt::StructOpt;
use tokio::{
    signal,
    sync::mpsc,
    task,
};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

use wiretap_common::{IPv4PacketLog, IPv4PacketLogWrapper};

mod storage;
use storage::{Config, IPv4FlowLog};

mod utils;
use utils::l3_protocol;



#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long)]
    path: String,

    #[structopt(long, default_value = "eth0")]
    iface: String,

    #[structopt(long)]
    storage_region: String,

    #[structopt(long)]
    storage_endpoint: String,

    #[structopt(long)]
    storage_bucket: String,

    #[structopt(long, default_value = "1000000")]
    max_packets_per_log: usize,

    #[structopt(long, default_value = "5")]
    packet_log_interval: u64,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::from_args();

    // Set up S3 compatible cloud storage
    let region = Region::Custom { 
        region: opt.storage_region, 
        endpoint: opt.storage_endpoint,
    };
    let credentials = Credentials::default()?;
    let bucket = Bucket::new(&opt.storage_bucket, region, credentials).unwrap();

    // Set up wiretap BPF program
    let data = fs::read(&opt.path)?;
    let mut bpf = Bpf::load(&data)?;
    let probe: &mut Xdp = bpf.program_mut("wiretap")?.try_into()?;
    probe.load()?;
    probe.attach(&opt.iface, XdpFlags::SKB_MODE)?;
    let mut perf_array = AsyncPerfEventArray::try_from(bpf.map_mut("IPv4_PACKETS")?)?;
    
    let config = Config{
        max_packets_per_log: opt.max_packets_per_log,
        packet_log_interval: Duration::from_secs(opt.packet_log_interval * 60),
    };

    let (packet_tx, packet_rx) = mpsc::channel(config.max_packets_per_log);

    for cpu_id in online_cpus()? {
        let mut buf = perf_array.open(cpu_id, None)?;
        let tx = packet_tx.clone();

        task::spawn(async move {
            let mut buffers = (0..10)
                .map(|_| BytesMut::with_capacity(1024))
                .collect::<Vec<_>>();

            loop {
                let events = buf.read_events(&mut buffers).await.unwrap();
                for i in 0..events.read {
                    let buf = &mut buffers[i];
                    let packet_log = IPv4PacketLogWrapper
                     {data: buf.as_ptr() as *const IPv4PacketLog};
                    let data = unsafe { packet_log.data.read_unaligned() };
                    let src_addr = net::Ipv4Addr::from(data.src);
                    let dst_addr = net::Ipv4Addr::from(data.dst);
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_secs();

                    // IPv4PacketLog field accesses wrapped in {} to prevent warnings from unaligned fields
                    // See https://github.com/rust-lang/rust/issues/82523
                    let log = IPv4FlowLog{
                        src: IpAddr::V4(src_addr),
                        dst: IpAddr::V4(dst_addr),
                        src_port: {data.src_port},
                        dst_port: {data.dst_port},
                        l3_protocol: {data.l3_protocol},
                        action: {data.action},
                        timestamp: timestamp,
                    };

                    tx.send(log).await;
                }
            }
        });
    }

    // Send packet logs to cloud storage.
    task::spawn(async move {
        // Wrap rx in a stream and split it into chunks of max_packets_per_log
        let mut packet_events = ReceiverStream::new(packet_rx)
            .chunks_timeout(config.max_packets_per_log, config.packet_log_interval);
        
        while let Some(packet_logs) = packet_events.next().await {
            // TODO: send chunks as CSV to cloud storage.
            for log in packet_logs {
                let protocol = l3_protocol(log.l3_protocol);
                println!("{}: {} {}:{} -> {}:{}, ACTION {}", log.timestamp, protocol, log.src, log.src_port, log.dst, log.dst_port, log.action);
            }
        }
        
    });

    signal::ctrl_c().await.expect("failed to listen for event");
    Ok::<_, anyhow::Error>(())
}