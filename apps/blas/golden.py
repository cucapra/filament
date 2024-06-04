import numpy as np

"""
Computes alpha*x + y
"""
def axpy(inputs):
    alpha, x, y = inputs
    x = np.array(x)
    y = np.array(y)
    return [(alpha*x + y).tolist()]

def dot(inputs):
    x,y = inputs
    x = np.array(x).flatten()
    y = np.array(y).flatten()
    r = np.dot(x,y)
    return [r]

def rot(inputs):
    [c,s,x,y] = inputs
    x = np.array(x)
    y = np.array(y)
    out_1 = c*x + s*y
    out_2 = -s*x + c*y
    return [out_1.tolist(),out_2.tolist()]

def scal(inputs):
    a, x = inputs
    x = np.array(x)
    return [(a*x).tolist()]

def syr(inputs):
    [a, x, A ] = inputs
    x = np.array(x)
    a_x = a*x
    mat = a_x @ x.T
    return [(mat + A).tolist()]