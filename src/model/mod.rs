use crate::model::pcap::Pcap;
use crate::model::pcapng::PcapNG;

mod pcap;
mod pcapng;

pub enum Errors {
    ParsePcapError,
    ParsePcapNgError,
}

pub enum Recording {
    PCAP(Pcap),
    PCAPNG(PcapNG),
}

pub struct UdpHeader {
    pub source_port: u16,
    pub destination_port: u16,
    pub length: u16,
    pub checksum: u16,
}