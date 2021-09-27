// A small tool used to calculate the hashes within the python script
// to calculate the predecessors and succesors for each node

use accord::routing::id::HashIdentifier;
use std::net::SocketAddr;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "hasher", about = "chord hash generator for nodes")]
struct Opt {
    #[structopt(name = "adress", help = "address and port")]
    address: SocketAddr,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    println!("{:}", opt.address.hash_id());
}
