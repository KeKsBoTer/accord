import socket
import subprocess
import random
import time

portspace = (49152, 65535)
##
randomfixedport = random.randint(portspace)
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

print(type(stdout))
arr = str(stdout, 'utf-8').split('\n')[:-1]#last one is
print(len(arr))
for item in arr:
    print(socket.gethostbyname(item),":",randomfixedport)
