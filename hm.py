import json
from hypermapper import optimizer
import math

scenario = {
  "application_name": "test",
  "optimization_objectives": ["Value", "Energy"],
  "optimization_iterations": 10,
  "evaluations_per_optimization_iteration": 10,
  "input_parameters": {
    "x1": {
      "parameter_type": "real",
      "values": [0, 1]
    },
    "x2": {
      "parameter_type": "real",
      "values": [0, 1]
    }
  },
}

with open('out.json', 'w') as f:
  json.dump(scenario, f)

def Branin(X):
    x1 = X['x1']
    x2 = X['x2']
    a = 1.0
    b = 5.1 / (4.0 * math.pi * math.pi)
    c = 5.0 / math.pi
    r = 6.0
    s = 10.0
    t = 1.0 / (8.0 * math.pi)
    y_value = a * (x2 - b * x1 * x1 + c * x1 - r) ** 2 + s * (1 - t) * math.cos(x1) + s
    y_energy = x1 + x2

    optimization_metrics = {}
    optimization_metrics["Value"] = y_value
    optimization_metrics["Energy"] = y_energy

    return optimization_metrics

def batch_test(X):
  results = {"Value": [], "Energy": []}
  for x1, x2 in zip(X['x1'], X['x2']):
    in_ = {"x1": x1, "x2": x2}
    result = Branin(in_)
    results["Value"].append(result["Value"])
    results["Energy"].append(result["Energy"])
  return results


optimizer.optimize('out.json', batch_test)