// Research-only replacement for the author's Gurobi tuning/retry policy.
#include "Context.h"
#include <iostream>
size_t max_allowed_parallelism = 1;
int model_number = 0;
Context::Context(bool log) : grb(true), MAXTHREADS(tbb::global_control::max_allowed_parallelism, 1) {
  grb.set(GRB_IntParam_LogToConsole, log ? 1 : 0); grb.start();
}
void Context::optimize_model(GRBModel* model) const {
  model->set(GRB_IntParam_Threads, 1);
  // Do not pretend Gurobi-specific barrier, PSD tolerance or scaling parameters
  // have SCIP equivalents. A failed/limited result is never returned as a fit.
  model->optimize();
}
