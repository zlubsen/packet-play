use std::str::FromStr;
use std::time::Duration;
use crate::model::pcap::Pcap;
use crate::model::pcapng::PcapNG;
use crate::player::PlayerState;

pub(crate) mod pcap;
pub(crate) mod pcapng;

pub(crate) const ETHERNET_HEADER_LENGTH : u16 = 13;
pub(crate) const IP_HEADER_LENGTH : u16 = 20;
pub(crate) const UDP_HEADER_LENGTH : u16 = 8;

pub enum Mode {
    Cli,
    Gui,
}

impl FromStr for Mode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "cli" => { Ok(Mode::Cli) }
            "gui" => { Ok(Mode::Gui) }
            _ => Err(Error::ArgumentError),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Error {
    ArgumentError,
    ParsePcapError,
    ParsePcapNgError,
    PlayerInitError,
    FileTypeNotSupported(String),
    CommandChannelError,
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
    // SyncTerm, // indicates the player that the CLI is ready drawing
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
pub enum Event {
    Error(Error),
    PlayerReady,
    PlayerStateChanged(StateChange),
    PlayerPositionChanged(PositionChange),
    QuitCommanded,
}

impl Event {
    pub(crate) fn state_event(state: PlayerState) -> Self {
        Event::PlayerStateChanged(StateChange{
            state
        })
    }

    pub(crate) fn position_event(current_pos: usize, max_pos: usize, current_time:Duration, total_time: Duration) -> Self {
        // This function increases the position with +1 to compensate for 0-based vec indexing.
        Event::PlayerPositionChanged(PositionChange{
            position: current_pos+1,
            max_position: max_pos,
            time_position: current_time,
            time_total: total_time,
        })
    }

    pub(crate) fn error(error: Error) -> Self {
        Event::Error(error)
    }
}

#[derive(Copy, Clone)]
pub struct StateChange {
    pub(crate) state: PlayerState,
}

#[derive(Copy, Clone)]
pub struct PositionChange {
    pub(crate) position: usize,
    pub(crate) max_position: usize,
    pub(crate) time_position: Duration,
    pub(crate) time_total: Duration,
}

impl Default for PositionChange {
    fn default() -> Self {
        Self {
            position: 0,
            max_position: 0,
            time_position: Duration::from_secs(0),
            time_total: Duration::from_secs(0),
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