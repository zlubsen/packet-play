use std::fs::File;
use std::io::{BufReader, Read};
use log::trace;
use nom::bytes::complete::take;
use nom::combinator::peek;
use nom::IResult;
use nom::multi::many1;
use nom::number::complete::{le_u32, u16, u32};
use nom::number::Endianness;
use crate::model::{Error};

#[derive(Debug)]
pub struct Pcap {
    pub header: PcapFileHeader,
    pub packets: Vec<PcapPacketRecord>,
}

#[derive(Debug)]
pub struct PcapFileHeader {
    pub magic_number: PcapMagicNumber,
    pub major_version: u16,
    pub minor_version: u16,
    pub snap_len: u32,
    pub link_type: u32,
    pub frame_cyclic_sequence: u8,
    pub f_bit: bool,
}

#[derive(Debug)]
pub enum PcapMagicNumber {
    LeMicros,     // 0xA1B2C3D4 - Little Endian - time fraction in micro seconds
    BeNanos,      // 0xA1B23C4D - Big Endian - time fraction in nano seconds
}

impl From<u32> for PcapMagicNumber {
    fn from(value: u32) -> Self {
        match value {
            0xA1B23C4D => PcapMagicNumber::BeNanos,
            0xA1B2C3D4 | _ => PcapMagicNumber::LeMicros,
        }
    }
}

#[derive(Debug)]
pub struct PcapPacketRecord {
    pub ts_secs: u32,
    pub ts_secs_fraction: u32,
    pub captured_packet_length: u32,
    pub original_packet_length: u32,
    pub packet_data: Vec<u8>,
}

impl TryFrom<File> for Pcap {
    type Error = Error;

    fn try_from(file: File) -> Result<Self, Self::Error> {
        let mut buf = Vec::<u8>::new();
        let mut reader = BufReader::new(file);
        let _read = reader.read_to_end(&mut buf);
        trace!("start parsing pcap file");
        match parse_pcap_file(&buf) {
            Ok((_input, pcap)) => { Ok(pcap) }
            Err(_err) => {
                Err(Error::ParsePcapError) }
        }
    }
}

fn parse_pcap_file(input: &[u8]) -> IResult<&[u8], Pcap> {
    let (input, magic_number_as_le) = peek(le_u32)(input)?;
    let endianness = determine_endianness(magic_number_as_le);

    let (input, header) = pcap_header(endianness)(input)?;
    let (input, packets) = many1(pcap_packet_record(endianness))(input)?;
    Ok((input, Pcap {
        header,
        packets,
    }))
}

fn pcap_header(endianness: Endianness) -> impl Fn(&[u8]) -> IResult<&[u8], PcapFileHeader> {
    move |input| {
        let (input, magic_number) = u32(endianness)(input)?;
        let magic_number = PcapMagicNumber::from(magic_number);

        let (input, major_version) = u16(endianness)(input)?;
        let (input, minor_version) = u16(endianness)(input)?;
        let (input, _reserved) = take(8usize)(input)?;
        let (input, snap_len) = u32(endianness)(input)?;
        let (input, link_type) = u32(endianness)(input)?;
        let frame_cyclic_sequence: u8 = ((link_type & 0xF0000000) >> 28) as u8;
        let f_bit = if (link_type & 0x10000000) >> 27 == 0 { false } else { true };

        Ok((input, PcapFileHeader {
            magic_number,
            major_version,
            minor_version,
            snap_len,
            link_type,
            frame_cyclic_sequence,
            f_bit,
        }))
    }
}

fn pcap_packet_record(endianness: Endianness) -> impl Fn(&[u8]) -> IResult<&[u8], PcapPacketRecord> {
    move |input| {
        let (input, ts_secs) = u32(endianness)(input)?;
        let (input, ts_secs_fraction) = u32(endianness)(input)?;
        let (input, captured_packet_length) = u32(endianness)(input)?;
        let (input, original_packet_length) = u32(endianness)(input)?;
        // TODO also parse the Ethernet and IP headers if we need to support other data link and network protocols
        // let (input, _ethernet_header) = take(ETHERNET_HEADER_LENGTH)(input)?;
        // let (input, _ip_header) = take(IP_HEADER_LENGTH)(input)?;
        // TODO also parse the UDP header if needed to know what the original ports and addresses were.
        // let (input, _udp_header) = take(UDP_HEADER_LENGTH)(input)?;
        // trace!("captured packet length: {captured_packet_length}");
        // trace!("original packet length: {original_packet_length}");
        let (input, packet_data) = take(captured_packet_length)(input)?;

        let packet_data = packet_data.to_vec();
        Ok((input, PcapPacketRecord {
            ts_secs,
            ts_secs_fraction,
            captured_packet_length,
            original_packet_length,
            packet_data,
        }))
    }
}

fn determine_endianness(magic_number: u32) -> Endianness {
    if magic_number == 0xA1B2C3D4 {
        Endianness::Little
    } else { // magic_number == 0xA1B23C4D
        Endianness::Big
    }
}