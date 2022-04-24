use std::fs::File;
use std::net::SocketAddr;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(name = "Packet-Play")]
#[clap(author, version, about,long_about = None)]
struct Cli {
    file: String,
    #[clap(parse(try_from_str))]
    destination: Option<SocketAddr>,
}

fn main() {
    let cli = Cli::parse();
    println!("file {}", cli.file);
    println!("dest {}", cli.destination.unwrap());
}
