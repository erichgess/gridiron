#!/usr/bin/env python3

import argparse
import csv
import math
import platform
import psutil
import cpuinfo
import matplotlib.pyplot as plt


def parse_args():
    ap = argparse.ArgumentParser(
        description='Draws charts comparing benchmark results from euler')

    ap.add_argument('-i', '--input', required=True,
                    help='CSV file containing benchmarch results from test.py.')
    ap.add_argument('-b', '--blocks', required=False,
                    help='Only chart the given block sizes. The default is to chart all blocks.')
    ap.add_argument('-g', '--grids', required=False,
                    help='Only chart the given grids. The default is to chart all grids.')
    ap_args = vars(ap.parse_args())
    blocks = None
    if ap_args['blocks'] is not None:
        blocks = [int(b) for b in ap_args['blocks'].split(',')]
    grids = None
    if ap_args['grids'] is not None:
        grids = [int(b) for b in ap_args['grids'].split(',')]
    return {
        'input': ap_args['input'],
        'blocks': blocks,
        'grids': grids,
    }


def fmt_mem_size(bytes, suffix="B"):
    """
    Scale bytes to its proper format
    e.g:
        1253656 => '1.20MB'
        1253656678 => '1.17GB'
    """
    factor = 1024
    for unit in ["", "K", "M", "G", "T", "P"]:
        if bytes < factor:
            return f"{bytes:.2f}{unit}{suffix}"
        bytes /= factor


def get_sys_info():
    phys_cores = psutil.cpu_count(logical=False)
    log_cores = psutil.cpu_count(logical=True)
    cpu_freq = psutil.cpu_freq()
    cpu_freq = cpu_freq.max
    vmem = psutil.virtual_memory()
    total_mem = fmt_mem_size(vmem.total)
    return {
        'cpu': {
            'name': cpuinfo.get_cpu_info()['brand_raw'],
            'cores': {
                'logical': log_cores,
                'physical': phys_cores,
            },
            'freq': cpu_freq
        },
        'total_memory': total_mem,
    }


def read_results(file):
    results = {}
    with open(file) as csv_file:
        csv_reader = csv.DictReader(csv_file, delimiter=',')
        line_count = 0
        for row in csv_reader:
            if int(row['GridSize']) not in results:
                results[int(row['GridSize'])] = []
            results[int(row['GridSize'])].append({
                't': int(row['NumThreads']),
                'p': int(row['BlockSize']),
                'sec': float(row['TotalSec'])})
            line_count += 1

    return results


def extract_ts(results, grid, block):
    g = results[grid]
    line = [[row['t'], row['sec']] for row in g if row['p'] == block]
    line.sort()
    x = [a[0] for a in line]
    y = [a[1] for a in line]
    return x, y


# parse CLI arguments
args = parse_args()

# Load CSV test results
source_csv = args['input']
results = read_results(source_csv)

# construct the set of block sizes and grid sizes
patch_sz = set()
grid_sz = set()
for k in results:
    for row in results[k]:
        patch_sz.add(row['p'])
        grid_sz.add(k)

# restrict the patches that will be charted to those given in the `blocks`
# argument
if args['blocks'] is not None:
    user_blocks = set(args['blocks'])
    patch_sz = patch_sz.intersection(user_blocks)

patch_sz = list(patch_sz)
patch_sz.sort()

# restrict the grids that will be charted to those given in the `grid`
# argument
if args['grids'] is not None:
    user_grids = set(args['grids'])
    grid_sz = grid_sz.intersection(user_grids)

grid_sz = list(grid_sz)
grid_sz.sort()

# configure the chart layout to have at most 3 columns
cols = min(3, len(grid_sz))
rows = len(grid_sz) // cols + min(1, len(grid_sz) % cols)

sys_info = get_sys_info()
plt.suptitle('CPU:{}\nMemory: {}'.format(
    sys_info['cpu']['name'], sys_info['total_memory']))

subplot = 1
for g in grid_sz:
    plt.subplot(rows, cols, subplot)
    subplot += 1
    for p in patch_sz:
        x, y = extract_ts(results, g, p)
        r = [1 / yi for yi in y]
        ideal = [r[0] * xi / x[0] for xi in x]
        plt.plot(x, r, '-o', mfc='none', label='N={} B={}'.format(g, p))
        plt.xlabel('Threads')
        plt.ylabel('Rate [1/s]')

plt.plot(x, ideal, '--', lw='1.0', c='k', label='Ideal')
plt.legend()
plt.show()
