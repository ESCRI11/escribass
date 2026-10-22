#!/usr/bin/env bash
# convert each graph with the approved converter, compile+render it with cmaj, keep every log
SP=${SPIKE_ROOT:?set SPIKE_ROOT to the scratch directory}
PY=$SP/onnxenv/.venv/bin/python; CM=$SP/tree/cmajor/build/tools/command/cmaj; CONV=$SP/tree/cmajor/tools/onnx/onnxToCmajor.py
M=$SP/m3; mkdir -p $M/patch $M/render
for f in "$@"; do
  n=$(basename $f .onnx); rm -rf $M/patch/$n
  $PY -I $CONV --model $M/models/$n.onnx --patchDir $M/patch/$n > $M/patch/$n.convert.log 2>&1; c=$?
  in=$M/in2.wav; [ "$n" = gru ] && in=$M/in1.wav; [ "$n" = gemm_transA ] && in=$M/in2.wav
  $CM render --input=$in --output=$M/render/$n.wav $M/patch/$n/model.cmajorpatch > $M/render/$n.log 2>&1; r=$?
  echo "$n convert=$c render=$r out=$(stat -c %s $M/render/$n.wav 2>/dev/null || echo none) $(grep -m1 -iE 'error|invalid' $M/patch/$n.convert.log $M/render/$n.log | cut -c1-160)"
done
