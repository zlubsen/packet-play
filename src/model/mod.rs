use std::time::Duration;
use crate::model::pcap::Pcap;
use crate::model::pcapng::PcapNG;
use crate::player::PlayerState;

pub(crate) mod pcap;
pub(crate) mod pcapng;

pub(crate) const ETHERNET_HEADER_LENGTH : u16 = 13;
pub(crate) const IP_HEADER_LENGTH : u16 = 20;
pub(crate) const UDP_HEADER_LENGTH : u16 = 8;

#[derive(Clone, Debug)]
pub(crate) enum Error {
    ParsePcapError,
    ParsePcapNgError,
    PlayerInitError,
}

pub(crate) enum Recording {
    PCAP(Pcap),
    PCAPNG(PcapNG),
}

#[derive(Copy, Clone, PartialEq)]
pub(crate) enum Command {
    Play,
    Pause,
    Rewind,
    Quit,
    Unspecified,
    SyncTerm, // indicates the player that the CLI is ready drawing
}

impl Command {
    pub fn as_vec() -> Vec<&'static str> {
        vec![
            "Play",
            "Pause",
            "Rewind",
            "Quit",
            "", // empty line for the progress bar
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

#[derive(Clone)]
pub(crate) enum Event {
    Error(Error),
    PlayerStateChanged(StateChange),
    PlayerPositionChanged(PositionChange),
}

#[derive(Copy, Clone)]
pub(crate) struct StateChange {
    pub(crate) state: PlayerState,
}

#[derive(Copy, Clone)]
pub(crate) struct PositionChange {
    pub(crate) position: usize,
    pub(crate) max_position: usize,
    pub(crate) time_position: Duration,
    pub(crate) time_total: Duration,
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