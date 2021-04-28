#!/usr/bin/env python3

import argparse
import cbor2


def parse_args():
    parser = argparse.ArgumentParser(
        description='Stitches two or more Euler output files together into one')
    parser.add_argument('-f', '--files', nargs="+", required=True)
    args = parser.parse_args()
    return args.files


files = parse_args()
print(files)

combined = cbor2.load(open(files[0], 'rb'))

for i in range(1, len(files)):
    additional = cbor2.load(open(files[i], 'rb'))

    combined['primitive'].extend(additional['primitive'])

with open('stitched.cbor', 'wb') as fp:
    cbor2.dump(combined, fp)
