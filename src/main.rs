extern crate pnet;

use pnet::datalink::{self, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ipv4::Ipv4Packet;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("usage: wiretap <interface name>");
        process::exit(0);
    }

    let interface_name = &args[1];

    let interface_names_match =
        |iface: &NetworkInterface| iface.name == *interface_name;

    // Find the network interface with the provided name
    let interfaces = datalink::interfaces();
    let interface = interfaces.into_iter()
                              .filter(interface_names_match)
                              .next()
                              .unwrap();

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
            },
            Err(e) => {
                println!("An error occurred while reading: {}", e);
            }
        }
    }
}

