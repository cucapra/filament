#!/usr/bin/env python3
import argparse
import sys
import json
import re
import logging as log
import coloredlogs


START_MATCH = re.compile(r'.*KERNEL_START\s*=\s*\b(.*)\b')
END_MATCH = re.compile(r'.*KERNEL_END\s*=\s*\b(.*)\b')


def extract_sigs(file):
    """
    Extracts all signatures from a file. The format is:
    KERNEL_START=<name>
    <signature>
    ...
    <signature>
    KERNEL_END=<name>

    A signature may span multiple lines. The method returns a dictionary mapping from kernel names to lines for that signature.
    """

    with open(file, 'r') as f:
        lines = f.readlines()

    sigs = {}
    kernel_name = None
    for line in lines:
        # Check if we are starting a new kernel
        start_match = START_MATCH.match(line)
        end_match = END_MATCH.match(line)
        # print(line, start_match, end_match)
        if start_match:
            kernel_name = start_match.group(1).lower()
            sigs[kernel_name] = ""
        elif end_match:
            name = end_match.group(1).lower()
            if name != kernel_name:
                raise Exception(
                    'Mismatched kernel name: {} != {}'.format(name, kernel_name))
            else:
                kernel_name = None
        elif kernel_name is not None:
            sigs[kernel_name] += line.strip() + "\n"

    return sigs


if __name__ == "__main__":
    coloredlogs.install()
    # Accept any number of files from the commaond line
    parser = argparse.ArgumentParser(
        description='Extracts signatures from a file.')
    parser.add_argument('files', metavar='file', type=str, nargs='+',
                        help='a file to extract signatures from')
    # Number of expected signatures
    parser.add_argument('--num', type=int,
                        help='number of expected signatures')
    args = parser.parse_args()

    # Run the parser and collate all the signatures
    sigs = {}
    for file in args.files:
        # If the output from a file was empty, produce a warning on stderr
        s = extract_sigs(file)
        if len(s) == 0:
            log.warning(f'No signatures found in {file}')

        sigs.update(s)

    if args.num is not None and len(sigs) != args.num:
        raise Exception(
            f'Expected {args.num} signatures, but found {len(sigs)}')

    # Print the signatures
    print(json.dumps(sigs, indent=2))
