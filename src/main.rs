use accord::{
    network::{self},
    node::Node,
};
use std::net::SocketAddr;
use std::sync::Mutex;
use structopt::StructOpt;

use tokio::time::{timeout, Duration};

type ChordNode = Node<String, String>;

#[derive(StructOpt)]
#[structopt(name = "akkord", about = "Chord Node Process")]
struct Opt {
    #[structopt(name = "address", help = "address to bind to")]
    address: SocketAddr,

    #[structopt(long, help = "address of entry node")]
    entry_node: Option<SocketAddr>,

    #[structopt(
        long,
        default_value = "10",
        help = "number of minutes after the node will kill itself"
    )]
    ttl: u64,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    let mut n = ChordNode::new(opt.address);
    if let Some(entry_node) = opt.entry_node {
        n.join(entry_node).await;
        println!("{:} joined chord network {:}", n.address, entry_node);
    } else {
        println!("creating new chord network {:}", n.address);
    }

    let m = Mutex::new(&mut n);

    let listener = network::listen_for_messages(opt.address, |msg| async {
        if let Ok(mut n) = m.lock() {
            n.handle_message(msg).await
        } else {
            None
        }
    });

    // kill process after some time
    timeout(Duration::from_secs(opt.ttl * 60), listener)
        .await
        .unwrap()
        .unwrap();
}
