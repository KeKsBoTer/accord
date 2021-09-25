import socket
import subprocess
import random
import time

portspace = (49152, 65535)
##
randomfixedport = random.randint(*portspace)
##
max_node_number = 64

cmd = ["/share/apps/ifi/available-nodes.sh"]
MyOut = subprocess.Popen(
    cmd,
    stdout=subprocess.PIPE,
    stderr=subprocess.STDOUT
)

stdout,stderr = MyOut.communicate()
# print(stdout)
# print(stderr)

available_nodes = str(stdout, 'utf-8').split('\n')[:-1] #last one is
random.shuffle(available_nodes)

entry_node_p = 62222
ws_port = 52222


## nodes can enter in a ordered way, to speed things up bootom oof for
entry_node = available_nodes[0]
for i in range(0,(len(available_nodes[1:5])-1)):
    time.sleep(1)
    #print(f"{socket.gethostbyname()}:{random.randint(*portspace)}")
    p = subprocess.Popen([
        f"ssh", 
        f"-f",
        f"{i}",
        "\"",
            "./target/debug/accord",
            f"{available_nodes[i+1]}:{entry_node_p}"
            f"{available_nodes}:{entry_node_p}",
            f"{available_nodes}:{ws_port}",
            "--entry-node", f"{available_nodes[0]}:{entry_node_p}","\""]
        )
            # available_nodes[i] would do the trick.

    processes.append(p)
for p in processes:
    p.wait()
