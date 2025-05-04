test0 = """
addi x1 x0 1
sw x1 0(x0)
"""

test1 = """
addi x1 x0 2
addi x0 x0 0
addi x0 x0 0
addi x0 x0 0
sw x1 0(x1)
"""

tests0 = [test0]
tests1 = [test1]