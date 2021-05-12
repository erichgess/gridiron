#!/usr/bin/env python3

import argparse
import subprocess
import time
import glob
import os


def parse_args():
    parser = argparse.ArgumentParser(
        description='Runs an Euler cluster with the specified number of nodes.  Default is 1.')
    parser.add_argument('-p', '--peers', default="1", required=True)
    parser.add_argument('-f', '--folds', default="1")
    parser.add_argument('-t', '--threads', default="1")
    parser.add_argument('--tfinal', default="0.1")
    parser.add_argument('--gui', default=False, action='store_true')
    args = parser.parse_args()
    return {
        'peers': int(args.peers),
        'folds': int(args.folds),
        'threads': int(args.threads),
        'tfinal': float(args.tfinal),
        'gui': args.gui,
    }


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


def euler_cmd(t, n, b, folds, tfinal, peers, rank):
    return [
        "cargo", "run", "--release", "--example",
        "euler", "--",
        "-t", str(t),
        "-n", str(n),
        "-b", str(b),
        "-f", str(folds),
        "--tfinal", str(tfinal),
        "--strategy", "rayon",
        "--peers", str.join(" ", peers),
        "--rank", str(rank), ]


def run_cmd(cmd):
    return subprocess.Popen(str.join(" ", cmd), shell=True)


args = parse_args()

# If old results are in the directory don't run
if check_for_old_results():
    print("There are still old CBOR files from previous runs.  Please delete this files so that there is no contamination or accidental loss of old test results")
    exit()

peers = ["127.0.0.1:{}".format(8000 + i) for i in range(0, args['peers'])]
print(peers)

cmds = [
    euler_cmd(args['threads'], 1000, 100, args['folds'],
              args['tfinal'], peers, rank)
    for rank in range(0, args['peers'])
]

print(cmds)
out = []
for c in cmds:
    out.append(run_cmd(c))

[o.wait() for o in out]

if args['gui']:
    results = get_result_files()
    stitch_results(results)
    show_chart()
