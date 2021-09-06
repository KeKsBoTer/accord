use accord::{com, node::Node};
use std::{net::SocketAddr, thread, time::Duration};
use structopt::StructOpt;

type StringNode = Node<String, String, 8>;

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
    let opt = Opt::from_args();
    suicide_thread(Duration::from_secs(opt.ttl * 60));

    let node = StringNode::new(opt.entry_node, opt.entry_node);

    let socket = com::MessageSocket::bind(opt.address).unwrap();

    let receiver = thread::spawn(move || {
        while let Ok((msg, sender)) = socket.recv_from() {
            match msg {
                com::Message::Ping => {
                    socket.send_to(com::Message::Pong, sender).unwrap();
                }
                com::Message::Pong => {
                    todo!("keep track of alive nodes");
                }
                com::Message::Lookup(key, querier) => {
                    if node.contains_bucket(key) {
                        socket.send_to(com::Message::Result(true), querier).unwrap();
                    }
                }
                com::Message::Result(dst) => {
                    // connect to node and proxy value to clinet
                    todo!("implement result receiving!");
                }
                com::Message::Notify(r) => {
                    todo!("implement notify!");
                }
            };
        }
    });
    receiver.join().unwrap();
}
