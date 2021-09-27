use accord::node::{message_listener, Node};
use std::{net::SocketAddr, sync::Arc};
use structopt::StructOpt;
use warp::hyper::{body::Bytes, Response};
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

    #[structopt(name = "predecessor", help = "predecessor ip and port")]
    predecessor: SocketAddr,

    #[structopt(name = "predecessor_ws", help = "predecessor HTTP API ip and port")]
    predecessor_ws: SocketAddr,

    #[structopt(name = "successor", help = "successor ip and port")]
    successor: SocketAddr,

    #[structopt(name = "successor_ws", help = "successor HTTP API ip and port")]
    successor_ws: SocketAddr,

    #[structopt(
        long,
        default_value = "10",
        help = "number of minutes after the node will kill itself"
    )]
    ttl: u64,
}

async fn api_get(node: Arc<ChordNode>, key: String) -> Result<Response<String>, warp::Rejection> {
    match node.lookup(key).await {
        Ok(value) => {
            let b = Response::builder();
            let resp = if let Some(v) = value {
                b.status(warp::http::StatusCode::OK).body(v)
            } else {
                b.status(warp::http::StatusCode::NOT_FOUND)
                    .body("".to_string())
            };
            Ok(resp.unwrap())
        }
        Err(err) => {
            eprintln!("error in lookup: {:?}", err);
            Err(warp::reject::reject())
        }
    }
}

async fn api_put(
    node: Arc<ChordNode>,
    key: String,
    value: Bytes,
) -> Result<String, warp::Rejection> {
    let body = std::str::from_utf8(&value).unwrap();
    let ok = node.put(key, body.to_string()).await;

    match ok {
        Ok(_) => Ok("ok".to_string()),
        Err(e) => {
            eprintln!("{:?}", e);
            Err(warp::reject::reject())
        }
    }
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    let chord_node = Arc::new(ChordNode::new(
        opt.address,
        opt.webserver_adress,
        opt.predecessor,
        opt.predecessor_ws,
        opt.successor,
        opt.successor_ws,
    ));

    let chord_server = message_listener(chord_node.clone());

    let storage_api = warp::path!("storage" / String);

    let get_chord_node = chord_node.clone();
    // get items api
    let get = storage_api
        .and(warp::get())
        .and_then(move |key| api_get(get_chord_node.clone(), key));

    let put_chord_node = chord_node.clone();
    // store items api
    let put = storage_api
        .and(warp::put())
        .and(warp::body::bytes())
        .and_then(move |key: String, value: Bytes| api_put(put_chord_node.clone(), key, value));

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
