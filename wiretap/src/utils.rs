use wiretap_common;

pub fn l3_protocol(protocol: u8) -> String {
    match protocol {
        wiretap_common::ICMP_PROTOCOL => String::from("ICMP"),
        wiretap_common::UDP_PROTOCOL => String::from("UDP"),
        wiretap_common::TCP_PROTOCOL => String::from("TCP"),
        _ => format!("{:#04x}", protocol),
    }
}