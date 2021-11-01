from os import lseek
import subprocess
from types import ClassMethodDescriptorType
from typing import Dict, List
import requests
import time
import random
import argparse
import socket 
import sys
from multiprocessing.pool import ThreadPool
from pprint import pprint

# Logger
import logging
logging.basicConfig(format="%(levelname)s: %(message)s")
logger = logging.getLogger()

def randport():
    return random.randint(59152,65534)

def check_stable(nodes: List[int], ports: List[int], debug: bool):
    p = ThreadPool(len(nodes))

    def get_successor(node_and_port):
        resp = requests.get(f'http://{node_and_port}/node-info')
        return (node_and_port, resp.json()["successor"])

    logger.debug("get successors...")
    edges = p.map(get_successor, [
        f"{nodes[i]}:{ports[i]+1}" for i in range(len(nodes))])
    # print(edges)

    successors = [succ for (node, succ) in edges if node != succ]

    logger.debug("stabilization status: {len(successors)}/{len(nodes)}")

    p.close()
    # check if each node has a unique successor
    print(f"len succ = {len(set(successors))}, nodes == {len(nodes)}")
    return len(set(successors)) == len(nodes)


def create_network(args, ips, ports) -> List[subprocess.Popen]:

    logger.info(f"spawning {len(ips)} nodes")

    processes = []

    kwargs = {}
    if args.log_level != "DEBUG":
        kwargs["stdout"] = subprocess.PIPE


    for i in range(len(ips)):
        ip = ips[i]
        chport = ports[i]
        wsport = chport + 1
        eachnodecmd = ["~/accord/target/debug/accord",
                        # own ip
                        f"{ip}:{chport}",
                        f"{ip}:{wsport}",
                        "--stabilization-period", "500"]
        cmd = [
            f"ssh",
            "-f",
            f"{ip}",
            " ".join(eachnodecmd)]
        p = subprocess.Popen(cmd, **kwargs)
        processes.append(p)
        
    # wait one second to make sure all nodes have been created
    time.sleep(1)

    pprint(ips)
    pprint(ports)

    p = ThreadPool(len(ips))

    def join(i): ##join the first node 
        return requests.get(
            f'http://{ips[i]}:{ports[i]+1}/join?nprime={ips[0]}:{ports[0]+1}')

    logger.info(f"start joining...")
    start = time.time()
    p.map(join, range(1,len(ips)))

    logger.info(f"waiting for stabilization...")

    while not check_stable(ips, ports, False):
        time.sleep(1)

    end = time.time()
    logger.info(f"stabilization took {end-start:.3f} seconds")

    print("list of processes completed")
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
    

    cmd = ["/share/apps/ifi/available-nodes.sh"]
    MyOut = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT
    )
    stdout,stderr = MyOut.communicate()

    available_nodes = str(stdout, 'utf-8').split('\n')[:-1] #last one is emty line
    available_ips = list(map((lambda x : socket.gethostbyname(x)), available_nodes))
    nodes = []
    ips = []
    ports = []
    for i in range(0,int(sys.argv[1])):
        randomindex = random.randint(0,len(available_ips))
        ip = available_ips[randomindex]
        node = available_nodes[randomindex]
        chport = randport()
        # wsport = chport + 1
        nodes.append(node) ## hostname just in case
        ports.append(chport)
        ips.append(ip)
        print(f"{node} -> {ip}:{chport}")
    
    processes = create_network(args, ips, ports)
    
    if(args.num_leaves > 0):
        test_leave(args)

    # wait until user kills process

    for p in processes:
        p.wait()
