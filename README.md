# Accord

Chord implementation in rust.

## Build

Install rust first.
Go into project directory and run the following:

```bash
$ cargo build
```

## Create Network

Install python and run the `create_network.py` script to create a network with a defined number of nodes.
Use script as following:

```bash
$ python3 create_network.py --help
usage: network_creator [-h] [--chord-port-start CHORD_PORT] [--api-port-start WS_PORT] [--stabilization-period STABILIZATION_PERIOD]
                       [--num-leaves NUM_LEAVES] [--log-level LOG_LEVEL] [--quit-after-stabilization]
                       num_nodes

creates a chord network

positional arguments:
  num_nodes             number of nodes in the network

optional arguments:
  -h, --help            show this help message and exit
  --chord-port-start CHORD_PORT
                        smallest port for chord network
  --api-port-start WS_PORT
                        smallest port for HTTP API
  --stabilization-period STABILIZATION_PERIOD
                        delay beteween stabilizatios
  --num-leaves NUM_LEAVES
                        number of nodes that leave the network after stabilization (test)
  --log-level LOG_LEVEL
                        log level (DEBUG prints chord binary output)
  --quit-after-stabilization
                        quit program after initial stabilization
```
