# for determining the right answer

def rot(u, v, c, s):
  assert len(u) == len(v)
  x = [0] * len(u)
  y = [0] * len(v)
  for i in range(len(u)):
    x[i] = c*u[i] + s*v[i]
    y[i] = (-s)*u[i] + c*v[i]
  return (x,y)

if __name__ == '__main__':
  u0 = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15]
  v0 = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15]
  c0 = 1
  s0 = 1
  (x0, y0) = rot(u0, v0, c0, s0)
  print(f"x0: {x0}")
  print(f"y0: {y0}")

  print("\n======================\n")

  u1 = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]
  v1 = [16,15,14,13,12,11,10,9,8,7,6,5,4,3,2,1]
  c1 = 1
  s1 = 1
  (x1, y1) = rot(u1, v1, c1, s1)
  print(f"x1: {x1}")
  print(f"y1: {y1}")