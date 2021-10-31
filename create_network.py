from os import umask
import subprocess
from typing import List
import requests
import time
import random
from multiprocessing.pool import ThreadPool

entry_node = 9000
ws_port = 8000

processes = []

num_nodes = 50
num_leaves = (2*num_nodes)//3

stabilization_period = 500


def check_stable(nodes: List[int], debug: bool):
    p = ThreadPool(len(nodes))

    def get_successor(node):
        resp = requests.get(f'http://{node}/node-info')
        return (node, resp.json()["successor"])

    if debug:
        print("DEBUG: get successors...")
    edges = p.map(get_successor, [
        f"127.0.0.1:{ws_port+i}" for i in nodes])
    # print(edges)

    successors = [succ for (node, succ) in edges if node != succ]

    if debug:
        print(f"DEBUG: stabilization status: {len(successors)}/{len(nodes)}")

    # check if each node has a unique successor
    return len(set(successors)) == len(nodes)


print(f"INFO: spawning {num_nodes} nodes")

for i in range(num_nodes):
    p = subprocess.Popen(["./target/debug/accord",
                          f"127.0.0.1:{entry_node+i}",
                          f"127.0.0.1:{ws_port+i}",
                          "--stabilization-period", "500"], stdout=subprocess.PIPE)
    processes.append(p)

# wait one second to make sure all nodes have been created
time.sleep(1)

p = ThreadPool(num_nodes)


def join(i):
    return requests.get(
        f'http://127.0.0.1:{ws_port+i}/join?nprime=127.0.0.1:{ws_port}')


print(f"INFO: start joining...")
start = time.time()
p.map(join, range(num_nodes))

print(f"INFO: waiting for stabilization...")

while not check_stable(list(range(num_nodes)), False):
    time.sleep(1)

end = time.time()
print(f"INFO: stabilization took {end-start:.3f} seconds")


print(f"INFO: {num_leaves} nodes are leaving the network")

alive_nodes = list(range(num_nodes))
leave_nodes = []
# stupid way to get random subset of nodes
for i in range(num_nodes-num_leaves):
    i = alive_nodes[random.randrange(len(alive_nodes))]
    leave_nodes.append(i)
    alive_nodes.remove(i)

burst_leave = True


def leave(i):
    return requests.get(
        f'http://127.0.0.1:{ws_port+i}/leave')


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
print(f"INFO: restabilization took {end-start:.3f} seconds")

for p in processes:
    p.wait()
