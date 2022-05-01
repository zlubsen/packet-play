mod model;

use std::fs::File;
use std::net::{Ipv4Addr, SocketAddr};
use std::process::exit;
use clap::Parser;
use crate::model::Recording::PCAP;

#[derive(Parser, Debug)]
#[clap(name = "packet-play")]
#[clap(author, version, about,long_about = None)]
struct Cli {
    file: String,
    #[clap(parse(try_from_str))]
    destination: Option<SocketAddr>,
    ttl: Option<u8>,
}

fn main() {
    let cli = Cli::parse();
    let filename : String = cli.file;
    let destination = cli.destination.unwrap_or(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(),3000));

    println!("file: {filename}");
    let file_path = std::path::Path::new(filename.as_str());
    let recording = if file_path.is_file() && file_path.exists() {
        File::open(file_path)
    } else {
        println!("Provided file {filename} is not a file or does not exist.");
        exit(1);
    };
    println!("recording: {recording:?}");
    println!("dest {destination}");


}
