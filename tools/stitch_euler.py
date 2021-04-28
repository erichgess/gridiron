#!/usr/bin/env python3

import argparse
import cbor2


def parse_args():
    parser = argparse.ArgumentParser(
        description='Stitches two or more Euler output files together into one. Checks for duplicate patches but does not check for missing patches or overlapping patches.')
    parser.add_argument('-f', '--files', nargs="+", required=True)
    args = parser.parse_args()
    return args.files


def check_for_duplicate_patches(data):
    primitives = data['primitive']

    # Convert the rects of each patch to a list of tuples
    rects = [(p['rect'][0]['start'], p['rect'][0]['end'], p['rect']
              [1]['start'], p['rect'][1]['end']) for p in primitives]
    distinct_rects = set(rects)

    return len(rects) != len(distinct_rects)


files = parse_args()
print(files)

combined = cbor2.load(open(files[0], 'rb'))

for i in range(1, len(files)):
    additional = cbor2.load(open(files[i], 'rb'))

    combined['primitive'].extend(additional['primitive'])

if check_for_duplicate_patches(combined):
    print("ERROR: There are duplicate patches in these files")
else:
    with open('stitched.cbor', 'wb') as fp:
        cbor2.dump(combined, fp)
