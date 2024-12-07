import argparse
from harness import parse_instrs, init, reg_to_binary, generate_json
import tests
from tests import *
import json
import subprocess
import os

def run_instr_seq(instrs, mem, rf):
    for instr in instrs:
        run_instr(instr, mem, rf)
    return mem

def parse_rtype(instr):
    rd = instr[1]
    rs1 = instr[2]
    rs2 = instr[3]
    return [rd, rs1, rs2]

def parse_itype(instr):
    rd = instr[1]
    rs1 = instr[2]
    imm = int(instr[3])
    return [rd, rs1, imm]

def parse_store(instr):
    rs2 = instr[1]
    imm_rs1 = instr[2]
    [imm, rs1] = imm_rs1.split('(')
    rs1 = rs1.split(')')[0]
    return [rs1, rs2, int(imm)]

def parse_load(instr):
    rd = instr[1]
    imm_rs1 = instr[2]
    [imm, rs1] = imm_rs1.split('(')
    rs1 = rs1.split(')')[0]
    return [rd, rs1, int(imm)]

def unsigned(n):
    return n & 0xffffffff

def run_instr(instr, mem, rf):
    instr_split = instr.split(' ')
    op = instr_split[0]
    match op:
        case "add":
            [rd, rs1, rs2] = parse_rtype(instr_split)
            rf[rd] = rf[rs1] + rf[rs2]
        case "sub":
            [rd, rs1, rs2] = parse_rtype(instr_split)
            rf[rd] = rf[rs1] - rf[rs2]
        case "xor":
            [rd, rs1, rs2] = parse_rtype(instr_split)
            rf[rd] = rf[rs1] ^ rf[rs2]
        case "or":
            [rd, rs1, rs2] = parse_rtype(instr_split)
            rf[rd] = rf[rs1] | rf[rs2]
        case "and":
            [rd, rs1, rs2] = parse_rtype(instr_split)
            rf[rd] = rf[rs1] & rf[rs2]
        case "sll":
            [rd, rs1, rs2] = parse_rtype(instr_split)
            rf[rd] = rf[rs1] << rf[rs2]
        case "sra":
            [rd, rs1, rs2] = parse_rtype(instr_split)
            rf[rd] = rf[rs1] >> rf[rs2]
        case "slt":
            [rd, rs1, rs2] = parse_rtype(instr_split)
            rf[rd] = rf[rs1] < rf[rs2]
        case "sltu": # todo
            [rd, rs1, rs2] = parse_rtype(instr_split)
            rf[rd] = rf[rs1] < rf[rs2]
        case "addi":
            [rd, rs1, imm] = parse_itype(instr_split)
            rf[rd] = rf[rs1] + imm
        case "xori":
            [rd, rs1, imm] = parse_itype(instr_split)
            rf[rd] = rf[rs1] ^ imm
        case "ori":
            [rd, rs1, imm] = parse_itype(instr_split)
            rf[rd] = rf[rs1] | imm
        case "andi":
            [rd, rs1, imm] = parse_itype(instr_split)
            rf[rd] = rf[rs1] & imm
        case "slli":
            [rd, rs1, imm] = parse_itype(instr_split)
            rf[rd] = rf[rs1] << imm
        case "srli":
            [rd, rs1, imm] = parse_itype(instr_split)
            rf[rd] = rf[rs1] >> imm
        case "srai":
            [rd, rs1, imm] = parse_itype(instr_split)
            rf[rd] = rf[rs1] >> imm
        case "slti":
            [rd, rs1, imm] = parse_itype(instr_split)
            rf[rd] = rf[rs1] < imm
        case "sltiu":
            [rd, rs1, imm] = parse_itype(instr_split)
            rf[rd] = rf[rs1] < imm
        case "sw":
            [rs1, rs2, imm] = parse_store(instr_split)
            mem[rf[rs1]+imm] = rf[rs2]
        case "lw":
            [rd, rs1, imm] = parse_load(instr_split)
            rd = mem[rf[rs1]+imm]

# Formats mem as an array
def mem_to_array(mem):
    mem_arr = [99999] * 64
    for k in sorted(mem.keys()):
        mem_arr[k] = mem[k]
    return mem_arr

# Formats a result memory into a json
def mem_to_json(instrs, mem):
    output_dict = {}
    output_dict["iram"] = parse_instrs(instrs)

    output_dict["res"] = mem_to_array(mem)
    output_json = json.dumps(output_dict)
    return output_json

def run_fud(insn_size, data_size, dump_vcd):
    subprocess.run([
        "fud",
        "e",
        f"top_{insn_size}_{data_size}.futil",
        "--to",
        "vcd" if dump_vcd else "dat",
        "--through",
        "icarus-verilog",
        "-s",
        "verilog.data",
        "top.json"
        ], stdout=open("top.out",'w'), stderr=open("top.err", 'w')
    )

def generate_input_json(instrs, data_size):
    int_lst = parse_instrs(instrs)
    output_dict= {}
    output_dict["iram"] = {}
    output_dict["iram"]["data"] = int_lst

    # set format
    output_dict["iram"]["format"] = {}
    output_dict["iram"]["format"]["numeric_type"] = "bitnum"
    output_dict["iram"]["format"]["is_signed"] = False
    output_dict["iram"]["format"]["width"] = 32

    output_dict["res"] = {}
    output_dict["res"]["data"] = data_size * [99999]
    output_dict["res"]["format"] = {}
    output_dict["res"]["format"]["numeric_type"] = "bitnum"
    output_dict["res"]["format"]["is_signed"] = False
    output_dict["res"]["format"]["width"] = 32

    output_json = json.dumps(output_dict)
    
    with open("top.json", 'w') as f:
        f.write(output_json)

def generate_top(insn_size, data_size):
    subprocess.run([
        "cp",
        "top.futil",
        f"top_{insn_size}_{data_size}.futil"
    ])
    os.system(f"sed -i '' 's/INSN_SIZE/{insn_size}/g;s/DATA_SIZE/{data_size}/g' top_{insn_size}_{data_size}.futil")

def main():
    parser = argparse.ArgumentParser(description="Parse arguments for generating tests")
    parser.add_argument("--vcd", action="store_true")
    args = parser.parse_args()

    # test is a string
    for test in tests.tests1:
        mem = {}
        rf = {}
        for i in range(0, 32):
            x_str = 'x' + str(i)
            rf[x_str] = 0

        # instrs is a list of strings
        instrs = init(test)
        insn_size = len(instrs)
        data_size = 64
        exp_mem = run_instr_seq(instrs, mem, rf)
        print(f"exp_mem: {exp_mem}")

        # the expected memory output
        exp_mem_json = mem_to_json(instrs, exp_mem)

        generate_input_json(instrs, data_size)

        generate_top(insn_size, data_size)

        # run fud 
        run_fud(insn_size, data_size, args.vcd)

        processed_fud_out = {}
        with open("top.out") as f:
            fud_out = json.load(f)
            processed_fud_out = fud_out["memories"]
            processed_fud_out = str(processed_fud_out).replace("\'", "\"")

        if exp_mem_json != processed_fud_out and not args.vcd:
            print(f"expected {exp_mem_json} but got {processed_fud_out}\n")
        elif exp_mem_json == processed_fud_out:
            print(f"got {exp_mem_json}")

if __name__ == "__main__":
    main()
