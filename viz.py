# script that viszualizes the chord ring

import graphviz
import requests

dot = graphviz.Digraph(comment='Chord Ring')

queue = set([8000])
visited = set()

while len(queue) > 0:
    node = queue.pop()
    dot.node(str(node), f'localhost:{node}')
    x = requests.get(f'http://localhost:{node}/neighbors')

    successors = [int(s.split(":")[1]) for s in x.json()]
    for s in successors:
        if s not in visited:
            queue.add(s)
        dot.edge(str(node), str(s))
    visited.add(node)
    print("visisted", node)

dot.render('ring.gv', view=True)
