#!/usr/bin/env python3

from itertools import groupby
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
    try:
        out = np.float32(struct.unpack('<f', struct.pack('<I', h))[0])
    except struct.error:
        out = str(h)
    return out


def format_float(f):
    """
    Format a floating point value. We need 13 digits of precision to
    unambiguously represent a 32-bit float.
    """
    return "{:.13f}".format(f)


def json_arr_apply(j, f):
    """
    Function that recursively digs into a JSON object and applies f to the innermost arrays
    """
    if isinstance(j, dict):
        for k, v in j.items():
            j[k] = json_arr_apply(v, f)
        return j
    elif isinstance(j, list):
        return f(j)
    else:
        return j


def convert_to_float32(args):
    """
    Convert a JSON file with unsigned integer arrays to float32 string representation.
    Reads from STDIN if no file is provided.
    """
    def conv(arr):
        return [int_to_float32(x) for x in arr]

    if args.file is None:
        fd = sys.stdin
    else:
        fd = open(args.file, 'r')

    j = json.load(fd)
    j = json_arr_apply(j, conv)
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
    j = json_arr_apply(j, conv)
    print(json.dumps(j, indent=2))


def all_equal(iterable):
    g = groupby(iterable)
    return next(g, True) and not next(g, False)


def check(args):
    if args.file is None:
        fd = sys.stdin
    else:
        fd = open(args.file, 'r')
    j = json.load(fd)
    for (k, v) in j[args.fields[0]].items():
        vals = [j[f][k] for f in args.fields]
        if not all_equal(vals):
            # Construct dictionary with all values
            out = {f: j[f][k] for f in args.fields}
            print(f"Mismatch for key {k}: " +
                  json.dumps(out, indent=2, default=format_float))


def random_data(args):
    """
    Generate random floating point data and print out as JSON
    """

    out = {'left': [], 'right': [], 'res': []}
    for _ in range(args.count):
        # Use numpy to generate a random float32
        left = np.float32(random.uniform(-1, 1))
        right = np.float32(random.uniform(0, 1))
        if args.op == 'add':
            res = left + right
        elif args.op == 'mul':
            res = left * right
        else:
            raise ValueError("Unknown op " + args.op)

        # Append unsigned int representation of float if --raw is provided
        if args.raw:
            out['left'].append(float32_to_int(left))
            out['right'].append(float32_to_int(right))
            out['res'].append(float32_to_int(res))
        else:
            out['left'].append(left)
            out['right'].append(right)
            out['res'].append(res)

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
    gen_parser.add_argument(
        "--op", choices=['add', 'mul'], default='add', help='Operation to perform')
    gen_parser.set_defaults(func=random_data)

    to_float_parser = subparsers.add_parser('to_float')
    to_float_parser.add_argument(
        "-f", "--file", help="JSON file to be converted")
    to_float_parser.set_defaults(func=convert_to_float32)

    to_int_parser = subparsers.add_parser('to_int')
    to_int_parser.add_argument(
        "-f", "--file", help="JSON file to be converted")
    to_int_parser.set_defaults(func=convert_to_int)

    check_parser = subparsers.add_parser('check')
    check_parser.add_argument(
        "-f", "--file", help="JSON file to be checked")
    check_parser.add_argument(
        "--fields", nargs='+', default=["verilog_nopipe", "out", "verilog_pipe"])
    check_parser.set_defaults(func=check)

    args = parser.parse_args()

    args.func(args)
