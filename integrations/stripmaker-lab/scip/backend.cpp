#include "gurobi_c++.h"
#include <scip/scip.h>
#include <scip/scipdefplugins.h>
#include <scip/cons_linear.h>
#include <scip/cons_nonlinear.h>
#include <algorithm>
#include <iostream>
#include <limits>
#include <cstdlib>
#include <Eigen/Eigenvalues>
#include <osqp.h>
static_assert(sizeof(OSQPFloat)==8 && sizeof(OSQPInt)==4, "Research OSQP ABI must be double/int32");

namespace {
void call(SCIP_RETCODE result) {
  if (result != SCIP_OKAY) throw GRBException("SCIP API error " + std::to_string(result));
}
void compatible(const ScipExpr& a, const ScipExpr& b) {
  if (a.owner && b.owner && a.owner != b.owner) throw GRBException("Mixed solver models");
}
void finite(double value) { if (!std::isfinite(value)) throw GRBException("Nonfinite coefficient"); }
struct Session {
  SCIP* scip = nullptr;
  std::vector<SCIP_VAR*> variables;
  std::vector<SCIP_CONS*> constraints;
  Session() { call(SCIPcreate(&scip)); }
  ~Session() {
    for (auto& cons : constraints) SCIPreleaseCons(scip, &cons);
    for (auto& var : variables) SCIPreleaseVar(scip, &var);
    SCIPfree(&scip);
  }
};
void clean(ScipExpr& e) {
  for (auto i = e.linear.begin(); i != e.linear.end();) {
    finite(i->second); if (i->second == 0) i = e.linear.erase(i); else ++i;
  }
  for (auto i = e.quadratic.begin(); i != e.quadratic.end();) {
    finite(i->second); if (i->second == 0) i = e.quadratic.erase(i); else ++i;
  }
  finite(e.constant);
}
void constraint(Session& s, const ScipExpr& e, double lhs, double rhs) {
  std::vector<SCIP_VAR*> lv, qa, qb;
  std::vector<double> lc, qc;
  for (auto [id, coef] : e.linear) { lv.push_back(s.variables.at(id)); lc.push_back(coef); }
  for (auto [ids, coef] : e.quadratic) {
    qa.push_back(s.variables.at(ids.first)); qb.push_back(s.variables.at(ids.second)); qc.push_back(coef);
  }
  const double inf = SCIPinfinity(s.scip);
  lhs = lhs <= -GRB_INFINITY ? -inf : lhs - e.constant;
  rhs = rhs >= GRB_INFINITY ? inf : rhs - e.constant;
  SCIP_CONS* c = nullptr;
  const auto name = "constraint_" + std::to_string(s.constraints.size());
  if (qc.empty()) call(SCIPcreateConsBasicLinear(s.scip, &c, name.c_str(),
    static_cast<int>(lv.size()), lv.data(), lc.data(), lhs, rhs));
  else call(SCIPcreateConsBasicQuadraticNonlinear(s.scip, &c, name.c_str(),
    static_cast<int>(lv.size()), lv.data(), lc.data(), static_cast<int>(qc.size()),
    qa.data(), qb.data(), qc.data(), lhs, rhs));
  s.constraints.push_back(c);
  call(SCIPaddCons(s.scip, c));
}
double evaluate(const ScipExpr& e, const std::vector<double>& values) {
  double v = e.constant;
  for (auto [id, coef] : e.linear) v += coef * values.at(id);
  for (auto [ids, coef] : e.quadratic) v += coef * values.at(ids.first) * values.at(ids.second);
  finite(v); return v;
}
bool convex(const ScipExpr& objective) {
  std::map<size_t, int> ids;
  for (auto [pair, coef] : objective.quadratic) { ids.emplace(pair.first, 0); ids.emplace(pair.second, 0); }
  if (ids.empty() || ids.size() > 512) return false; // bounded research certificate
  int index = 0; for (auto& item : ids) item.second = index++;
  Eigen::MatrixXd q = Eigen::MatrixXd::Zero(index, index);
  for (auto [pair, coef] : objective.quadratic) {
    int a = ids.at(pair.first), b = ids.at(pair.second);
    if (a == b) q(a, b) += coef;
    else { q(a, b) += coef / 2; q(b, a) += coef / 2; }
  }
  Eigen::SelfAdjointEigenSolver<Eigen::MatrixXd> check(q, Eigen::EigenvaluesOnly);
  return check.info() == Eigen::Success && check.eigenvalues().minCoeff() >=
    -1e-12 * std::max(1.0, q.cwiseAbs().maxCoeff());
}
struct Csc {
  std::vector<OSQPInt> rows, columns;
  std::vector<OSQPFloat> data;
  OSQPCscMatrix matrix{};
  Csc(OSQPInt m, OSQPInt n, const std::map<std::pair<size_t,size_t>,double>& entries) {
    columns.push_back(0);
    auto entry = entries.begin(); // key order is column, row
    for (OSQPInt col=0; col<n; ++col) {
      while(entry != entries.end() && entry->first.first == static_cast<size_t>(col)) {
        finite(entry->second);
        if(entry->second != 0) { rows.push_back(static_cast<OSQPInt>(entry->first.second)); data.push_back(entry->second); }
        ++entry;
      }
      columns.push_back(static_cast<OSQPInt>(data.size()));
    }
    if(entry != entries.end()) throw GRBException("QP matrix index out of bounds");
    matrix={m,n,columns.data(),rows.data(),data.data(),static_cast<OSQPInt>(data.size()),-1,0};
  }
};
struct QpSession {
  OSQPSolver* solver=nullptr;
  ~QpSession() { if(solver) osqp_cleanup(solver); }
};
std::vector<double> solve_qp(const ScipExpr& objective, const std::vector<ScipConstraint>& constraints,
  const std::vector<std::pair<double,double>>& bounds, double seconds, bool debug, int& status) {
  if(seconds <= 0) { status=GRB_TIME_LIMIT; throw GRBException("QP deadline expired before solve"); }
  const auto n=static_cast<OSQPInt>(bounds.size()), m=static_cast<OSQPInt>(constraints.size()+bounds.size());
  std::map<std::pair<size_t,size_t>,double> p,a;
  std::vector<OSQPFloat> q(n,0),lower(m),upper(m);
  for(auto [id,c] : objective.linear) q.at(id)=c;
  // OSQP minimizes 0.5*x'P*x + q'x; only the upper triangle is supplied.
  for(auto [ids,c] : objective.quadratic) p[{ids.second,ids.first}]+=ids.first==ids.second ? 2*c : c;
  size_t row=0;
  for(const auto& c : constraints) {
    for(auto [id,coef] : c.expression.linear) a[{id,row}]+=coef;
    lower[row]=c.relation=='<' ? -OSQP_INFTY : -c.expression.constant;
    upper[row]=c.relation=='>' ? OSQP_INFTY : -c.expression.constant;
    ++row;
  }
  for(size_t id=0; id<bounds.size(); ++id,++row) {
    a[{id,row}]=1;
    lower[row]=bounds[id].first<=-GRB_INFINITY ? -OSQP_INFTY : bounds[id].first;
    upper[row]=bounds[id].second>=GRB_INFINITY ? OSQP_INFTY : bounds[id].second;
  }
  Csc P(n,n,p), A(m,n,a);
  OSQPSettings settings; osqp_set_default_settings(&settings);
  settings.verbose=debug; settings.polishing=1; settings.warm_starting=0;
  settings.eps_abs=1e-7; settings.eps_rel=1e-7; settings.max_iter=20000; settings.time_limit=seconds;
  QpSession session;
  auto error=osqp_setup(&session.solver,&P.matrix,q.data(),&A.matrix,lower.data(),upper.data(),m,n,&settings);
  if(error) {status=GRB_NUMERIC; throw GRBException("OSQP setup failed: "+std::to_string(error));}
  error=osqp_solve(session.solver);
  auto code=session.solver->info->status_val;
  status=code==OSQP_SOLVED ? GRB_OPTIMAL : code==OSQP_PRIMAL_INFEASIBLE ? GRB_INFEASIBLE :
    code==OSQP_DUAL_INFEASIBLE ? GRB_UNBOUNDED : GRB_TIME_LIMIT;
  if(error || code!=OSQP_SOLVED || !session.solver->solution || !session.solver->solution->x)
    throw GRBException("OSQP rejected result: "+std::string(session.solver->info->status)+", API "+std::to_string(error));
  if(debug) std::cout<<"OSQP validated solve, variables "<<n<<", iterations "<<session.solver->info->iter
    <<", primal residual "<<session.solver->info->prim_res<<", dual residual "<<session.solver->info->dual_res<<'\n';
  return {session.solver->solution->x,session.solver->solution->x+n};
}
}
double GRBVar::get(GRB_DoubleAttr attr) const {
  if (attr != GRB_DoubleAttr_X || !owner || !owner->solved)
    throw GRBException("No validated solver solution");
  return owner->x.at(index);
}
ScipExpr& ScipExpr::operator+=(const ScipExpr& other) {
  compatible(*this, other); if (!owner) owner = other.owner;
  constant += other.constant;
  for (auto [id, coef] : other.linear) linear[id] += coef;
  for (auto [ids, coef] : other.quadratic) quadratic[ids] += coef;
  clean(*this); return *this;
}
ScipExpr& ScipExpr::operator-=(const ScipExpr& other) { return *this += -other; }
ScipExpr operator+(ScipExpr a, const ScipExpr& b) { return a += b; }
ScipExpr operator-(ScipExpr a, const ScipExpr& b) { return a -= b; }
ScipExpr operator-(const ScipExpr& a) { return ScipExpr(-1) * a; }
ScipExpr operator*(const ScipExpr& a, const ScipExpr& b) {
  compatible(a, b);
  if ((!a.quadratic.empty() && (!b.linear.empty() || !b.quadratic.empty())) ||
      (!b.quadratic.empty() && !a.linear.empty())) throw GRBException("Degree exceeds quadratic");
  ScipExpr e(a.constant * b.constant); e.owner = a.owner ? a.owner : b.owner;
  for (auto [id, c] : a.linear) e.linear[id] += c * b.constant;
  for (auto [id, c] : b.linear) e.linear[id] += c * a.constant;
  for (auto [ids, c] : a.quadratic) e.quadratic[ids] += c * b.constant;
  for (auto [ids, c] : b.quadratic) e.quadratic[ids] += c * a.constant;
  for (auto [i, c] : a.linear) for (auto [j, d] : b.linear)
    e.quadratic[{std::min(i, j), std::max(i, j)}] += c * d;
  clean(e); return e;
}
void ScipExpr::addTerm(double c, GRBVar a, GRBVar b) { *this += ScipExpr(c) * a * b; }
ScipExpr operator/(const ScipExpr& a, double divisor) {
  finite(divisor); if (divisor == 0) throw GRBException("Zero expression divisor");
  return a * ScipExpr(1 / divisor);
}
void ScipExpr::addTerms(const double* c, const GRBVar* v, size_t n) {
  for (size_t i = 0; i < n; ++i) *this += ScipExpr(c[i]) * v[i];
}
void ScipExpr::addTerms(const double* c, const GRBVar* a, const GRBVar* b, size_t n) {
  for (size_t i = 0; i < n; ++i) addTerm(c[i], a[i], b[i]);
}
ScipConstraint operator<=(const ScipExpr& a, const ScipExpr& b) { return {a - b, '<'}; }
ScipConstraint operator>=(const ScipExpr& a, const ScipExpr& b) { return {a - b, '>'}; }
ScipConstraint operator==(const ScipExpr& a, const ScipExpr& b) { return {a - b, '='}; }
void GRBEnv::set(GRB_IntParam p, int v) {
  if (p != GRB_IntParam_LogToConsole) throw GRBException("Unsupported environment parameter");
  log = v != 0;
}
void GRBModel::check(const ScipExpr& e) const {
  if (e.owner && e.owner != values) throw GRBException("Expression belongs to another model");
}
GRBVar GRBModel::addVar(double l, double u, double obj, char type, const std::string&) {
  finite(l); finite(u); finite(obj);
  if (l > u || (type != GRB_CONTINUOUS && type != GRB_BINARY) || variables.size() >= 20000)
    throw GRBException("Invalid variable or research size limit");
  values->solved = false;
  variables.push_back({l, u, obj, type}); values->x.push_back(0);
  return {values, variables.size() - 1};
}
void GRBModel::addConstr(const ScipConstraint& c) {
  check(c.expression);
  if (constraints.size() >= 100000) throw GRBException("Research constraint limit");
  constraints.push_back(c); values->solved = false;
}
void GRBModel::addGenConstrAbs(GRBVar result, GRBVar argument) {
  // Exact absolute value: y >= x, y >= -x, y^2 == x^2. Not an L1-only relaxation.
  addConstr(result >= argument); addConstr(result >= -ScipExpr(argument));
  addConstr(ScipExpr(result) * result == ScipExpr(argument) * argument);
}
void GRBModel::setObjective(const ScipExpr& e, int sense) {
  check(e); if (sense != GRB_MINIMIZE) throw GRBException("Only minimization is implemented");
  objective = e; explicit_objective = true; values->solved = false;
}
void GRBModel::set(GRB_IntParam p, int v) {
  if (p == GRB_IntParam_LogToConsole) log = v != 0;
  else if (p != GRB_IntParam_Threads || v != 1) throw GRBException("Research backend supports one solver thread");
}
void GRBModel::set(GRB_DoubleParam p, double v) {
  finite(v);
  if (v < 0) throw GRBException("Negative solver limit");
  if (p == GRB_DoubleParam_TimeLimit) time_limit = std::min(v, 30.0);
  else if (p == GRB_DoubleParam_NodeLimit) node_limit = std::min(v, 100000.0);
  else throw GRBException("Unsupported solver parameter");
}
double GRBModel::get(GRB_DoubleAttr attr) const {
  if (attr != GRB_DoubleAttr_ObjVal || !values->solved) throw GRBException("No objective solution");
  return evaluate(objective, values->x);
}
void GRBModel::run(const char* write_path) {
  values->solved = false; status = 0;
  if (!explicit_objective) {
    objective = 0;
    for (size_t i = 0; i < variables.size(); ++i) objective += variables[i].objective * ScipExpr(GRBVar{values, i});
  }
  const bool debug = std::getenv("RAINEE_SCIP_DEBUG") != nullptr;
  bool qp = !write_path && convex(objective)
    && std::all_of(variables.begin(),variables.end(),[](auto v){return v.type==GRB_CONTINUOUS;})
    && std::all_of(constraints.begin(),constraints.end(),[](const auto& c){return c.expression.quadratic.empty();});
  if(qp) {
    std::vector<std::pair<double,double>> bounds;
    for(auto v:variables) bounds.emplace_back(v.lower,v.upper);
    values->x=solve_qp(objective,constraints,bounds,time_limit,log||debug,status);
  } else {
  Session s;
  call(SCIPincludeDefaultPlugins(s.scip));
  call(SCIPsetIntParam(s.scip, "display/verblevel", log || debug ? 4 : 0));
  call(SCIPsetRealParam(s.scip, "limits/time", time_limit));
  call(SCIPsetLongintParam(s.scip, "limits/nodes", static_cast<SCIP_Longint>(node_limit)));
  call(SCIPsetRealParam(s.scip, "limits/memory", 1024.0));
  call(SCIPcreateProbBasic(s.scip, "rainee_stripmaker_research"));
  const double inf = SCIPinfinity(s.scip);
  auto add = [&](double lower, double upper, double obj, SCIP_VARTYPE type) {
    SCIP_VAR* var = nullptr;
    const auto name = "v" + std::to_string(s.variables.size());
    call(SCIPcreateVarBasic(s.scip, &var, name.c_str(), lower, upper, obj, type));
    s.variables.push_back(var); call(SCIPaddVar(s.scip, var));
  };
  for (size_t i = 0; i < variables.size(); ++i) {
    auto v = variables[i];
    const auto found = objective.linear.find(i);
    double obj = objective.quadratic.empty() && found != objective.linear.end() ? found->second : 0;
    add(v.lower <= -GRB_INFINITY ? -inf : v.lower, v.upper >= GRB_INFINITY ? inf : v.upper,
      obj, v.type == GRB_BINARY ? SCIP_VARTYPE_BINARY : SCIP_VARTYPE_CONTINUOUS);
  }
  for (const auto& c : constraints) constraint(s, c.expression,
    c.relation == '<' ? -GRB_INFINITY : 0, c.relation == '>' ? GRB_INFINITY : 0);
  if (!objective.quadratic.empty()) {
    // SCIP has a linear objective: minimize t subject to f(x) - t <= 0.
    add(-inf, inf, 1, SCIP_VARTYPE_CONTINUOUS);
    ScipExpr epigraph = objective; epigraph.linear[variables.size()] = -1;
    constraint(s, epigraph, -GRB_INFINITY, 0);
  }
  if (debug) std::cout << "SCIP model variables " << variables.size() << " quadratic terms " << objective.quadratic.size() << '\n';
  if (write_path) { call(SCIPwriteOrigProblem(s.scip, write_path, nullptr, FALSE)); return; }
  call(SCIPsolve(s.scip));
  const auto result = SCIPgetStatus(s.scip);
  status = result == SCIP_STATUS_OPTIMAL ? GRB_OPTIMAL :
    result == SCIP_STATUS_INFEASIBLE ? GRB_INFEASIBLE :
    result == SCIP_STATUS_UNBOUNDED ? GRB_UNBOUNDED :
    result == SCIP_STATUS_INFORUNBD ? GRB_INF_OR_UNBD : GRB_TIME_LIMIT;
  if (status != GRB_OPTIMAL) throw GRBException("SCIP did not reach optimality; status " + std::to_string(result)
    + ", variables " + std::to_string(variables.size()) + ", primal " + std::to_string(SCIPgetPrimalbound(s.scip))
    + ", dual " + std::to_string(SCIPgetDualbound(s.scip)));
  SCIP_SOL* solution = SCIPgetBestSol(s.scip);
  if (!solution) throw GRBException("SCIP returned no primal solution");
  for (size_t i = 0; i < variables.size(); ++i) {
    double v = SCIPgetSolVal(s.scip, solution, s.variables[i]); finite(v);
    if (variables[i].type == GRB_BINARY) {
      if (std::abs(v - std::round(v)) > 1e-6) throw GRBException("Invalid integer solution");
      v = std::round(v); // author orientation tests exact equality with zero
    }
    values->x[i] = v;
  }
  }
  // Shared post-solve checks against the original model, independent of solver.
  for(size_t i=0;i<variables.size();++i) {
    double v=values->x.at(i); finite(v);
    if (v < variables[i].lower - 1e-5 || v > variables[i].upper + 1e-5)
      throw GRBException("Solution violates variable bounds");
  }
  for (const auto& c : constraints) {
    double v = evaluate(c.expression, values->x);
    double scale = 1 + std::abs(c.expression.constant);
    for (auto [id, coef] : c.expression.linear) scale += std::abs(coef * values->x.at(id));
    for (auto [ids, coef] : c.expression.quadratic) scale += std::abs(coef * values->x.at(ids.first) * values->x.at(ids.second));
    double tolerance = 1e-5 * scale;
    if ((c.relation != '>' && v > tolerance) || (c.relation != '<' && v < -tolerance))
      throw GRBException("Solution violates original constraint");
  }
  values->solved = true;
}
