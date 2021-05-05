#!/usr/bin/env python3

import argparse
import subprocess
import time


def parse_args():
    parser = argparse.ArgumentParser(
        description='Runs an Euler cluster with the specified number of nodes.  Default is 1.')
    parser.add_argument('-n', '--number', default="1", required=True)
    args = parser.parse_args()
    return int(args.number)


def euler_cmd(t, n, b, peers, rank):
    return [
        "cargo", "run", "--release", "--example",
        "euler", "--",
        "-t", str(t),
        "-n", str(n),
        "-b", str(b),
        "--strategy", "rayon",
        "--peers", str.join(" ", peers),
        "--rank", str(rank), ]


def run_cmd(cmd):
    return subprocess.Popen(str.join(" ", cmd), shell=True)


number = parse_args()

peers = ["127.0.0.1:{}".format(8000 + i) for i in range(0, number)]
print(peers)

cmds = [
    euler_cmd(4, 1000, 100, peers, rank)
    for rank in range(0, number)
]

print(cmds)
out = []
for c in cmds:
    out.append(run_cmd(c))

[o.wait() for o in out]
