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
            results.append((row['event'], int(row['id']), int(
                row['start']), int(row['stop'])))
            line_count += 1

    return results


def extract_results_by_event(event, results):
    work = [(id, start, stop)
            for (ty, id, start, stop) in results if ty == event]
    return work


def extract_results_by_id(id, results):
    results = [(i, start, stop)
               for (i, start, stop) in results if i == id]
    return results


def extract_work_results(results):
    return extract_results_by_event('work', results)


def extract_network_results(results):
    return extract_results_by_event('network', results)


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


def extract_network(results):
    x = []
    for (_, start, stop) in results:
        x.append([[start, stop], [0, 0]])
    return x


def extract_work_lines(y, results):
    x = []
    for (_, start, stop) in results:
        x.append([[start, stop], [y, y]])
    return x


def chart_durations(files):
    plt.suptitle('Microseconds/Patch Above Waiting For Remotes')

    # configure the chart layout to have at most 3 columns
    cols = min(1, len(files))
    rows = len(files) // cols + min(1, len(files) % cols)

    subplot = 1
    for f in files:
        plt.subplot(rows, cols, subplot)
        subplot += 1

        results = read_results(f)
        work = extract_work_results(results)
        network = extract_network_results(results)

        x, y = extract_duration(work)
        plt.plot(x, y, 'o', mfc='none')

        x = extract_network(network)
        for idx in range(0, len(x)):
            plt.plot(x[idx][0], x[idx][1], '-^', mfc='none')

    plt.legend()
    plt.show()


def chart_work_periods(files):
    plt.suptitle('Time Spent On Patch\nAbove\nWaiting For Remotes')

    # configure the chart layout to have at most 3 columns
    cols = min(1, len(files))
    rows = len(files) // cols + min(1, len(files) % cols)

    subplot = 1
    for f in files:
        plt.subplot(rows, cols, subplot)
        subplot += 1

        # Read metric data
        results = read_results(f)

        # Chart Time Spent Working On Patches
        work = extract_work_results(results)
        ids = extract_ids(work)
        for idx in range(0, len(ids)):
            worker_results = extract_results_by_id(ids[idx], work)

            x = extract_work_lines(idx + 2, worker_results)
            for idx in range(0, len(x)):
                plt.plot(x[idx][0], x[idx][1], '-', mfc='none')

        # Chart Time Spent Waiting For Remotes
        network = extract_network_results(results)
        x = extract_network(network)
        for idx in range(0, len(x)):
            plt.plot(x[idx][0], x[idx][1], '-^', mfc='none')

    plt.legend()
    plt.show()


# parse CLI arguments
args = parse_args()

# Load CSV test results
files = args['input']
# chart_durations(files)
chart_work_periods(files)
