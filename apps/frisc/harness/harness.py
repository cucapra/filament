import json

reg_to_binary = {}

r_op =  0b0110011
i_op =  0b0010011
ld_op = 0b0000011
st_op = 0b0100011
b_op  = 0b1100011

reg_reg = ["add", "sub", "xor", "or", "and", "sll", "srl",
        "sra", "slt", "sltu"]
reg_imm = ["addi", "xori", "ori", "andi", "slli", "srli",
       "srai", "slti", "sltiu"]
branches = ["beq", "bne", "bge", "bgeu", "blt", "bltu"]
loads = ["lw"]
stores = ["sw"]
jumps = ["jalr", "jal"]

# instr -> [opcode, funct3, funct7]

# reg-reg types
rv_table = {}
rv_table["add"]  = [r_op, 0b000, 0b0000000]
rv_table["sub"]  = [r_op, 0b000, 0b0100000]
rv_table["xor"]  = [r_op, 0b100, 0b0000000]
rv_table["or"]   = [r_op, 0b110, 0b0000000]
rv_table["and"]  = [r_op, 0b111, 0b0000000]
rv_table["sll"]  = [r_op, 0b001, 0b0000000]
rv_table["srl"]  = [r_op, 0b101, 0b0000000]
rv_table["sra"]  = [r_op, 0b101, 0b0000010]
rv_table["slt"]  = [r_op, 0b010, 0b0000000]
rv_table["sltu"] = [r_op, 0b011, 0b0000000]

# reg-imm types
rv_table["addi"]  = [i_op, 0b000]
rv_table["xori"]  = [i_op, 0b100]
rv_table["ori"]   = [i_op, 0b110]
rv_table["andi"]  = [i_op, 0b111]
rv_table["slli"]  = [i_op, 0b001]
rv_table["srli"]  = [i_op, 0b101]
rv_table["srai"]  = [i_op, 0b101]
rv_table["slti"]  = [i_op, 0b010]
rv_table["sltiu"] = [i_op, 0b011]

rv_table["lw"] = [ld_op, 0b010]

rv_table["sw"] = [st_op, 0b010]

rv_table["beq"]  = [b_op, 0b010]
rv_table["bne"]  = [b_op, 0b001]
rv_table["blt"]  = [b_op, 0b100]
rv_table["bge"]  = [b_op, 0b101]
rv_table["bltu"] = [b_op, 0b110]
rv_table["bgeu"] = [b_op, 0b111]

rv_table["jalr"] = [0b1100111, 0b000]
rv_table["jal"]  = [0b1101111]

rv_table["lui"]   = [0b0110111]
rv_table["auipc"] = [0b0010111]

def index_bitstring(bits, upper, lower):
    return bits[::-1][lower:upper+1][::-1]


# returns an integer since that is what fud accepts
def parse_instr(instr: str) -> int:
    instr = instr.split()
    op = instr[0]

    if op in reg_reg:
        rd = reg_to_binary[instr[1]]
        rs1 = reg_to_binary[instr[2]]
        rs2 = reg_to_binary[instr[3]]
        [opcode, funct3, funct7] = rv_table[op]
        opcode = str(bin(opcode))[2:].zfill(7)
        funct3 = str(bin(funct3))[2:].zfill(3)
        funct7 = str(bin(funct7))[2:].zfill(7)
        instr_bin_str = funct7 + rs2 + rs1 + funct3 + rd + opcode
        instr_int = int(instr_bin_str,2)
        return instr_int
    if op in reg_imm:
        rd = reg_to_binary[instr[1]]
        rs1 = reg_to_binary[instr[2]]
        imm = bin(int(instr[3]) % 2**12)[2:].zfill(12)
        [opcode, funct3] = rv_table[op]
        opcode = str(bin(opcode))[2:].zfill(7)
        funct3 = str(bin(funct3))[2:].zfill(3)
        instr_bin_str = imm + rs1 + funct3 + rd + opcode
        return int(instr_bin_str, 2)
    if op in stores:
        rs2 = reg_to_binary[instr[1]]
        imm_rs1 = instr[2]
        [imm,rs1] = imm_rs1.split('(')
        rs1 = rs1.split(')')[0]
        imm = bin(int(imm) % 2**12)[2:].zfill(12)
        imm_11_5 = index_bitstring(imm, 11, 5)
        imm_4_0 = index_bitstring(imm, 4, 0)
        rs1 = reg_to_binary[rs1]
        [opcode, funct3] = rv_table[op]
        opcode = str(bin(opcode))[2:].zfill(7)
        funct3 = str(bin(funct3))[2:].zfill(3)
        instr_bin_str = imm_11_5 + rs2 + rs1 + funct3 + imm_4_0 + opcode
        return int(instr_bin_str, 2)

def parse_instrs(instrs):
    ints = []
    for instr in instrs:
        n = parse_instr(instr)
        ints.append(n)
    return ints

# returns a list of instructions in string format
def init(test):
    for i in range(0,32):
        str_reg = "x" + str(i)
        reg_to_binary[str_reg] = bin(i)[2:].zfill(5)

    return list(filter(lambda s: s != '', test.splitlines()))

def generate_json(test):
    instrs = init(test)
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
    output_dict["res"]["data"] = len(int_lst) * [99999]
    output_dict["res"]["format"] = {}
    output_dict["res"]["format"]["numeric_type"] = "bitnum"
    output_dict["res"]["format"]["is_signed"] = False
    output_dict["res"]["format"]["width"] = 32

    output_json = json.dumps(output_dict)
    
    with open("top.json", 'w') as f:
        f.write(output_json)

def main():
    instrs = init()
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
    output_dict["res"]["data"] = len(int_lst) * [99999]
    output_dict["res"]["format"] = {}
    output_dict["res"]["format"]["numeric_type"] = "bitnum"
    output_dict["res"]["format"]["is_signed"] = False
    output_dict["res"]["format"]["width"] = 32

    output_json = json.dumps(output_dict)
    
    with open("top.json", 'w') as f:
        f.write(output_json)

if __name__ == "__main__":
    main()