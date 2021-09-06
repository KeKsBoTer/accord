use accord::{com, node::Node};
use std::net::{IpAddr, Ipv4Addr};
use std::{
    net::{SocketAddr, SocketAddrV4},
    thread,
    time::Duration,
};
use structopt::StructOpt;

static mut NODES: Vec<Node<String, String>> = Vec::new();

#[derive(StructOpt)]
#[structopt(name = "akkord", about = "Chord Node Process")]
struct Opt {
    #[structopt(name = "address", help = "address to bind to")]
    address: SocketAddr,

    #[structopt(name = "entry_node", help = "address of entry node")]
    entry_node: SocketAddr,

    #[structopt(
        long,
        default_value = "10",
        help = "number of minutes after the node will kill itself"
    )]
    ttl: u64,
}

fn suicide_thread(timeout: Duration) {
    thread::spawn(move || {
        thread::sleep(timeout);
        println!("timeout: suicide!");
        std::process::exit(1);
    });
}

fn main() {
    // let opt = Opt::from_args();
    //suicide_thread(Duration::from_secs(opt.ttl * 60));

    for i in 1..10 {
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        unsafe {
            NODES.push(Node::new(SocketAddr::new(ip, 8080 + i)));
        }
    }
}
