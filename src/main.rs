use accord::{network::Message, node::Node};
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc};
use structopt::StructOpt;
use warp::hyper::body::Bytes;
use warp::reply::Json;
use warp::{http::Response, Filter};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::{sleep, Duration},
};

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

#[derive(Serialize)]
struct InfoReponse {
    node_hash: String,
    successor: SocketAddr,
    others: Vec<SocketAddr>,
}

fn api_info(node: Arc<ChordNode>) -> Json {
    let succ = node.successor.lock().unwrap();
    let mut resp = InfoReponse {
        node_hash: format!("{:x}", u64::from(node.id)),
        successor: succ.web_addr,
        others: Vec::with_capacity(1),
    };
    let pred = node.predecessor.lock().unwrap();
    if pred.is_some() {
        resp.others.push(pred.unwrap().web_addr);
    }
    return warp::reply::json(&resp);
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    let chord_node = Arc::new(ChordNode::new(opt.address, opt.webserver_adress));
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

    let info_chord_node = chord_node.clone();
    let info = warp::path!("node-info")
        .and(warp::get())
        .map(move || api_info(info_chord_node.clone()));

    let webserver = warp::serve(get.or(put).or(info)).bind(opt.webserver_adress);
    let stabilize_node = chord_node.clone();

    let stabilizer_task = async {
        loop {
            sleep(Duration::from_secs(opt.stabilization_period)).await;
            if let Err(err) = stabilize_node.stabilize().await {
                println!("error: {:?}", err);
            }
        }
    };

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
            println!("suicide!");
        },
    }
}
