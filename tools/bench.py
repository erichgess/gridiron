#!/usr/bin/env python

import subprocess
import time

def euler_cmd(t, n, b):
    return [
        "cargo", "-q", "run", "--release", "--example",
        "euler", "--",
        "-t", str(t),
        "-n", str(n),
        "-b", str(b),
        "--strategy", "stupid",
        "--tfinal", str(0.02)]


def run_cmd(cmd):
    out = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    stdout, stderr = out.communicate()
    return stdout, stderr


threads = range(2, 29)
grid = [2400]
patch = [50, 100, 200]

print("NumThreads,GridSize,BlockSize,TotalSec")
for n in grid:
    for b in patch:
        for t in threads:
            ec = euler_cmd(t, n, b)
            start = time.time()
            out, err = run_cmd(ec)
            if err:
                print(err)
                exit()
            sec = time.time() - start
            print('{},{},{},{}'.format(t, n, b, sec))
