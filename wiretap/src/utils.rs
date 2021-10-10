use std::time::{SystemTime, UNIX_EPOCH};

use wiretap_common;

pub fn l3_protocol(protocol: u8) -> String {
    match protocol {
        wiretap_common::ICMP_PROTOCOL => String::from("ICMP"),
        wiretap_common::UDP_PROTOCOL => String::from("UDP"),
        wiretap_common::TCP_PROTOCOL => String::from("TCP"),
        _ => format!("{:#04x}", protocol),
    }
}

pub fn timestamp() -> u64 {
    return SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
}