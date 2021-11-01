import subprocess
import json
from tqdm import tqdm
nodes = [2, 4, 8, 16, 25, 32, 50, 64, 96, 100]

results = []

for n in tqdm(nodes):
    runs = []
    for i in range(3):
        try:
            result = subprocess.run(["python3", "create_network.py", str(
                n), "--quit-after-stabilization"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            last_line = result.stderr.splitlines()[-1]
            duration = float(last_line.decode("utf-8").split(" ")[-2])
            runs.append(duration)
        except Exception:
            pass
    results.append({
        "num_nodes": n,
        "runs": runs
    })
    with open("profile/stabilization_times.json", "w") as f:
        json.dump(results, f)

print(results)
with open("profile/stabilization_times.json", "w") as f:
    json.dump(results, f)
