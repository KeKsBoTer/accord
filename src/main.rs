use accord::{network, node::Node};
use std::sync::Mutex;
use std::{net::SocketAddr, thread, time::Duration};
use structopt::StructOpt;

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

fn suicide_thread(timeout: Duration) {
    thread::spawn(move || {
        thread::sleep(timeout);
        println!("timeout: suicide!");
        std::process::exit(1);
    });
}

fn main() {
    let opt = Opt::from_args();
    suicide_thread(Duration::from_secs(opt.ttl * 60));

    let mut n = ChordNode::new(opt.address);
    if let Some(entry_node) = opt.entry_node {
        n.join(entry_node);
        println!("{:} joined chord network {:}", n.address, entry_node);
    } else {
        println!("creating new chord network {:}", n.address);
    }

    let m = Mutex::new(&mut n);
    network::listen_for_messages(opt.address, |msg| {
        if let Ok(mut n) = m.lock() {
            n.handle_message(msg)
        } else {
            None
        }
    })
    .unwrap();
}
