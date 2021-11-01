use std::net::SocketAddr;
use std::sync::Arc;
use warp::hyper::body::Bytes;

use serde::{Deserialize, Serialize};
use warp::http::Response;
use warp::hyper::body::to_bytes;
use warp::hyper::{Client, Uri};
use warp::reply::Json;
use warp::Filter;

use crate::node::Node;

pub type ChordNode = Node<String, String>;

pub async fn get(node: Arc<ChordNode>, key: String) -> Result<Response<String>, warp::Rejection> {
    let b = Response::builder();
    if node.is_crashed().await {
        return Ok(b
            .status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body("oh no I crashed :(".to_string())
            .unwrap());
    }

    match node.lookup(key).await {
        Ok(value) => {
            let b = Response::builder();
            let resp = if let Some(v) = value {
                b.status(warp::http::StatusCode::OK)
                    .header("content-type", "text/plain")
                    .body(v)
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
) -> Result<Response<String>, warp::Rejection> {
    let b = Response::builder();
    if node.is_crashed().await {
        return Ok(b
            .status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body("oh no I crashed :(".to_string())
            .unwrap());
    }
    let body = std::str::from_utf8(&value).unwrap();

    let (status, msg) = match node.put(key.clone(), body.to_string()).await {
        Ok(_) => (warp::http::StatusCode::OK, "ok"),
        Err(err) => {
            eprintln!(
                "[{:}] error performing put (key={:}, value={:?}): {:?}",
                node.address, key, value, err
            );
            (
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                "error occured while performing puts operation",
            )
        }
    };
    Ok(Response::builder()
        .status(status)
        .body(msg.to_string())
        .unwrap())
}

#[derive(Serialize, Deserialize)]
struct InfoReponse {
    node_hash: String,
    successor: SocketAddr,
    chord_address: SocketAddr,
    others: Vec<SocketAddr>,
}

pub async fn info(node: Arc<ChordNode>) -> Result<Json, warp::Rejection> {
    if node.is_crashed().await {
        panic!("tried to call info for crashed node");
    }
    let succ = { node.successor.lock().await.clone() };
    let mut resp = InfoReponse {
        node_hash: format!("{:x}", u64::from(node.id)),
        successor: succ.web_addr,
        others: Vec::with_capacity(1),
        chord_address: node.address,
    };

    if let Some(p) = node.predecessor.lock().await.clone() {
        resp.others.push(p.web_addr);
    }
    if let Some(s) = node.second_successor.lock().await.clone() {
        resp.others.push(s.web_addr);
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
    if node.is_crashed().await {
        return Ok(b
            .status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body("oh no I crashed :(".to_string())
            .unwrap());
    }
    let client = Client::new();

    let url: Uri = format!("http://{:}/node-info", req.nprime).parse().unwrap();

    let ok: bool;
    let mut err_str: String = "".to_string();

    // TODO fix this ugly code
    match client.get(url).await {
        Ok(mut resp) => match to_bytes(resp.body_mut()).await {
            Ok(body_bytes) => match serde_json::from_slice::<InfoReponse>(&body_bytes) {
                Ok(info) => {
                    if let Err(err) = node.join(info.chord_address).await {
                        err_str = format!("{:?}", err);
                        ok = false;
                    } else {
                        ok = true;
                    }
                }
                Err(err) => {
                    err_str = format!("{:?}", err);
                    ok = false;
                }
            },
            Err(err) => {
                err_str = format!("{:?}", err);
                ok = false;
            }
        },
        Err(err) => {
            err_str = format!("{:?}", err);
            ok = false;
        }
    }
    if ok {
        println!("[{:}] joined chord network {:}", node.address, req.nprime);
        Ok(b.status(warp::http::StatusCode::OK)
            .body("ok".to_string())
            .unwrap())
    } else {
        println!("[{:}] cannot join chord network {:}", node.address, err_str);
        Ok(b.status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body("cannot join network".to_string())
            .unwrap())
    }
}

pub async fn leave(node: Arc<ChordNode>) -> Result<Response<String>, warp::Rejection> {
    let b = Response::builder();
    if node.is_crashed().await {
        return Ok(b
            .status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body("oh no I crashed :(".to_string())
            .unwrap());
    }

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

pub async fn sim_crash(node: Arc<ChordNode>) -> Result<Response<String>, warp::Rejection> {
    let b = Response::builder();
    if node.is_crashed().await {
        return Ok(b
            .status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body("oh no I crashed :(".to_string())
            .unwrap());
    }
    if let Err(err) = node.sim_crash().await {
        eprintln!(
            "[{:}] error while simulating crash: {:?}",
            node.address, err
        );
        Ok(b.status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body("error while sim-crash".to_string())
            .unwrap())
    } else {
        println!("[{:}] artificially crashed", node.address);
        Ok(b.status(warp::http::StatusCode::OK)
            .body("ok".to_string())
            .unwrap())
    }
}
pub async fn sim_recover(node: Arc<ChordNode>) -> Result<Response<String>, warp::Rejection> {
    let b = Response::builder();
    if let Err(err) = node.sim_recover().await {
        eprintln!(
            "[{:}] error while simulating crash: {:?}",
            node.address, err
        );
        Ok(b.status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body("error while sim-recovering".to_string())
            .unwrap())
    } else {
        println!(
            "[{:}] can now try to reconnect to chord network",
            node.address
        );
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
        .and_then(move |req: JoinRequest| join(join_chord_node.clone(), req));

    let leave_chord_node = node.clone();
    let leave = warp::path!("leave").and_then(move || leave(leave_chord_node.clone()));

    let sim_crash_node = node.clone();
    let sim_crash = warp::path!("sim-crash").and_then(move || sim_crash(sim_crash_node.clone()));

    let sim_recover_node = node.clone();
    let sim_recover =
        warp::path!("sim-recover").and_then(move || sim_recover(sim_recover_node.clone()));

    warp::serve(
        get.or(put)
            .or(info)
            .or(join)
            .or(leave)
            .or(sim_crash)
            .or(sim_recover),
    )
    .bind(addr)
    .await;
}
