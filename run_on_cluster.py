import subprocess
import sys
import random
import socket
import requests


def hash_ip(ip):
    return int(subprocess.run(["./target/debug/hasher",
                               ip], stdout=subprocess.PIPE).stdout)

cmd = ["/share/apps/ifi/available-nodes.sh"]
MyOut = subprocess.Popen(
    cmd,
    stdout=subprocess.PIPE,
    stderr=subprocess.STDOUT
)
stdout,stderr = MyOut.communicate()

available_nodes = str(stdout, 'utf-8').split('\n')[:-1] #last one is emty line
random.shuffle(available_nodes)
available_ips = list(map((lambda x : socket.gethostbyname(x)), available_nodes))

#nodes = [entry_node+i for i in range(int(sys.argv[1]))]
nodes = available_ips[:int(sys.argv[1])]

#port = 65343
port = random.randint(59152,65535)

sorted_nodes = sorted(nodes, key=lambda x: hash_ip(f"{x}:{port}"))
print(sorted_nodes)
print("nodes here")
print(f"ports = {port}, {port-10000}")

processes = []

for i in range(0, len(nodes)):
    n = sorted_nodes[i]
    pred = sorted_nodes[(i-1) % len(sorted_nodes)]
    suc = sorted_nodes[(i+1) % len(sorted_nodes)]
    eachnodecmd = ["~/accord/target/debug/accord",
           # own ip
           f"{n}:{port}",
           f"{n}:{port-10000}",
           # predecessor
           f"{pred}:{port}",
           f"{pred}:{port-10000}",
           # successor
           f"{suc}:{port}",
           f"{suc}:{port-10000}", ]
    cmd = [
        f"ssh",
        "-f",
        f"{n}",
        " ".join(eachnodecmd)]
    #print(cmd)
    p = subprocess.Popen(cmd)

    processes.append(p)

print("started all!")
for p in processes:
    p.wait()


# print("use just an index")
# while True:
#     print(sorted_nodes)
#     nodetotest = int(input())
#     print(f"testing on node http://{sorted_nodes[nodetotest]}:{port-10000}/neighbors")
#     try:
#         x = requests.get(f'http://{sorted_nodes[nodetotest]}:{port-10000}/neighbors')
#         print(x.content)
#         print(f"http://{sorted_nodes[nodetotest]}:{port-10000}/neighbors")
#     except Exception as e:
#         print(f"{str(e)} on {sorted_nodes[nodetotest]}:{port-10000}")
