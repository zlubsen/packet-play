mod model;

use std::env;
use std::fs::File;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::process::exit;
use std::time::{Duration, Instant};
use clap::Parser;
use log::{info, error, trace};
use crate::model::{ETHERNET_HEADER_LENGTH, IP_HEADER_LENGTH, UDP_HEADER_LENGTH};

const DEFAULT_DEST_PORT : u16 = 3000;
const DEFAULT_SRC_PORT : u16 = 3000;
const DEFAULT_TTL : u32 = 1;

#[derive(Parser, Debug)]
#[clap(name = "packet-play")]
#[clap(author, version, about,long_about = None)]
struct Cli {
    file: String,
    #[clap(parse(try_from_str))]
    #[clap(short, long, default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST),DEFAULT_DEST_PORT))]
    destination: SocketAddr,
    #[clap(short = 's', long = "source", default_value_t = DEFAULT_SRC_PORT)]
    source_port: u16,
    #[clap(short, long, default_value_t = DEFAULT_TTL)]
    ttl: u32,
}

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();

    info!("Using file: {}", cli.file);
    info!("Replaying to destination: {:?}", cli.destination);
    // if let Some(destination) = cli.destination {
    //     info!("Replaying to destination: {:?}", destination);
    // } else {
    //     info!("Replaying to default destination ({})", IpAddr::V4(Ipv4Addr::BROADCAST));
    // }

    let file_path = std::path::Path::new(cli.file.as_str());
    if !file_path.is_file() || !file_path.exists() {
        error!("Provided path {} is not a file or does not exist.", {cli.file});
        exit(1);
    };

    // TODO tidy up... fix builder
    let player = Player::new(cli.file).destination(cli.destination).source_port(cli.source_port).ttl(cli.ttl);
    // let player = if let Some(destination) = cli.destination {
    //     player.destination(destination)
    // } else { player };
    // let player = if let Some(source_port) = cli.source_port {
    //     player.source_port(source_port)
    // } else { player };
    // let player = if let Some(ttl) = cli.ttl {
    //     player.ttl(ttl)
    // } else {player};
    player.play();
}

struct Player {
    file_path: String,
    destination: SocketAddr,
    source_port: u16,
    ttl: u32,
}

impl Player {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            destination: SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST),DEFAULT_DEST_PORT),
            source_port: DEFAULT_SRC_PORT,
            ttl: DEFAULT_TTL,
        }
    }

    pub fn file(self, file_path: String) -> Self {
        // if file_path.is_some() {
        //     let file_path = file_path.unwrap();
            Self {
                file_path,
                ..self
            }
        // } else { self }
    }

    pub fn destination(self, destination: SocketAddr) -> Self {
        // if destination.is_some() {
        //     let destination = destination.unwrap();
            Self {
                destination,
                ..self
            }
        // } else { self }
    }

    pub fn source_port(self, source_port: u16) -> Self {
        // if source_port.is_some() {
        //     let source_port = source_port.unwrap();
            Self {
                source_port,
                ..self
            }
        // } else { self }
    }

    pub fn ttl(self, ttl: u32) -> Self {
        // if ttl.is_some() {
        //     let ttl = ttl.unwrap();
            Self {
                ttl,
                ..self
            }
        // } else { self }
    }

    pub fn play(&self) {
        let socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), self.source_port)).expect(format!("Failed to bind socket to port {:?}", self.source_port).as_str());
        socket.set_broadcast(true).expect("Failed to set socket SO_BROADCAST option.");
        socket.set_ttl(self.ttl).expect("Failed to set socket TTL value");

        let file = File::open(self.file_path.clone()).unwrap();
        let recording = model::pcap::Pcap::try_from(file);

        if let Ok(recording) = recording {
            trace!("Ready to play: {:?}", recording.header);
            info!("Replaying {} packets", recording.packets.len());
            let start_time = recording.packets.first().unwrap().ts_secs;
            let strip_headers_index = (ETHERNET_HEADER_LENGTH+IP_HEADER_LENGTH+UDP_HEADER_LENGTH+1) as usize;

            for packet in recording.packets {
                let bytes_send = socket.send_to(&packet.packet_data.as_slice()[strip_headers_index..], self.destination).expect("Could not send packet");
                trace!("bytes transmitted: {bytes_send}");
                std::thread::sleep(Duration::from_millis(500));
            }
        } else {
            let error = recording.unwrap_err();
            error!("Cannot play recording, because: {:?}", error);
        }
    }
}
