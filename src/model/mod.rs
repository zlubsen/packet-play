use crate::model::pcap::Pcap;
use crate::model::pcapng::PcapNG;

pub(crate) mod pcap;
pub(crate) mod pcapng;

pub(crate) const ETHERNET_HEADER_LENGTH : u16 = 13;
pub(crate) const IP_HEADER_LENGTH : u16 = 20;
pub(crate) const UDP_HEADER_LENGTH : u16 = 8;

#[derive(Debug)]
pub enum Errors {
    ParsePcapError,
    ParsePcapNgError,
}

pub enum Recording {
    PCAP(Pcap),
    PCAPNG(PcapNG),
}

#[derive(Copy, Clone, PartialEq)]
pub enum Command {
    Play,
    Pause,
    Rewind,
    Quit,
    Unspecified,
}

impl Command {
    pub fn as_vec() -> Vec<&'static str> {
        vec![
            "Play",
            "Pause",
            "Rewind",
            "Quit",
        ]
    }
}

impl From<usize> for Command {
    fn from(value: usize) -> Self {
        match value {
            0 => { Command::Play }
            1 => { Command::Pause }
            2 => { Command::Rewind }
            3 => { Command::Quit }
            _ => { Command::Unspecified }
        }
    }
}

// pub struct UdpPacket {
//     pub header: UdpHeader,
//     pub data: Vec<u8>,
// }
//
// pub struct UdpHeader {
//     pub source_port: u16,
//     pub destination_port: u16,
//     pub length: u16,
//     pub checksum: u16,
// }
//
// fn parse_udp_packet(input: &[u8]) -> IResult<&[u8], UdpPacket> {
//     let (input, header) = udp_header(input)?;
//     let (input, data) = take(header.length - UDP_HEADER_LENGTH)(input)?;
//     let data = data.to_vec();
//
//     Ok((input, UdpPacket {
//         header,
//         data
//     }))
// }
//
// fn udp_header(input: &[u8]) -> IResult<&[u8], UdpHeader> {
//     let (input, source_port) = be_u16(input)?;
//     let (input, destination_port) = be_u16(input)?;
//     let (input, length) = be_u16(input)?;
//     let (input, checksum) = be_u16(input)?;
//
//     Ok((input, UdpHeader {
//         source_port,
//         destination_port,
//         length,
//         checksum,
//     }))
// }