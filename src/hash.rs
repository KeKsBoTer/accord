use accord::routing::id::HashIdentifier;
use std::net::SocketAddr;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "hasher", about = "Chord Node Process")]
struct Opt {
    #[structopt(name = "hash", help = "address to bind to")]
    address: SocketAddr,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    println!("{:}", opt.address.hash_id());
}
