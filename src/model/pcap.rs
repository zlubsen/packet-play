use std::fs::File;
use std::io::{BufReader, Read};
use log::trace;
use nom::bytes::complete::take;
use nom::IResult;
use nom::multi::many1;
use nom::number::complete::{be_u16, be_u32};
use crate::model::Errors;

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
    MILLIS, // 0xA1B2C3D4
    NANOS, // 0xA1B23C4D
}

impl From<u32> for PcapMagicNumber {
    fn from(value: u32) -> Self {
        match value {
            0xA1B23C4D => PcapMagicNumber::NANOS,
            0xA1B2C3D4 | _ => PcapMagicNumber::MILLIS,
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
    type Error = Errors;

    fn try_from(mut file: File) -> Result<Self, Self::Error> {
        let mut buf = Vec::<u8>::new();
        let mut reader = BufReader::new(file);
        let _read = reader.read_to_end(&mut buf);
        trace!("start parsing pcap file");
        match parse_pcap_file(&buf) {
            Ok((input, pcap)) => { Ok(pcap) }
            Err(err) => { trace!("{err}"); Err(Errors::ParsePcapError) }
        }
        // if let Ok((input, pcap)) = parse_pcap_file(&buf) {
        //     Ok(pcap)
        // } else {
        //     Err(Errors::ParsePcapError)
        // }
    }
}

fn parse_pcap_file(input: &[u8]) -> IResult<&[u8], Pcap> {
    trace!("parse the file");
    let (input, header) = pcap_header(input)?;
    trace!("parsed the header");
    let (input, packets) = many1(pcap_packet_record)(input)?;
    trace!("parsed the packets");
    Ok((input, Pcap {
        header,
        packets,
    }))
}

fn pcap_header(input: &[u8]) -> IResult<&[u8], PcapFileHeader> {
    trace!("parse the header");
    let (input, magic_number) = be_u32(input)?;
    let (input, major_version) = be_u16(input)?;
    let (input, minor_version) = be_u16(input)?;
    let (input, _reserved) = take(8usize)(input)?;
    let (input, snap_len) = be_u32(input)?;
    let (input, link_type) = be_u32(input)?;
    let frame_cyclic_sequence : u8 = ((link_type & 0xF0000000) >> 28) as u8;
    let f_bit = if (link_type & 0x10000000) >> 27 == 0 {false} else {true};

    let magic_number = PcapMagicNumber::from(magic_number);

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

fn pcap_packet_record(input: &[u8]) -> IResult<&[u8], PcapPacketRecord> {
    trace!("parse a packet record");
    let (input, ts_secs) = be_u32(input)?;
    let (input, ts_secs_fraction) = be_u32(input)?;
    let (input, captured_packet_length) = be_u32(input)?;
    let (input, original_packet_length) = be_u32(input)?;
    trace!("parsed packet record fields");
    let (input, packet_data) = take(captured_packet_length)(input)?;
    trace!("parsed packet record data");

    let packet_data = packet_data.to_vec();
    Ok((input, PcapPacketRecord {
        ts_secs,
        ts_secs_fraction,
        captured_packet_length,
        original_packet_length,
        packet_data,
    }))
}