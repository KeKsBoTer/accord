import subprocess
from typing import Dict, List
import requests
import time
import random
import argparse
from multiprocessing.pool import ThreadPool

# Logger
import logging
logging.basicConfig(format="%(levelname)s: %(message)s")
logger = logging.getLogger()


def check_stable(nodes: List[int], debug: bool):
    p = ThreadPool(len(nodes))

    def get_successor(node):
        resp = requests.get(f'http://{node}/node-info')
        return (node, resp.json()["successor"])

    logger.debug("get successors...")
    edges = p.map(get_successor, [
        f"127.0.0.1:{args.ws_port+i}" for i in nodes])
    # print(edges)

    successors = [succ for (node, succ) in edges if node != succ]

    logger.debug("stabilization status: {len(successors)}/{len(nodes)}")

    p.close()
    # check if each node has a unique successor
    return len(set(successors)) == len(nodes)


def create_network(args) -> List[subprocess.Popen]:

    logger.info(f"spawning {args.num_nodes} nodes")

    processes = []

    kwargs = {}
    if args.log_level != "DEBUG":
        kwargs["stdout"] = subprocess.PIPE

    for i in range(args.num_nodes):
        p = subprocess.Popen(["./target/debug/accord",
                              f"127.0.0.1:{args.chord_port+i}",
                              f"127.0.0.1:{args.ws_port+i}",
                              "--stabilization-period", "500"], **kwargs)
        processes.append(p)

    # wait one second to make sure all nodes have been created
    time.sleep(1)

    p = ThreadPool(args.num_nodes)

    def join(i):
        return requests.get(
            f'http://127.0.0.1:{args.ws_port+i}/join?nprime=127.0.0.1:{args.ws_port}')

    logger.info(f"start joining...")
    start = time.time()
    p.map(join, range(args.num_nodes))

    logger.info(f"waiting for stabilization...")

    while not check_stable(list(range(args.num_nodes)), False):
        time.sleep(1)

    end = time.time()
    logger.info(f"stabilization took {end-start:.3f} seconds")

    return processes


def test_leave(args: Dict):

    logger.info(f"{args.num_leaves} nodes are leaving the network")

    alive_nodes = list(range(args.num_nodes))
    leave_nodes = []
    # stupid way to get random subset of nodes
    for i in range(args.num_nodes-args.num_leaves):
        i = alive_nodes[random.randrange(len(alive_nodes))]
        leave_nodes.append(i)
        alive_nodes.remove(i)

    burst_leave = True

    def leave(i):
        return requests.get(
            f'http://127.0.0.1:{args.ws_port+i}/leave')

    start = time.time()

    # tell half of the nodes to leave the network
    if burst_leave:
        p.map(leave, leave_nodes)
    else:
        for i in leave_nodes:
            leave(i)

    while not check_stable(alive_nodes, True):
        time.sleep(1)

    end = time.time()
    logger.info(f"restabilization took {end-start:.3f} seconds")


def parse_args():
    parser = argparse.ArgumentParser(
        prog="network_creator", description="creates a chord network")

    parser.add_argument("--chord-port-start", dest="chord_port", type=int,
                        default=9000,
                        help="smallest port for chord network")

    parser.add_argument("--api-port-start", dest="ws_port", type=int,
                        default=8000,
                        help="smallest port for HTTP API")

    parser.add_argument("--stabilization-period", dest="stabilization_period", type=int,
                        default=1000,
                        help="delay beteween stabilizatios")

    parser.add_argument("--num-leaves", dest="num_leaves", type=int,
                        default=0,
                        help="number of nodes that leave the network after stabilization (test)")

    parser.add_argument("--log-level", dest="log_level", type=str,
                        default="INFO",
                        help="number of nodes that leave the network after stabilization (test)")

    parser.add_argument("num_nodes", type=int,
                        help="number of nodes in the network")

    return parser.parse_args()


if __name__ == "__main__":
    args = parse_args()
    logger.setLevel("INFO")

    processes = create_network(args)
    if(args.num_leaves > 0):
        test_leave(args)

    # wait until user kills process

    for p in processes:
        p.wait()
