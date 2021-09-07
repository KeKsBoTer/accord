import subprocess
import time

entry_node = 9000
ws_port = 8080

processes = [
    subprocess.Popen(
        ["./target/debug/accord", f"127.0.0.1:{entry_node}", f"127.0.0.1:{ws_port}", ])
]

time.sleep(2)

for i in range(1, 8+1):
    p = subprocess.Popen(["./target/debug/accord",
                          f"127.0.0.1:{entry_node+i}",
                          f"127.0.0.1:{ws_port+i}",
                          "--entry-node", f"127.0.0.1:{entry_node}"])

    processes.append(p)

for p in processes:
    p.wait()
