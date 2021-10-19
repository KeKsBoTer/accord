import socket
import subprocess
import random
import time
import requests


portspace = (49152, 65535)
##
randomfixedport = random.randint(*portspace)
##
max_node_number = 64

cmd = ["/share/apps/ifi/available-nodes.sh"]
MyOut = subprocess.Popen(
    cmd,
    stdout=subprocess.PIPE,
    stderr=subprocess.STDOUT,
    bufsize=1, universal_newlines=True
)

stdout,stderr = MyOut.communicate()
# print(stdout)
# print(stderr)

available_nodes = str(stdout, 'utf-8').split('\n')[:-1] #last one is emty line
random.shuffle(available_nodes)
available_ips = list(map((lambda x : socket.gethostbyname(x)), available_nodes))

#print(available_nodes)
#print(available_ips)
## fixed for testing
entry_node_p = 62222
ws_port = 52222

## nodes can enter in a ordered way, to speed things up bootom oof for
##  test fixed entry node


accordpath = "~/accord/target/debug/accord"

processes = [
    subprocess.Popen([
        f"ssh",
        f"-f",
        f"{available_nodes[0]}",
        f"{accordpath} {available_ips[0]}:{entry_node_p} {available_ips[0]}:{ws_port}"],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        bufsize=1, universal_newlines=True
        )
]


for i in range(1,(len(available_nodes[1:9])-1)):
    time.sleep(1)
    print(f"process = ssh -f {available_nodes[i]} {accordpath} {available_ips[i]}:{entry_node_p} {available_ips[i]}:{ws_port} --entry-node {available_ips[i-1]}:{entry_node_p}")
    #print(f"{socket.gethostbyname()}:{random.randint(*portspace)}")
    p = subprocess.Popen([
        f"ssh",
        f"-f",
        f"{available_nodes[i]}",
        f"{accordpath} {available_ips[i]}:{entry_node_p} {available_ips[i]}:{ws_port} --entry-node {available_ips[i-1]}:{entry_node_p}"],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT
        )
        # available_nodes[i] would do the trick.
    processes.append(p)

for p in processes:
    p.wait()

# test tloop
print("use just an index")
while True:
    print(available_ips[0:9])
    node = int(input())
    port = ws_port
    try:
        x = requests.get(f'http://{available_ips[node]}:{port}/neighbors')
        print(x.content)

        print(f"http://{available_ips[node]}:{port}/neighbors")
    except Exception as e:
        print(f"{str(e)} on {available_ips[node]}:{port}")
