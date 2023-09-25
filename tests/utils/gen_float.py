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
    return struct.unpack("<I", struct.pack("<f", f))[0]


# Convert hex to a 32-bit floating point number
def int_to_float32(h):
    try:
        out = np.float32(struct.unpack("<f", struct.pack("<I", h))[0])
    except struct.error:
        out = str(h)
    return out


def format_float(precision):
    """
    Format a floating point value. We need 13 digits of precision to
    unambiguously represent a 32-bit float.
    """
    fmt_str = "{{:.{}f}}".format(precision)

    def fmt(f):
        return fmt_str.format(f)

    return fmt


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
        fd = open(args.file, "r")

    j = json.load(fd)
    j = json_arr_apply(j, conv)
    print(json.dumps(j, indent=2, default=format_float(args.precision)))


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
        fd = open(args.file, "r")

    j = json.load(fd)
    j = json_arr_apply(j, conv)
    print(json.dumps(j, indent=2))


def all_equal(iterable):
    g = groupby(iterable)
    return next(g, True) and not next(g, False)


def f32_as_int(x):
    bytes = struct.pack(">f", x)
    return int.from_bytes(bytes, byteorder="big")


def int_as_f32(x):
    bytes = x.to_bytes(length=4, byteorder="big")
    return struct.unpack(">f", bytes)[0]


def check(args):
    err = 0
    if args.file is None:
        fd = sys.stdin
    else:
        fd = open(args.file, "r")

    j = json.load(fd)
    for k in j[args.fields[0]].keys():
        vals = [j[f][k] for f in args.fields]
        if not all_equal(vals):
            err += 1
            # Construct dictionary with all values
            out = {f: j[f][k] for f in args.fields}
            print(
                f"Mismatch for key {k}: "
                + json.dumps(out, indent=2, default=format_float)
            )

    sys.exit(err)


def random_data(args):
    """
    Generate random floating point data and print out as JSON
    """

    # Dictionary mapping each field to a list of values
    fields = {k: [] for k in args.fields}
    if args.op != "unknown":
        fields["expected"] = []

    for _ in range(args.count):
        for k in args.fields:
            if args.op.startswith("f32"):
                v = random.uniform(*args.bounds)
                fields[k].append(f32_as_int(v))
            else:
                v = random.randint(*args.bounds)
                fields[k].append(v)

        check_fun = all_equal
        if args.op == "f32_add":
            l = int_as_f32(fields["left"][-1])
            r = int_as_f32(fields["right"][-1])
            fields["expected"][-1] = f32_as_int(np.add(l, r, dtype=np.float32))
        elif args.op == "f32_mul":
            l = int_as_f32(fields["left"][-1])
            r = int_as_f32(fields["right"][-1])
            fields["expected"][-1] = f32_as_int(np.multiply(l, r, dtype=np.float32))

    print(json.dumps(fields, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="cmd")
    subparsers.required = True

    gen_parser = subparsers.add_parser("gen")
    gen_parser.add_argument(
        "count", type=int, default=1, help="Number of random data to generate"
    )
    gen_parser.add_argument("--width", type=int, default=32)
    gen_parser.set_defaults(func=random_data)
    gen_parser.add_argument(
        "--fields", nargs="+", default=["left", "right", "expected"]
    )
    gen_parser.add_argument("--op", default="unknown", help="Operation being generated")
    gen_parser.add_argument("--bounds", nargs=2, type=int, default=[0, 2**32 - 1])

    to_float_parser = subparsers.add_parser("to_float")
    to_float_parser.add_argument("-f", "--file", help="JSON file to be converted")
    to_float_parser.add_argument(
        "-p", "--precision", type=int, default=13, help="Precision of the output"
    )
    to_float_parser.set_defaults(func=convert_to_float32)

    to_int_parser = subparsers.add_parser("to_int")
    to_int_parser.add_argument("-f", "--file", help="JSON file to be converted")
    to_int_parser.set_defaults(func=convert_to_int)

    check_parser = subparsers.add_parser("check")
    check_parser.add_argument("-f", "--file", help="JSON file to be checked")
    check_parser.add_argument(
        "--fields",
        nargs="+",
        default=["gold", "verilog_nopipe", "out", "verilog_pipe", "filament_lib"],
    )
    check_parser.set_defaults(func=check)

    args = parser.parse_args()

    args.func(args)
