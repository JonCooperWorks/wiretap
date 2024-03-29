use aya::{
    maps::{
        HashMap,
        perf::AsyncPerfEventArray,
    },
    programs::{Xdp, XdpFlags},
    util::online_cpus,
    Bpf,
};
use bytes::BytesMut;
use csv_async::AsyncSerializer;
use futures_batch::ChunksTimeoutStreamExt;
use rusoto_core::Region;
use rusoto_s3::{PutObjectRequest, S3Client, S3};
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use std::{
    convert::{TryFrom, TryInto},
    fs, net,
};
use structopt::StructOpt;
use tokio::{signal, sync::mpsc, task};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use wiretap_common::{PacketLog, PacketLogWrapper};

mod storage;
mod utils;

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

    #[structopt(long, default_value="169.254.169.254")]
    banned_ips: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::from_args();

    // Set up S3 compatible cloud storage
    let region = Region::Custom {
        name: opt.storage_region,
        endpoint: opt.storage_endpoint,
    };
    let s3 = S3Client::new(region);

    // Set up wiretap BPF program
    let data = fs::read(&opt.path)?;
    let mut bpf = Bpf::load(&data)?;
    let probe: &mut Xdp = bpf.program_mut("wiretap")?.try_into()?;
    probe.load()?;
    probe.attach(&opt.iface, XdpFlags::SKB_MODE)?;

    // Send blocklist to BPF program.
    let mut outbound_blocklist = HashMap::try_from(bpf.map_mut("OUTBOUND_BLOCKLIST")?)?;
    for ip in opt.banned_ips {
        let ipaddr: Ipv4Addr = ip.parse().ok().unwrap();
        let number = u32::from(ipaddr);
        outbound_blocklist.insert(number, 1u8, 0)?;
    }
    
    let mut packets = AsyncPerfEventArray::try_from(bpf.map_mut("PACKETS")?)?;

    let config = storage::Config {
        max_packets_per_log: opt.max_packets_per_log,
        packet_log_interval: Duration::from_secs(opt.packet_log_interval * 60),
        storage_bucket: opt.storage_bucket,
    };

    let (packet_tx, packet_rx) = mpsc::channel(config.max_packets_per_log);

    for cpu_id in online_cpus()? {
        let mut buf = packets.open(cpu_id, None)?;
        let tx = packet_tx.clone();

        task::spawn(async move {
            let mut buffers = (0..10)
                .map(|_| BytesMut::with_capacity(1024))
                .collect::<Vec<_>>();

            loop {
                let events = buf.read_events(&mut buffers).await.unwrap();
                for i in 0..events.read {
                    let buf = &mut buffers[i];
                    let packet_log = PacketLogWrapper {
                        data: buf.as_ptr() as *const PacketLog,
                    };
                    let data = unsafe { packet_log.data.read_unaligned() };
                    let timestamp = utils::timestamp();

                    // PacketLog field accesses wrapped in {} to prevent warnings from unaligned fields
                    // See https://github.com/rust-lang/rust/issues/82523
                    let log = if data.is_ipv4 {
                        let src = u32::try_from(data.src).ok().unwrap();
                        let dst = u32::try_from(data.dst).ok().unwrap();
                        let src_addr = net::Ipv4Addr::from(src);
                        let dst_addr = net::Ipv4Addr::from(dst);
                        storage::FlowLog {
                            src: IpAddr::V4(src_addr),
                            dst: IpAddr::V4(dst_addr),
                            src_port: { data.src_port },
                            dst_port: { data.dst_port },
                            l3_protocol: { data.l3_protocol },
                            action: { data.action },
                            timestamp: timestamp,
                        }
                    } else {
                        let src_addr = net::Ipv6Addr::from(data.src);
                        let dst_addr = net::Ipv6Addr::from(data.dst);
                        storage::FlowLog {
                            src: IpAddr::V6(src_addr),
                            dst: IpAddr::V6(dst_addr),
                            src_port: { data.src_port },
                            dst_port: { data.dst_port },
                            l3_protocol: { data.l3_protocol },
                            action: { data.action },
                            timestamp: timestamp,
                        }
                    };

                    tx.send(log).await.ok();
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
            let mut serializer = AsyncSerializer::from_writer(vec![]);

            for log in &packet_logs {
                serializer.serialize(&log).await.unwrap();
            }
            let f = serializer.into_inner().await.unwrap();

            let timestamp = utils::timestamp();
            let filename = format!("{}.csv", timestamp);
            let req = PutObjectRequest {
                bucket: config.storage_bucket.to_owned(),
                key: filename.to_owned(),
                body: Some(f.into()),
                ..Default::default()
            };

            // TODO: handle errors from S3
            let _res = s3.put_object(req).await.unwrap();

            for log in packet_logs {
                let protocol = utils::l3_protocol(log.l3_protocol);
                println!(
                    "{}: {} {}:{} -> {}:{}, ACTION {}",
                    log.timestamp,
                    protocol,
                    log.src,
                    log.src_port,
                    log.dst,
                    log.dst_port,
                    log.action
                );
            }
            println!("Saved {}", filename);
        }
    });

    signal::ctrl_c().await.expect("failed to listen for event");
    Ok::<_, anyhow::Error>(())
}
