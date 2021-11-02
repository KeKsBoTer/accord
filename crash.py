from typing import List
from matplotlib import pyplot as plt
import seaborn as sns
import random

sns.set()


def has_successive_fail(node: List[bool]):
    if node[0] and node[-1]:
        return True
    for a, b in zip(node, node[1:]):
        if a and b:
            return True
    return False


def random_fails(num_nodes: int, fails: int):
    nodes = [False] * num_nodes
    while fails > 0:
        i = random.randrange(num_nodes)
        if not nodes[i]:
            nodes[i] = True
            fails -= 1
    return nodes


def avg_fail(n, failes):
    crashes = 0
    tests = 10000
    for i in range(tests):
        nodes = random_fails(n, failes)
        if has_successive_fail(nodes):
            crashes += 1
    return crashes/tests


data50 = [(f, avg_fail(50, f)) for f in range(0, 15)]
data25 = [(f, avg_fail(25, f)) for f in range(0, 15)]
data10 = [(f, avg_fail(10, f)) for f in range(0, 10)]

plt.plot([f for f, d in data50], [d for f, d in data50], label="50 nodes")
plt.plot([f for f, d in data25], [d for f, d in data25], label="25 nodes")
plt.plot([f for f, d in data10], [d for f, d in data10], label="10 nodes")
plt.legend()
plt.xlabel("Number of crashes")
plt.ylabel("Network failure probability")
# plt.show()
plt.savefig("vis/failure_proba.png")
plt.savefig("vis/failure_proba.eps")
