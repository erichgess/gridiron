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

    ap.add_argument('-i', '--input', nargs='+', required=True,
                    help='CSV file containing benchmarch results from running GridIron.')
    ap_args = vars(ap.parse_args())
    return {
        'input': ap_args['input'],
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
    results = []
    with open(file) as csv_file:
        csv_reader = csv.DictReader(csv_file, delimiter=',')
        line_count = 0
        for row in csv_reader:
            results.append((int(row['id']), int(
                row['start']), int(row['stop'])))
            line_count += 1

    return results


def extract_ids(results):
    ids = [id for (id, _, _) in results]
    return list(set(ids))


def extract_duration(results):
    x = [start for (_, start, _) in results]
    y = [(stop - start) for (_, start, stop) in results]
    return x, y


def extract_id_duration(id, results):
    nth = 1
    id_results = [(i, s, e) for (i, s, e) in results if i == id]
    x = [start for (i, start, _) in id_results[::nth] if i == id]
    y = [(stop - start) for (i, start, stop) in id_results[::nth] if i == id]
    return x, y


# parse CLI arguments
args = parse_args()

# Load CSV test results
files = args['input']

sys_info = get_sys_info()
plt.suptitle('CPU:{}\nMemory: {}'.format(
    sys_info['cpu']['name'], sys_info['total_memory']))

# configure the chart layout to have at most 3 columns
cols = min(3, len(files))
rows = len(files) // cols + min(1, len(files) % cols)

subplot = 1
for f in files:
    plt.subplot(rows, cols, subplot)
    subplot += 1

    results = read_results(f)
    ids = extract_ids(results)

    x, y = extract_duration(results)
    plt.plot(x, y, 'o', mfc='none')

plt.legend()
plt.show()
