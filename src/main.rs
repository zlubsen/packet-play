use std::fs::File;
use std::net::{Ipv4Addr, SocketAddr};
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(name = "packet-play")]
#[clap(author, version, about,long_about = None)]
struct Cli {
    file: String,
    #[clap(parse(try_from_str))]
    destination: Option<SocketAddr>,
}

fn main() {
    let cli = Cli::parse();
    println!("file {}", cli.file);
    println!("dest {}", cli.destination.unwrap_or(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(),3000)));
}
