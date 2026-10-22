"""m5.1-spike: a preflight that refuses, by node and operator, everything outside what this spike
measured correct — before the converter runs. A prototype of agent default #17, not its code."""
import sys
import onnx
from onnx import TensorProto as T

MEASURED = {"MatMul", "Add", "Tanh", "Gemm"}  # correct to float32 rounding, through cmaj and the engine

def refuse(model):
    g = model.graph
    for o in model.opset_import:
        if o.domain not in ("", "ai.onnx"):
            yield f"opset domain '{o.domain}' is not the default domain"
    for v in list(g.input) + list(g.output):
        t = v.type.tensor_type
        if t.elem_type != T.FLOAT:
            yield f"graph {('input' if v in g.input else 'output')} '{v.name}' is not float32"
        if any(not d.HasField("dim_value") for d in t.shape.dim):
            yield f"'{v.name}' has a dynamic dimension; v1 converts static shapes only"
    for init in g.initializer:
        if init.data_location == T.EXTERNAL:
            yield f"initialiser '{init.name}' keeps its weights in an external file"
    shapes = {i.name: list(onnx.numpy_helper.to_array(i).shape) for i in g.initializer}
    for n in g.node:
        where = f"node '{n.name or n.output[0]}' ({n.op_type})"
        if n.domain not in ("", "ai.onnx"):
            yield f"{where} is in domain '{n.domain}'"
        if any(a.type in (onnx.AttributeProto.GRAPH, onnx.AttributeProto.GRAPHS) for a in n.attribute):
            yield f"{where} carries a subgraph"
        if n.op_type not in MEASURED:
            yield f"{where}: operator not supported in v1"
            continue
        if n.op_type == "Gemm":
            a = {x.name: onnx.helper.get_attribute_value(x) for x in n.attribute}
            for k, ok in (("alpha", 1.0), ("beta", 1.0), ("transA", 0), ("transB", 0)):
                if a.get(k, ok) != ok:
                    yield f"{where}: {k}={a[k]} is ignored by the converter, so it is refused rather than computed wrongly"
            if len(n.input) > 2 and len(shapes.get(n.input[2], [1, 1])) == 0:
                yield f"{where}: a scalar bias C makes the converter emit invalid Cmajor"

for path in sys.argv[1:]:
    why = list(refuse(onnx.load(path)))
    print(f"{path.rsplit('/', 1)[-1]:20s} {'REFUSED: ' + '; '.join(why) if why else 'accepted'}")
