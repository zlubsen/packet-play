mod model;

use std::env;
use std::fs::File;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::process::exit;
use std::time::{Duration, Instant};
use clap::Parser;
use log::{info, error, trace};
use crate::model::Recording::PCAP;

const DEFAULT_DEST_PORT : u16 = 3000;
const DEFAULT_SRC_PORT : u16 = 3000;
const DEFAULT_TTL : u32 = 1;

#[derive(Parser, Debug)]
#[clap(name = "packet-play")]
#[clap(author, version, about,long_about = None)]
struct Cli {
    file: String,
    #[clap(parse(try_from_str))]
    destination: Option<SocketAddr>,
    #[clap(short = 's', long = "source")]
    source_port: Option<u16>,
    ttl: Option<u32>,
}

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();
    let filename : String = cli.file;
    let destination = cli.destination.unwrap_or(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(),3000));
    let ttl : u32 = cli.ttl.unwrap_or(DEFAULT_TTL);

    info!("Loading file: {filename}");
    info!("Replaying to destination: {destination}");

    let file_path = std::path::Path::new(filename.as_str());
    if !file_path.is_file() || !file_path.exists() {
        error!("Provided file {filename} is not a file or does not exist.");
        exit(1);
    };

    let player = Player::new(filename).destination(destination).ttl(ttl);
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
        Self {
            file_path,
            ..self
        }
    }

    pub fn destination(self, destination: SocketAddr) -> Self {
        Self {
            destination,
            ..self
        }
    }

    pub fn ttl(self, ttl: u32) -> Self {
        Self {
            ttl,
            ..self
        }
    }

    pub fn play(&self) {
        let socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], self.source_port)),).expect(format!("Failed to bind socket {:?}", self.destination).as_str());
        socket.set_broadcast(true);
        socket.set_ttl(self.ttl);
        let mut file = File::open(self.file_path.clone()).unwrap();
        let recording = model::pcap::Pcap::try_from(file);
        if let Ok(recording) = recording {
            info!("ready to play: {:?}", recording.header);
            let start_time = recording.packets.first().unwrap().ts_secs;
            for packet in recording.packets {
                trace!("packet - secs: {} - len: {}", packet.ts_secs, packet.captured_packet_length);
                socket.send_to(&packet.packet_data.as_slice()[..8], self.destination).expect("Could not send packet");
                std::thread::sleep(Duration::from_millis(500));
            }
        } else {
            let error = recording.unwrap_err();
            error!("Cannot play recording, because: {:?}", error);
        }
    }
}
