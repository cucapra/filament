#!/usr/bin/env python3

import argparse
from functools import reduce
from itertools import chain
import json
import os
from pathlib import Path
from random import randint
import subprocess
import sys
from golden import *

"""
Converts a matrix into a flattened representation, then into bits, which is then read as an int
"""
def matrix_to_int_repr(matrix, width):
    flattened = list(chain.from_iterable(matrix))
    bin_str = reduce(lambda acc, i: acc + conv_int_to_bits(i, int(width)), flattened, "")
    try: 
        return int(bin_str, 2)
    except Exception as e:
        print(f"exception {e}")
        print(f"matrix: {matrix}")
        pass
"""
Overflow will happen in Filament design if result of a computation cannot fit in given bitwidth, so 
this function simulates that
"""
def conv_int_to_bits(i, width):
    # while i > pow(2, width)-1:
    #     i = i - pow(2,width)
    return bin(int(i) % 2**width)[2:].zfill(width)

"""
Converts a json file of matrices and vectors to plain old ints, for passing into designs/comparing output
"""
def convert_json(input_json, width):
    out_json = {}
    for k in input_json:
        if type(input_json[k][0]) is list:
            if type(input_json[k][0][0]) is list:
                # matrix
                converted = reduce(lambda acc, elt: acc + [matrix_to_int_repr(elt, width)], input_json[k], [])
                out_json[k] = converted
            else:
                # vector
                raise Exception("Unknown data format")
        else:
            # just a regular number
            out_json[k] = reduce(lambda acc, elt: acc + [elt % 2**int(width)], input_json[k], [])
    return out_json

"""
Separates result by input number to match cocotb output format
"""
def convert_expect(expects):
    new_expects = {}
    for key in expects:
        inner_json = {}
        key_len = len(expects[key])
        for i in range(key_len):
            inner_json[str(i)] = [expects[key][i]]
        new_expects[key] = inner_json
    return new_expects

"""
Randomly generates one test based on the configuration given in `port_info`.
"""
def generate_random_tests(port_info, width, num_tests):
    test_json = {}
    # port_info = json.load(input_file)
    inputs = port_info.get("inputs", {})
    for k in inputs:
        test_json[k] = []
        for _ in range(num_tests):
            info = inputs.get(k, {})
            type = info.get("type", {})
            if type == "matrix":
                [m,n] = info.get("dim",{})
                matrix_acc = []
                for i in range(m):
                    row_acc = []
                    for j in range(n):
                        row_acc.append(randint(0, pow(2,width)-1))
                    matrix_acc.append(row_acc)
                test_json[k] = test_json[k] + [matrix_acc]
            # elif type == "vector":
            #     [n] = info.get("dim", {})
            #     vec_acc = []
            #     for i in range(n):
            #         vec_acc.append(randint(0, pow(2,width)-1))
            #     test_json[k] = test_json[k] + [vec_acc]
            elif type == "scalar":
                test_json[k] = test_json[k] + [randint(0,pow(2, width)-1)]
            else:
                raise Exception(f"Unknown data type {type}")
    return test_json   

"""
Given an input file, uses f to compute expected output
"""
def get_golden_output(func, ports, data):
    result_json = {}
    input_ports = ports.get("inputs",{})
    output_ports = ports.get("outputs", {})

    # get length of data arrays
    port_keys = list(input_ports.keys())
    length = len(data[port_keys[0]])

    input_list = []
    for i in range(length):
        single_inputs = []
        for k in input_ports:
            single_inputs.append(data[k][i])
        input_list.append(single_inputs)

    for k in output_ports:
        result_json[k] = []

    for elt in input_list:
        res = func(elt)
        out_port_keys = list(output_ports.keys())
        for i in range(len(out_port_keys)):
            result_json[out_port_keys[i]] = result_json[out_port_keys[i]] + [res[i]]
    
    return result_json

def process_fud_output(fud_output):
    processed = {}
    for k in fud_output:
        if k != "cycles":
            processed[k] = fud_output[k]
    return processed

def run_fud(kernel):
    open(kernel+".out", 'w')
    basename,_ = os.path.splitext(kernel)

    print(" ".join([
        "fud",
        "e",
        "--to",
        "cocotb-out",
        kernel,
        "-s",
        "cocotb.data",
       kernel+".data",
        "-s",
        "calyx.flags",
        "' -d canonicalize'",
        "-s",
        "filament.flags",
        f" --bindings {basename}.toml"
        ]))
    subprocess.run([
        "fud",
        "e",
        "--to",
        "cocotb-out",
        kernel,
        "-s",
        "cocotb.data",
       kernel+".data",
        "-s",
        "calyx.flags",
        "' -d canonicalize'",
        "-s",
        "filament.flags",
        f" --bindings {basename}.toml"
        ], stdout=open(kernel+".out",'w'), stderr=subprocess.DEVNULL
    )

    with open(kernel+".out") as f:
        fud_out = json.load(f)

    processed_fud_out = str(process_fud_output(fud_out)).replace("\'", "\"")
    with open(kernel+".out", 'w') as f:
        f.write(processed_fud_out)

def main():
    parser = argparse.ArgumentParser(description="Parse arguments for generating tests")
    parser.add_argument("-d", "--data")
    parser.add_argument("-w", "--width")
    parser.add_argument("-k", "--kernel")
    parser.add_argument("-r", "--rand",action="store_true")
    parser.add_argument("-n", "--numtests")
    parser.add_argument("-c", "--check", action="store_true")
    args = parser.parse_args()

    path = os.path.normpath(args.kernel)
    called_func = path.split(os.sep)[2]

    data = args.data
    width = int(args.width)
    numtests = int(args.numtests)
    dir = os.path.dirname(args.kernel)
    ports = os.path.join(dir, "ports.json")
    
    if width is None:
        raise Exception("Requires specified bitwidth")
    
    # if -r is given, then ignore -d and generate random tests
    tests_to_run = None
    tests_to_check = None
    if args.rand:
        rand_tests = generate_random_tests(json.load(open(ports)), width, numtests)
        input_data_name = os.path.join(dir, 'test.fil.data')
        converted_tests = convert_json(rand_tests, width)
        tests_to_run = str(converted_tests).replace("\'","\"")
        tests_to_check = rand_tests
        with open(input_data_name, 'w') as f:
            f.write(tests_to_run)
    else:
        # convert the given file into just bits, write it into `test.fil.data`
        converted = convert_json(json.load(open(data)), width)
        rewritten_data_filename = os.path.join(dir, 'test.fil.data')
        tests_to_run = converted
        tests_to_check = json.load(open(data))
        with open(rewritten_data_filename, "w") as f:
            f.write(str(tests_to_run))

    func_dict = {}
    func_dict["axpy"] = axpy
    func_dict["dot"] = dot
    func_dict["rot"] = rot
    func_dict["scal"] = scal
    func_dict["syr"] = syr

    expected = get_golden_output(func_dict[called_func], json.load(open(ports)), tests_to_check)
    expected_cmp = convert_expect(convert_json(expected, width))
    expected_conv = str(expected_cmp).replace("\'","\"")
    expected_filename = os.path.join(dir, 'test.expect')
    with open(expected_filename, "w") as f:
        f.write(str(expected_conv))

    run_fud(args.kernel)

    fud_out = os.path.join(dir, 'test.fil.out')
    fud_out = json.load(open(fud_out))

    # print(f"expected_cmp: {expected_cmp}")

    if (fud_out != expected_cmp):
        print(f"expected {expected_cmp} but got {fud_out}\n")


if __name__ == "__main__":
    main()