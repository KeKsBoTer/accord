# script that viszualizes the chord ring

import graphviz
import requests

dot = graphviz.Digraph(comment='Chord Ring')

queue = set([8001])
visited = set()

while len(queue) > 0:
    node = queue.pop()
    dot.node(str(node), f'localhost:{node}')
    x = requests.get(f'http://localhost:{node}/node-info')

    successor = int(x.json()["successor"].split(":")[1])
    if successor not in visited:
        queue.add(successor)
    dot.edge(str(node), str(successor))
    visited.add(node)
    print("visisted", node)

dot.render('ring.gv', view=True)
