use nom::bytes::complete::take;
use nom::IResult;
use nom::number::complete::be_u16;
use crate::model::pcap::Pcap;
use crate::model::pcapng::PcapNG;

pub mod pcap;
pub mod pcapng;

const UDP_HEADER_LENGTH : u16 = 8;

#[derive(Debug)]
pub enum Errors {
    ParsePcapError,
    ParsePcapNgError,
}

pub enum Recording {
    PCAP(Pcap),
    PCAPNG(PcapNG),
}

pub struct UdpPacket {
    pub header: UdpHeader,
    pub data: Vec<u8>,
}

pub struct UdpHeader {
    pub source_port: u16,
    pub destination_port: u16,
    pub length: u16,
    pub checksum: u16,
}

fn parse_udp_packet(input: &[u8]) -> IResult<&[u8], UdpPacket> {
    let (input, header) = udp_header(input)?;
    let (input, data) = take(header.length - UDP_HEADER_LENGTH)(input)?;
    let data = data.to_vec();

    Ok((input, UdpPacket {
        header,
        data
    }))
}

fn udp_header(input: &[u8]) -> IResult<&[u8], UdpHeader> {
    let (input, source_port) = be_u16(input)?;
    let (input, destination_port) = be_u16(input)?;
    let (input, length) = be_u16(input)?;
    let (input, checksum) = be_u16(input)?;

    Ok((input, UdpHeader {
        source_port,
        destination_port,
        length,
        checksum,
    }))
}