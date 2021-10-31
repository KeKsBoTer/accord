use accord::api;
use accord::network::Message;

use std::{net::SocketAddr, sync::Arc};
use structopt::StructOpt;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::{sleep, Duration},
};
#[derive(StructOpt)]
#[structopt(name = "akkord", about = "Chord Node Process")]
struct Opt {
    #[structopt(name = "address", help = "address to bind to")]
    address: SocketAddr,

    #[structopt(name = "webserver-adress", help = "webserver address to bind to")]
    webserver_adress: SocketAddr,

    #[structopt(
        long,
        default_value = "1",
        help = "duration (seconds) between stabilization runs"
    )]
    stabilization_period: u64,

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

    let chord_node = Arc::new(api::ChordNode::new(opt.address, opt.webserver_adress));
    println!("creating new chord network {:}", chord_node.address);

    let listener = TcpListener::bind(opt.address).await.unwrap();
    let chord_server = async {
        loop {
            let (mut tcp_stream, _) = listener.accept().await.unwrap();

            let tcp_chord_node = chord_node.clone();
            tokio::spawn(async move {
                // TODO error handling
                let mut send_buf = Vec::with_capacity(32);
                tcp_stream.read_to_end(&mut send_buf).await.unwrap();
                let msg: Message = serde_cbor::from_slice(send_buf.as_slice()).unwrap();

                if let Some(resp) = tcp_chord_node.handle_message(msg).await.unwrap() {
                    let buf = serde_cbor::to_vec(&resp).unwrap();
                    tcp_stream.write_all(&buf).await.unwrap();
                    tcp_stream.shutdown().await.unwrap();
                }
            });
        }
    };

    let stabilize_node = chord_node.clone();
    let stabilizer_task = async {
        loop {
            sleep(Duration::from_secs(opt.stabilization_period)).await;
            if let Err(err) = stabilize_node.stabilize().await {
                println!("error: {:?}", err);
            }
        }
    };

    let api_node = chord_node.clone();
    let webserver = api::serve(opt.webserver_adress, api_node);

    tokio::select! {
        val = chord_server => {
            println!("chord server shut down: {:?}",val);
        },
        val = webserver => {
            println!("webserver shut down: {:?}",val);
        },
        val = stabilizer_task => {
            println!("stabilizer shut down: {:?}",val);
        },
        _ = sleep(Duration::from_secs(opt.ttl * 60)) => {
            // kill process after some time
            println!("[{:}] suicide!",chord_node.address);
        },
    }
}
