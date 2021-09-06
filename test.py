import subprocess
import time

entry_node = 9000

processes = [
    subprocess.Popen(["./target/debug/accord", f"127.0.0.1:{entry_node}"])
]

time.sleep(2)

for i in range(1, 8+1):
    p = subprocess.Popen(["./target/debug/accord", f"127.0.0.1:{entry_node+i}",
                          "--entry-node", f"127.0.0.1:{entry_node}"])

    processes.append(p)

for p in processes:
    p.wait()
