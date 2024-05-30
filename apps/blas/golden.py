import numpy as np

"""
Computes alpha*x + y
"""
def axpy(inputs):
    alpha, x, y = inputs
    x = np.array(x)
    y = np.array(y)
    return alpha*x + y

def dot(inputs):
    x,y = inputs
    x = np.array(x)
    y = np.array(y)
    return np.dot(x,y)

def rot(inputs):
    (u, v, c, s) = inputs
    assert len(u) == len(v)
    x = np.zeros(len(u))
    y = np.zeros(len(v))
    for i in range(len(u)):
        x[i] = c*u[i] + s*v[i]
        y[i] = (-s)*u[i] + c*v[i]
    return (x,y)

def scal(inputs):
    a, x = inputs
    x = np.array(x)
    return a*x

def syr(inputs):
    [a, x, A ] = inputs
    x = np.array(x)
    a_x = a*x
    mat = a_x @ x.T
    return [(mat + A).tolist()]

def main():
    print(axpy(3,[1,2],[3,4]))

if __name__ == "__main__":
    main()