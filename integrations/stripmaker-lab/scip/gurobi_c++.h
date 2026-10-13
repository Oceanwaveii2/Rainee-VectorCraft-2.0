// Original MIT OR Apache-2.0 adapter, not Gurobi SDK code.
// Only the mathematical expression subset used by pinned StripMaker is implemented.
#pragma once
#include <cmath>
#include <map>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

constexpr double GRB_INFINITY = 1e100;
constexpr char GRB_CONTINUOUS = 'C', GRB_BINARY = 'B';
constexpr int GRB_MINIMIZE = 1, GRB_OPTIMAL = 2, GRB_INFEASIBLE = 3,
  GRB_INF_OR_UNBD = 4, GRB_UNBOUNDED = 5, GRB_TIME_LIMIT = 9, GRB_NUMERIC = 12;
enum GRB_DoubleAttr { GRB_DoubleAttr_X, GRB_DoubleAttr_ObjVal };
enum GRB_IntAttr { GRB_IntAttr_Status };
enum GRB_IntParam { GRB_IntParam_LogToConsole, GRB_IntParam_Threads };
enum GRB_DoubleParam { GRB_DoubleParam_TimeLimit, GRB_DoubleParam_NodeLimit };
class GRBException : public std::runtime_error {
public:
  explicit GRBException(const std::string& message) : std::runtime_error(message) {}
  int getErrorCode() const { return -1; }
  std::string getMessage() const { return what(); }
};
struct ScipValues { std::vector<double> x; bool solved = false; };
struct GRBVar {
  std::shared_ptr<ScipValues> owner;
  size_t index = 0;
  double get(GRB_DoubleAttr attr) const;
};
struct ScipExpr {
  std::shared_ptr<ScipValues> owner;
  double constant = 0;
  std::map<size_t, double> linear;
  std::map<std::pair<size_t, size_t>, double> quadratic;
  ScipExpr(double value = 0) : constant(value) {
    if (!std::isfinite(value)) throw GRBException("Nonfinite expression");
  }
  ScipExpr(GRBVar variable) : owner(variable.owner) { linear[variable.index] = 1; }
  ScipExpr& operator+=(const ScipExpr& other);
  ScipExpr& operator-=(const ScipExpr& other);
  void addTerm(double coefficient, GRBVar a, GRBVar b);
  void addTerms(const double* coefficients, const GRBVar* vars, size_t count);
  void addTerms(const double* coefficients, const GRBVar* a, const GRBVar* b, size_t count);
};
using GRBLinExpr = ScipExpr;
using GRBQuadExpr = ScipExpr;
ScipExpr operator+(ScipExpr a, const ScipExpr& b);
ScipExpr operator-(ScipExpr a, const ScipExpr& b);
ScipExpr operator-(const ScipExpr& a);
ScipExpr operator*(const ScipExpr& a, const ScipExpr& b);
ScipExpr operator/(const ScipExpr& a, double divisor);
struct ScipConstraint { ScipExpr expression; char relation; };
ScipConstraint operator<=(const ScipExpr& a, const ScipExpr& b);
ScipConstraint operator>=(const ScipExpr& a, const ScipExpr& b);
ScipConstraint operator==(const ScipExpr& a, const ScipExpr& b);
struct GRBEnv {
  bool log = false;
  explicit GRBEnv(bool = true) {}
  void start() const {}
  void set(GRB_IntParam param, int value);
};
class GRBModel {
  struct Variable { double lower, upper, objective; char type; };
  std::shared_ptr<ScipValues> values = std::make_shared<ScipValues>();
  std::vector<Variable> variables;
  std::vector<ScipConstraint> constraints;
  ScipExpr objective;
  bool explicit_objective = false, log;
  double time_limit = 30, node_limit = 100000;
  int status = 0;
  void check(const ScipExpr& expression) const;
  void run(const char* write_path);
public:
  explicit GRBModel(const GRBEnv& environment) : log(environment.log) {}
  GRBModel(const GRBModel&) = delete;
  GRBModel& operator=(const GRBModel&) = delete;
  GRBVar addVar(double lower, double upper, double obj, char type, const std::string& name = "");
  void addConstr(const ScipConstraint& constraint);
  void addGenConstrAbs(GRBVar result, GRBVar argument);
  void setObjective(const ScipExpr& expression, int sense = GRB_MINIMIZE);
  void set(GRB_IntParam param, int value);
  void set(GRB_DoubleParam param, double value);
  void optimize() { run(nullptr); }
  void write(const std::string& path) { run(path.c_str()); }
  int get(GRB_IntAttr) const { return status; }
  double get(GRB_DoubleAttr attr) const;
};
