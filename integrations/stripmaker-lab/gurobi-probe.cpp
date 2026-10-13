// Local SDK/link/license probe only. Not part of the Rust editor.
#include <gurobi_c++.h>
#include <cmath>
#include <iostream>

int main() {
  int major = 0, minor = 0, technical = 0;
  GRBversion(&major, &minor, &technical);
  std::cout << "sdk_version=" << major << '.' << minor << '.' << technical << '\n';
  try {
    GRBEnv env(true);
    env.set(GRB_IntParam_OutputFlag, 0);
    env.start();
    GRBModel model(env);
    model.set(GRB_IntParam_Threads, 1);
    model.set(GRB_DoubleParam_TimeLimit, 5.0);
    GRBVar x = model.addVar(0.0, 1.0, 0.0, GRB_CONTINUOUS, "x");
    model.setObjective(GRBLinExpr(x), GRB_MAXIMIZE);
    model.optimize();
    const int status = model.get(GRB_IntAttr_Status);
    if (status != GRB_OPTIMAL || std::abs(x.get(GRB_DoubleAttr_X) - 1.0) > 1e-9) {
      std::cerr << "solve_not_optimal status=" << status << '\n';
      return 2;
    }
    std::cout << "native_solve=optimal x=1\n";
    return 0;
  } catch (GRBException &error) {
    // Do not print license paths, account details, or credentials from diagnostics.
    std::cerr << "gurobi_error_code=" << error.getErrorCode() << '\n';
    return 3;
  } catch (...) {
    std::cerr << "unexpected_native_probe_error\n";
    return 4;
  }
}
