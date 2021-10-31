use std::net::SocketAddr;
use std::sync::Arc;
use warp::hyper::body::Bytes;

use serde::{Deserialize, Serialize};
use warp::http::Response;
use warp::reply::Json;
use warp::Filter;

use crate::node::Node;

pub type ChordNode = Node<String, String>;

pub async fn get(node: Arc<ChordNode>, key: String) -> Result<Response<String>, warp::Rejection> {
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

pub async fn put(
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

pub async fn info(node: Arc<ChordNode>) -> Result<Json, warp::Rejection> {
    let succ = node.successor.lock().await;
    let mut resp = InfoReponse {
        node_hash: format!("{:x}", u64::from(node.id)),
        successor: succ.web_addr,
        others: Vec::with_capacity(1),
    };
    let pred = node.predecessor.lock().await;
    if pred.is_some() {
        resp.others.push(pred.unwrap().web_addr);
    }

    return Ok(warp::reply::json(&resp));
}

#[derive(Deserialize, Clone, Copy)]
pub struct JoinRequest {
    // web address of the chord node that the node should join
    nprime: SocketAddr,
}

pub async fn join(
    node: Arc<ChordNode>,
    req: JoinRequest,
) -> Result<Response<String>, warp::Rejection> {
    let b = Response::builder();
    // TODO right now the node joins the address given as http GET param
    // this is the web address of the node though
    // change to use the chord address of the node
    // maybe add api to retrieve chord address first?
    if let Err(err) = node.join(req.nprime).await {
        eprintln!("[{:}] cannot join network: {:?}", node.address, err);
        Ok(b.status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body("cannot leave network".to_string())
            .unwrap())
    } else {
        println!("[{:}] joined chord network {:}", node.address, req.nprime);
        Ok(b.status(warp::http::StatusCode::OK)
            .body("ok".to_string())
            .unwrap())
    }
}

pub async fn leave(node: Arc<ChordNode>) -> Result<Response<String>, warp::Rejection> {
    let b = Response::builder();
    // TODO right now the node joins the address given as http GET param
    // this is the web address of the node though
    // change to use the chord address of the node
    // maybe add api to retrieve chord address first?
    if let Err(err) = node.leave().await {
        eprintln!("[{:}] cannot leave network: {:?}", node.address, err);
        Ok(b.status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body("cannot leave network".to_string())
            .unwrap())
    } else {
        println!("[{:}] left chord network", node.address);
        Ok(b.status(warp::http::StatusCode::OK)
            .body("ok".to_string())
            .unwrap())
    }
}

pub async fn serve(addr: SocketAddr, node: Arc<ChordNode>) {
    let storage_api = warp::path!("storage" / String);
    let get_chord_node = node.clone();
    // get items api
    let get = storage_api
        .and(warp::get())
        .and_then(move |key| get(get_chord_node.clone(), key));

    let put_chord_node = node.clone();
    // store items api
    let put = storage_api
        .and(warp::put())
        .and(warp::body::bytes())
        .and_then(move |key: String, value: Bytes| put(put_chord_node.clone(), key, value));

    let info_chord_node = node.clone();
    let info = warp::path!("node-info")
        .and(warp::get())
        .and_then(move || info(info_chord_node.clone()));

    let join_chord_node = node.clone();
    let join = warp::path!("join")
        .and(warp::query())
        .and(warp::get())
        .and_then(move |req: JoinRequest| join(join_chord_node.clone(), req));

    warp::serve(get.or(put).or(info).or(join)).bind(addr).await;
}
