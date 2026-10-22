"""m5.1-spike measurement 3: synthetic ONNX graphs with known weights, and their references."""
import json, sys
import numpy as np
import onnx
from onnx import helper as h, TensorProto as T, numpy_helper as nh

OUT = sys.argv[1]
F = T.FLOAT
W1 = np.array([[0.9, -0.3, 0.5, 0.2], [0.1, 0.7, -0.4, 0.6]], np.float32)
B1 = np.array([0.05, -0.1, 0.0, 0.2], np.float32)
W2 = np.array([[0.8, 0.1], [-0.2, 0.9], [0.4, -0.3], [0.3, 0.5]], np.float32)
B2 = np.array([0.01, -0.02], np.float32)
G = np.array([[0.6, -0.2], [0.3, 0.8]], np.float32)

def model(name, nodes, inputs, outputs, inits, opset=17):
    g = h.make_graph(nodes, name, inputs, outputs, [nh.from_array(a, n) for n, a in inits.items()])
    m = h.make_model(g, opset_imports=[h.make_opsetid("", opset)], producer_name="m5.1-spike")
    m.ir_version = 9
    onnx.checker.check_model(m)
    onnx.save(m, f"{OUT}/{name}.onnx")

vi = lambda n, shape: h.make_tensor_value_info(n, F, shape)

def ff(name, w2):
    model(name, [h.make_node("MatMul", ["x", "W1"], ["h0"]), h.make_node("Add", ["h0", "B1"], ["h1"]),
                 h.make_node("Tanh", ["h1"], ["h2"]), h.make_node("MatMul", ["h2", "W2"], ["h3"]),
                 h.make_node("Add", ["h3", "B2"], ["y"])],
          [vi("x", [1, 2])], [vi("y", [1, 2])], {"W1": W1, "B1": B1, "W2": w2, "B2": B2})
ff("ff", W2)
ff("ff_w", W2 * 0.5)

def gemm(name, c=None, x_shape=(1, 2), **attrs):
    ins = ["x", "G"] + (["C"] if c is not None else [])
    inits = {"G": G} | ({"C": c} if c is not None else {})
    model(name, [h.make_node("Gemm", ins, ["y"], **attrs)], [vi("x", list(x_shape))], [vi("y", [1, 2])], inits)
gemm("gemm_full", np.array([[0.1, -0.2]], np.float32))
gemm("gemm_vec", np.array([0.1, -0.2], np.float32))
gemm("gemm_scalar", np.array(0.1, np.float32))
gemm("gemm_noc")
gemm("gemm_alphabeta", np.array([[0.1, -0.2]], np.float32), alpha=0.5, beta=2.0)
gemm("gemm_transB", np.array([[0.1, -0.2]], np.float32), transB=1)
gemm("gemm_transA", np.array([[0.1, -0.2]], np.float32), x_shape=(2, 1), transA=1)
model("softmax", [h.make_node("Softmax", ["x"], ["y"], axis=1)], [vi("x", [1, 2])], [vi("y", [1, 2])], {})

# A small recurrent case: one input feature, hidden size 2, one step per frame.
H = 2
rng = np.random.default_rng(7)  # seeded: the weights are a fixture, not a draw at run time
W = rng.uniform(-0.5, 0.5, (1, 3 * H, 1)).astype(np.float32)
R = rng.uniform(-0.5, 0.5, (1, 3 * H, H)).astype(np.float32)
Bg = rng.uniform(-0.1, 0.1, (1, 6 * H)).astype(np.float32)
WO = np.array([[0.7], [-0.4]], np.float32)
model("gru", [h.make_node("GRU", ["x", "W", "R", "B"], ["Y"], hidden_size=H),
              h.make_node("Squeeze", ["Y", "axes"], ["Ys"]),
              h.make_node("MatMul", ["Ys", "WO"], ["y"])],
      [vi("x", [1, 1, 1])], [vi("y", [1, 1])],
      {"W": W, "R": R, "B": Bg, "WO": WO, "axes": np.array([0, 1], np.int64)})
json.dump({"W1": W1.tolist(), "B1": B1.tolist(), "W2": W2.tolist(), "B2": B2.tolist(), "G": G.tolist()},
          open(f"{OUT}/weights.json", "w"))
print("ok")
