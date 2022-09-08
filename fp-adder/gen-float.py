import struct
import random
import numpy as np
import json


# Converts a float to a 32-bit binary string
def float_to_bin(f):
    return ''.join(bin(c).replace('0b', '').rjust(8, '0') for c in struct.pack('!f', f))


def bin_to_float(b):
    """
    Converts a 32-bit binary string to a float
    """
    return struct.unpack('!f', bytes(int(b[i:i+8], 2) for i in range(0, len(b), 8)))[0]


def bin_to_int(b):
    """
    Converts a 32-bit binary string to an int
    """
    return int(b, 2)


if __name__ == '__main__':
    out = {'left': [], 'right': [], 'sum': []}
    for _ in range(1):
        # Use numpy to generate a random float32
        left = np.float32(random.uniform(-100000, 1000000))
        right = np.float32(random.uniform(-100000, 1000000))
        sum = left + right
        out['left'].append(bin_to_int(float_to_bin(left)))
        out['right'].append(bin_to_int(float_to_bin(right)))
        out['sum'].append(bin_to_int(float_to_bin(sum)))

    print(json.dumps(out, indent=2))
