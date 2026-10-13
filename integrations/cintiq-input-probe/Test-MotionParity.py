"""Research oracle only. Compare shared Rust evaluator with official TensorFlow Lite."""
import json, sys
import numpy as np
import tensorflow as tf
report = json.load(open(sys.argv[1], encoding="utf-8-sig"))
interpreter = tf.lite.Interpreter(model_path=sys.argv[2], num_threads=1)
interpreter.allocate_tensors()
run = interpreter.get_signature_runner("serving_default")
worst = 0.0
for case in report["cases"]:
    out = run(**{name: np.asarray([values], dtype=np.float32) for name, values in zip(["r","phi","pressure","tilt","orientation"],case["features"])})
    expected = np.asarray(case["output"], dtype=np.float32)
    actual = np.concatenate([out[name].reshape(1,-1) for name in ["r","phi","pressure"]])
    error = float(np.max(np.abs(expected-actual)))
    worst = max(worst,error)
    if not np.allclose(expected,actual,atol=0.00005,rtol=0.00005):
        raise RuntimeError(f"TFLite parity failed: max_abs={error}")
print(json.dumps({"cases":len(report["cases"]),"max_abs":worst,"atol":0.00005,"rtol":0.00005,"tensorflow":tf.__version__,"passed":True}))
