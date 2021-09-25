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


for item in available_nodes[:5]:
    print(socket.gethostbyname(item),":",random.randint(*portspace))
    """processes = [
    subprocess.Popen(
        ["./target/debug/accord", f"127.0.0.1:{entry_node}", f"127.0.0.1:{ws_port}", ])
]"""

