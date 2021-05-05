#!/usr/bin/env python3

import argparse
import subprocess
import time
import glob
import os


def parse_args():
    parser = argparse.ArgumentParser(
        description='Runs an Euler cluster with the specified number of nodes.  Default is 1.')
    parser.add_argument('-n', '--number', default="1", required=True)
    args = parser.parse_args()
    return int(args.number)


def check_for_old_results():
    cbor_files = glob.glob("./*.cbor")
    return len(cbor_files) > 0


def get_result_files():
    results = glob.glob("./state-rank-*.cbor")
    return results


def stitch_results(results):
    cmd = ["./tools/stitch_euler.py", "-f {}".format(str.join(" ", results))]
    run_cmd(cmd).wait()
    os.rename("./stitched.cbor", "./state.cbor")


def show_chart():
    cmd = ["./tools/plot_euler.py"]
    run_cmd(cmd).wait()


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

# If old results are in the directory don't run
if check_for_old_results():
    print("There are still old CBOR files from previous runs.  Please delete this files so that there is no contamination or accidental loss of old test results")
    exit()

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

results = get_result_files()
stitch_results(results)
show_chart()
