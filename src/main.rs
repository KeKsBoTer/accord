use accord::{
    network::{self},
    node::Node,
};
use std::{net::SocketAddr, sync::Arc};
use structopt::StructOpt;
use warp::hyper::{body::Bytes};
use warp::Filter;

use tokio::time::{sleep, Duration};

type ChordNode = Node<String, String>;

#[derive(StructOpt)]
#[structopt(name = "akkord", about = "Chord Node Process")]
struct Opt {
    #[structopt(name = "address", help = "address to bind to")]
    address: SocketAddr,

    #[structopt(name = "address-adress", help = "webserver address to bind to")]
    webserver_adress: SocketAddr,

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

    let chord_node = Arc::new(ChordNode::new(opt.address));
    if let Some(entry_node) = opt.entry_node {
        if let Err(err) = chord_node.join(entry_node).await {
            println!("cannot join network: {:?}", err);
            return;
        } else {
            println!(
                "{:} joined chord network {:}",
                chord_node.address, entry_node
            );
        }
    } else {
        println!("creating new chord network {:}", chord_node.address);
    }

    let chord_server = network::listen_for_messages(opt.address, |msg| {
        let node = chord_node.clone();
        async move {
            match node.handle_message(msg).await {
                Ok(resp) => resp,
                Err(_) => None,
            }
        }
    });
    let storage_api = warp::path!("storage" / String);

    let get_chord_node = chord_node.clone();
    // get items api
    let get = storage_api.and(warp::get()).and_then(move |key| {
        let node = get_chord_node.clone();
        async move {
            let value = node.lookup(key).await.unwrap().map(|v| v.to_string());
            match value {
                Some(v) => Ok(v),
                None => Err(warp::reject::not_found()),
            }
        }
    });

    let put_chord_node = chord_node.clone();
    // store items api
    let put = storage_api
        .and(warp::put())
        .and(warp::body::bytes())
        .and_then(move |key: String, value: Bytes| {
            let node = put_chord_node.clone();
            async move {
                let body = std::str::from_utf8(&value).unwrap();
                let ok = node.put(key, body.to_string()).await;
                match ok {
                    Ok(_) => Ok("ok"),
                    Err(_) => Err(warp::reject::reject()), // TODO return 500
                }
            }
        });

    let n_chord_node = chord_node.clone();
    let neighbors = warp::path!("neighbors")
        .and(warp::get())
        .map(move || warp::reply::json(&n_chord_node.neighbors()));

    let webserver = warp::serve(get.or(put).or(neighbors)).bind(opt.webserver_adress);

    tokio::select! {
        val = chord_server => {
            println!("chord server shut down: {:?}",val);
        },
        val = webserver => {
            println!("webserver shut down: {:?}",val);
        },
        _ = sleep(Duration::from_secs(opt.ttl * 60)) => {
            // kill process after some time
            println!("suicide!");
        },
    }
}
