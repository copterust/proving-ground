from sympy import *

# Without an input our state changed only by biases
# So we repeat our quat
I4 = Identity(4)
# and we assume biases stay the same
I3 = Identity(3)
# we drop the first column to multiply quaternions with 3-vectors
def q2m(q):
    return Matrix([
        [-q.b, -q.c, -q.d],
        [ q.a, -q.d,  q.c],
        [ q.d,  q.a, -q.b],
        [-q.c,  q.b,  q.a]])

# no interaction between q and b
Z3x4 = zeros(3, 4)
# we assume constant angular speed between measurements
dt = symbols("dT")
# Estimated quaternion
q0, q1, q2, q3 = symbols("q0 q1 q2 q3")
q = Quaternion(q0, q1, q2, q3)
# Estimated bias
bx, by, bz = symbols("bx by bz")
b = Matrix([bx, by, bz])
# Our state
x = Matrix([q0, q1, q2, q3, bx, by, bz])
# State transition matrix
A = BlockMatrix([[I4, (-dt / 2.0) * q2m(q)], [Z3x4, I3]])
# Measured angular velocity control our attitude
wx, wy, wz = symbols("wx wy wz")
w = Matrix([wx, wy, wz])
# But doesn't influence the bias
Z3x3 = zeros(3, 3)
# So our control
B = BlockMatrix([[q2m(q)], [Z3x3]])
# Next state
nx = Matrix(A) * x + (dt / 2.0) * Matrix(B) * w

def reduce(exp):
    # TODO: why so complicated?
    return simplify(collect(collect(exp, [q0, q1, q2, q3]), dt))

assigns = ["q0n", "q1n", "q2n", "q3n", "bxn", "byn", "bzn"]
for i, r in enumerate(nx):
    print(rust_code(reduce(r), assign_to=assigns[i]))

