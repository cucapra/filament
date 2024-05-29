from functools import reduce
from itertools import chain

"""
For interop with fud, we represent matrices as integers.
The matrix [[1 2],[3 4]] gets flattened to [1,2,3,4]. Each int gets
converted to binary and concatenated, which is then interpreted as a
single integer. 

matrix: the matrix to convert to an integer
width: desired bitwidth of each integer in the matrix
"""
def matrix_to_int_repr(matrix, width):
    flattened = list(chain.from_iterable(matrix))
    bin_str = reduce(lambda acc, i: acc + bin(i)[2:].zfill(width), flattened, "")
    print(int(bin_str, 2))
    

def main():
    matrix_to_int_repr([[1,2],[3,4]], 4)

if __name__ == "__main__":
    main()