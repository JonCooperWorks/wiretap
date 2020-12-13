extern crate pnet;

use clap::{Arg, App};
use pnet::datalink::{self, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ipv4::Ipv4Packet;


fn main() {
    let args = App::new("wiretap")
        .version("0.1.0")
        .author("Jonathan Cooper <joncooperworks.com>")
        .about("Pulls flow logs from an interface")
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


    // Create a new channel, dealing with layer 3 packets
    let (_tx, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(_tx, rx)) => (_tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
    };

    loop {
        match rx.next() {
            Ok(packet) => {
                let packet = Ipv4Packet::new(packet).unwrap();
                println!("{} -> {}", packet.get_source(), packet.get_destination());

                // TODO: Send to CosmosDB
            },
            Err(e) => {
                println!("An error occurred while reading: {}", e);
            }
        }
    }
}

