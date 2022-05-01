use std::fs::File;
use std::io::Read;
use bytes::Bytes;
use nom::IResult;
use crate::model::Errors;

pub struct Pcap {
    pub header: PcapFileHeader,
    pub packets: Vec<PcapPacketRecord>,
}

pub struct PcapFileHeader {
    pub magic_number: PcapMagicNumber,
    pub major_version: u16,
    pub minor_version: u16,
    pub snap_len: u32,
    pub link_type: u32,
    pub frame_cyclic_sequence: u8,
    pub f_bit: bool,
}

pub enum PcapMagicNumber {
    MS(u32), // 0xA1B2C3D4
    NS(u32), // 0xA1B23C4D
}

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
        let mut buf = Bytes::new();
        let bytes_read = file.read(&mut buf).unwrap();
        if let Ok((input, pcap)) = parse_pcap(&buf) {
            Ok(pcap)
        } else {
            Err(Errors::ParsePcapError)
        }
    }
}

fn parse_pcap(input: &[u8]) -> IResult<&[u8], Pcap> {
    let (input, header) = pcap_header(input)?;
    let (input, packets) = pcap_packets(input)?;
    Ok((input, Pcap {
        header,
        packets,
    }))
}

fn pcap_header(input: &[u8]) -> IResult<&[u8], PcapFileHeader> {
    todo!()
}

fn pcap_packets(input: &[u8]) -> IResult<&[u8], Vec<PcapPacketRecord>> {
    todo!()
}