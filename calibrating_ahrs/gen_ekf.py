from sympy import *
# Our state is [q, b], where q is quatenion and b are biases
state_len = 7
x = IndexedBase('x', shape=(state_len,))
i = Idx('i', state_len)
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
q = Quaternion(x[0], x[1], x[2], x[3])
# Estimated bias
b = Matrix([x[4], x[5], x[6]])
# State transition matrix
A = BlockMatrix([[I4, (-dt / 2.0) * q2m(q)], [Z3x4, I3]])
# Measured angular velocity control our attitude
w = IndexedBase('w', shape=(3,))
j = Idx('j', 3)
w_m = Matrix([w[0], w[1], w[2]])
# But doesn't influence the bias
Z3x3 = zeros(3, 3)
# So our control
B = BlockMatrix([[q2m(q)], [Z3x3]])
# State in matrix form
x_m = Matrix([x[i] for i in range(state_len)])
# Next state
nx = Matrix(A) * x_m + (dt / 2.0) * Matrix(B) * w_m

def reduce(exp):
    # TODO: why so complicated?
    return simplify(collect(collect(exp, [q0, q1, q2, q3]), dt))

# Output our state transition equation
output = MatrixSymbol('nx', state_len, 1)
print(rust_code(nx, assign_to=output, contract=False))
