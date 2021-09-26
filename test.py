import subprocess
import time
from typing import Hashable

entry_node = 9000
ws_port = 8008


def hash_ip(ip):
    return int(subprocess.run(["./target/debug/hasher",
                               ip], stdout=subprocess.PIPE).stdout)


nodes = [entry_node+i for i in range(16)]

sorted_nodes = sorted(nodes, key=lambda x: hash_ip(f"127.0.0.1:{x}"))

print(sorted_nodes)

processes = []

for i in range(0, len(nodes)):
    n = sorted_nodes[i]
    pred = sorted_nodes[(i-1) % len(sorted_nodes)]
    suc = sorted_nodes[(i+1) % len(sorted_nodes)]
    cmd = ["./target/debug/accord",
           # own ip
           f"127.0.0.1:{n}",
           f"127.0.0.1:{n-1000}",
           # predecessor
           f"127.0.0.1:{pred}",
           f"127.0.0.1:{pred-1000}",
           # successor
           f"127.0.0.1:{suc}",
           f"127.0.0.1:{suc-1000}", ]
    p = subprocess.Popen(cmd)

    processes.append(p)

print("started all!")
for p in processes:
    p.wait()
