#!/usr/bin/env python3

import argparse
import subprocess
import time
import glob
import os


def euler_cmd(t, n, b, folds, peers, rank):
    return [
        "cargo", "run", "--release", "--example",
        "euler", "--",
        "-t", str(t),
        "-n", str(n),
        "-b", str(b),
        "-f", str(folds),
        "--strategy", "rayon",
        "--peers", str.join(" ", peers),
        "--rank", str(rank), ]


def run_cmd(cmd):
    return subprocess.Popen(str.join(" ", cmd), shell=True)


def generate_peers(n):
    peers = ["127.0.0.1:{}".format(8000 + i) for i in range(0, n)]
    return peers


def run_test(num_peers, num_threads):
    peers = generate_peers(num_peers)
    cmds = [
        euler_cmd(num_threads, 1000, 100, 1, peers, rank)
        for rank in range(0, len(peers))
    ]

    out = []
    for c in cmds:
        out.append(run_cmd(c))

    [o.wait() for o in out]


test = {
    1: [2, 3, 4, 5, 6, 7, 8],
    2: [2, 3, 4, 5, 6, 7, 8],
    3: [2, 3, 4, 5, 6, 7, 8],
    4: [2, 3, 4, 5, 6, 7, 8],
}

times = []
for p in test:
    for t in test[p]:
        start_time = time.time()
        run_test(p, t)
        elapsed_time = time.time() - start_time
        times.append((p, t, elapsed_time))

print("peers,threads,duration")
for (p, t, d) in times:
    print("{},{},{}".format(p, t, d))
