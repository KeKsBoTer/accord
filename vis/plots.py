from matplotlib import pyplot as plt
import seaborn as sns
import json
import numpy as np
sns.set()

with open("../profile/stabilization_times.json", "r") as f:
    data = json.load(f)
x = [d["num_nodes"] for d in data]
mean = [np.array(d["runs"]).mean() for d in data]
var = [np.array(d["runs"]).var() for d in data]
plt.errorbar(x, mean, yerr=var, fmt="-o", label="Stabilization time")
plt.plot(x, x, "--")
plt.xlabel("Number of nodes")
plt.ylabel("Seconds")
plt.title("Stabilization times")
plt.xticks(x, x)
plt.legend()
plt.tight_layout()
plt.savefig("stabilization_times.png", dpi=300)
plt.savefig("stabilization_times.eps", dpi=300)
