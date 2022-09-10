#!/usr/bin/env python3

import struct
import random
import numpy as np
import json
import argparse
import sys


# Generate hex representation of a 32-bit float
def float32_to_int(f):
    return struct.unpack('<I', struct.pack('<f', f))[0]


# Convert hex to a 32-bit floating point number
def int_to_float32(h):
    return struct.unpack('<f', struct.pack('<I', h))[0]


def format_float(f):
    """
    Format a floating point value. We need 13 digits of precision to
    unambiguously represent a 32-bit float.
    """
    return "{:.13f}".format(f)


def convert_to_float32(args):
    """
    Convert a JSON file with unsigned integer arrays to float32 string representation.
    Reads from STDIN if no file is provided.
    """
    def conv(arr):
        return [np.float32(int_to_float32(x)) for x in arr]

    if args.file is None:
        fd = sys.stdin
    else:
        fd = open(args.file, 'r')

    j = json.load(fd)
    j['left'] = conv(j['left'])
    j['right'] = conv(j['right'])
    j['sum'] = conv(j['sum'])
    print(json.dumps(j, indent=2, default=format_float))


def convert_to_int(args):
    """
    Convert a JSON file where each array contains strings representing 32-bit
    floating point values into unsigned integers
    """
    def conv(arr):
        return [float32_to_int(np.float32(x)) for x in arr]

    if args.file is None:
        fd = sys.stdin
    else:
        fd = open(args.file, 'r')

    j = json.load(fd)
    j['left'] = conv(j['left'])
    j['right'] = conv(j['right'])
    j['sum'] = conv(j['sum'])
    print(json.dumps(j, indent=2))


def random_data(args):
    """
    Generate random floating point data and print out as JSON
    """
    out = {'left': [], 'right': [], 'sum': []}
    for _ in range(args.count):
        # Use numpy to generate a random float32
        left = np.float32(random.uniform(-100000, 1000000))
        right = np.float32(random.uniform(-100000, 1000000))
        sum = left + right

        # Append unsigned int representation of float if --raw is provided
        if args.raw:
            out['left'].append(float32_to_int(left))
            out['right'].append(float32_to_int(right))
            out['sum'].append(float32_to_int(sum))
        else:
            out['left'].append(left)
            out['right'].append(right)
            out['sum'].append(sum)

    if args.raw:
        print(json.dumps(out, indent=2))
    else:
        print(json.dumps(out, indent=2, default=format_float))


if __name__ == '__main__':

    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest='cmd')
    subparsers.required = True

    gen_parser = subparsers.add_parser('gen')
    gen_parser.add_argument("count", type=int, default=1,
                            help='Number of random data to generate')
    gen_parser.add_argument("--raw", action='store_true')
    gen_parser.set_defaults(func=random_data)

    to_float_parser = subparsers.add_parser('to_float')
    to_float_parser.add_argument(
        "-f", "--file", help="JSON file to be converted")
    to_float_parser.set_defaults(func=convert_to_float32)

    to_int_parser = subparsers.add_parser('to_int')
    to_int_parser.add_argument(
        "-f", "--file", help="JSON file to be converted")
    to_int_parser.set_defaults(func=convert_to_int)

    args = parser.parse_args()

    args.func(args)
