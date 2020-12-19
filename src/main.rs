extern crate pnet;
use crate::pnet::packet::Packet;

use std::net::Ipv4Addr;
use std::time::SystemTime;

use clap::{Arg, App};

use pnet::datalink::{self, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::ip::IpNextHeaderProtocols;

fn timestamp() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}

struct FlowLog {
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    timestamp: u64,
    protocol: String
}

impl FlowLog {
    fn new(packet: Ipv4Packet) -> Option<FlowLog> {
        match packet.get_next_level_protocol() {
            IpNextHeaderProtocols::Tcp => {
                let tcp_packet = TcpPacket::new(packet.payload()).unwrap();
                let flow_log = FlowLog{
                    src_ip: packet.get_source(),
                    dst_ip: packet.get_destination(),
                    dst_port: tcp_packet.get_destination(),
                    src_port: tcp_packet.get_source(),
                    timestamp: timestamp(),
                    protocol: String::from("T")
                };
                return Some(flow_log);
            }

            IpNextHeaderProtocols::Udp => {
                let udp_packet = UdpPacket::new(packet.payload()).unwrap();
                let flow_log = FlowLog{
                    src_ip: packet.get_source(),
                    dst_ip: packet.get_destination(),
                    dst_port: udp_packet.get_destination(),
                    src_port: udp_packet.get_source(),
                    timestamp: timestamp(),
                    protocol: String::from("U")
                };
                return Some(flow_log);
            }
            _ => {
                println!("unsupported protocol");
                return None;
            }
        }
    }
}

fn main() {
    let args = App::new("wiretap")
        .version("0.1.0")
        .author("Jonathan Cooper <joncooperworks.com>")
        .about("Pulls flow logs from an interface and sends them to CosmosDB")
        .arg(Arg::with_name("interface")
                .short("i")
                .long("interface")
                .takes_value(true)
                .help("Network interface to be tapped"))
        .arg(Arg::with_name("cosmosdb-connection")
                .short("c")
                .long("cosmosdb-connection")
                .takes_value(true)
                .help("CosmosDB connection string"))
        .get_matches();

    let interface_name = args.value_of("interface").unwrap_or("wg0");

    let interface_names_match =
        |iface: &NetworkInterface| iface.name == interface_name;

    // Find the network interface with the provided name
    let interfaces = datalink::interfaces();
    let interfaces = interfaces.into_iter()
                              .filter(interface_names_match)
                              .next();

    let interface = match interfaces {
        Some(interface) => interface,
        None => {
            println!("interface {} not found", interface_name);
            std::process::exit(0);
        }
    };


    // Create a new channel, dealing with layer 2 packets
    let (_tx, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(_tx, rx)) => (_tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
    };

    loop {
        match rx.next() {
            Ok(packet) => {
                match Ipv4Packet::new(packet) {
                    Some(ip4_packet) => {
                        match FlowLog::new(ip4_packet) {
                            Some(flow_log) => {
                                println!(
                                    "{} - {} {}:{} -> {}:{}", 
                                    flow_log.timestamp, 
                                    flow_log.protocol, 
                                    flow_log.src_ip, 
                                    flow_log.src_port, 
                                    flow_log.dst_ip, 
                                    flow_log.dst_port
                                );
                                // TODO: batch flow logs into sqlite database for exfil to CosmosDB

                            }
                            None => println!("protocol not supported")
                        }
                    }

                    None => println!("only IPv4 supported at this time!")
                }
                

                
            },
            Err(e) => {
                println!("An error occurred while reading: {}", e);
            }
        }
    }
}

