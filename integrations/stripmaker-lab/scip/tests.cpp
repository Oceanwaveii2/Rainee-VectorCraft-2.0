#include "gurobi_c++.h"
#include <scip/scip.h>
#include <osqp.h>
#include <functional>
#include <iostream>
#include <limits>

void require(bool condition, const char* message) { if (!condition) throw std::runtime_error(message); }
void near(double a, double b, double tolerance = 1e-3) { require(std::abs(a - b) <= tolerance, "Solution mismatch"); }
void rejects(const std::function<void()>& operation) {
  bool rejected = false;
  try { operation(); } catch (const GRBException&) { rejected = true; }
  require(rejected, "Invalid operation was not rejected");
}
int main() {
  try {
    GRBEnv env;
    int count = 0;
    {
      GRBModel m(env); auto x = m.addVar(0, 1, 0, GRB_BINARY); auto y = m.addVar(0, 1, 0, GRB_BINARY);
      m.addConstr(x == 1); m.setObjective(-3 * (x-y)*(x-y)); m.optimize();
      near(x.get(GRB_DoubleAttr_X), 1); near(y.get(GRB_DoubleAttr_X), 0); near(m.get(GRB_DoubleAttr_ObjVal), -3); ++count;
    }
    {
      GRBModel m(env); auto x=m.addVar(-GRB_INFINITY, GRB_INFINITY, 0, GRB_CONTINUOUS);
      auto y=m.addVar(-GRB_INFINITY, GRB_INFINITY, 0, GRB_CONTINUOUS);
      m.addConstr(x+y == 3); m.addConstr(x>=0); m.setObjective((x-1)*(x-1)+(y-2)*(y-2)); m.optimize();
      near(x.get(GRB_DoubleAttr_X),1); near(y.get(GRB_DoubleAttr_X),2); ++count;
      m.setObjective((x-2)*(x-2)+(y-1)*(y-1)); rejects([&]{x.get(GRB_DoubleAttr_X);}); m.optimize();
      near(x.get(GRB_DoubleAttr_X),2); ++count;
    }
    {
      GRBModel m(env); auto x=m.addVar(-10, 10, 0, GRB_CONTINUOUS); auto y=m.addVar(-10,10,0,GRB_CONTINUOUS);
      m.addConstr(x == -2); m.addGenConstrAbs(y,x); m.setObjective(0); m.optimize();
      near(y.get(GRB_DoubleAttr_X),2); ++count;
    }
    {
      GRBModel m(env); auto x=m.addVar(-10,10,0,GRB_CONTINUOUS); auto a=m.addVar(0,20,0,GRB_CONTINUOUS);
      m.addConstr(x<=0); m.addConstr(a>=x-2); m.addConstr(a>=-(x-2)); m.setObjective(a); m.optimize();
      near(x.get(GRB_DoubleAttr_X),0); near(a.get(GRB_DoubleAttr_X),2); ++count;
    }
    {
      GRBModel m(env); auto x=m.addVar(0,1,0,GRB_CONTINUOUS); m.addConstr(x>=2);
      rejects([&]{m.optimize();}); require(m.get(GRB_IntAttr_Status)==GRB_INFEASIBLE,"Wrong infeasible status");
      rejects([&]{x.get(GRB_DoubleAttr_X);}); ++count;
    }
    {
      GRBModel m(env); auto x=m.addVar(-GRB_INFINITY,GRB_INFINITY,0,GRB_CONTINUOUS); m.setObjective(-ScipExpr(x));
      rejects([&]{m.optimize();}); rejects([&]{x.get(GRB_DoubleAttr_X);}); ++count;
    }
    {
      GRBModel m(env); auto x=m.addVar(-10,10,0,GRB_CONTINUOUS); m.setObjective((x-1)*(x-1));
      m.set(GRB_DoubleParam_TimeLimit,0); rejects([&]{m.optimize();}); rejects([&]{x.get(GRB_DoubleAttr_X);}); ++count;
    }
    {
      GRBModel a(env),b(env); auto x=a.addVar(0,1,0,GRB_BINARY); auto y=b.addVar(0,1,0,GRB_BINARY);
      rejects([&]{a.setObjective(x+y);}); rejects([&]{a.addConstr(y>=0);});
      rejects([&]{a.addVar(2,1,0,GRB_CONTINUOUS);});
      rejects([&]{a.setObjective(std::numeric_limits<double>::quiet_NaN());});
      rejects([&]{a.setObjective(ScipExpr(x)*x*x);}); ++count;
    }
    {
      GRBModel m(env); GRBVar v[]={m.addVar(0,3,0,GRB_CONTINUOUS),m.addVar(0,3,0,GRB_CONTINUOUS)};
      double q[]={1,1},l[]={-2,-4}; ScipExpr e=5; e.addTerms(q,v,v,2); e.addTerms(l,v,2);
      m.setObjective(e); m.optimize(); near(v[0].get(GRB_DoubleAttr_X),1); near(v[1].get(GRB_DoubleAttr_X),2); ++count;
    }
    for (int seed=0;seed<20;++seed) {
      GRBModel m(env); std::vector<GRBVar> vars;
      for(int i=0;i<6;++i) vars.push_back(m.addVar(0,1,0,GRB_BINARY));
      m.addConstr(vars[0]==1); ScipExpr objective=0;
      for(int i=0;i<6;++i) for(int j=i+1;j<6;++j) {
        double weight=((seed*13+i*5+j*7)%11)-5;
        objective+=weight*(vars[i]-vars[j])*(vars[i]-vars[j]);
      }
      double best=std::numeric_limits<double>::infinity();
      for(int bits=1;bits<64;bits+=2) {
        double val=0;
        for(int i=0;i<6;++i) for(int j=i+1;j<6;++j) {
          double d=((bits>>i)&1)-((bits>>j)&1);
          val+=(((seed*13+i*5+j*7)%11)-5)*d*d;
        }
        best=std::min(best,val);
      }
      m.setObjective(objective); m.optimize(); near(m.get(GRB_DoubleAttr_ObjVal),best,1e-6); ++count;
    }
    {
      GRBModel m(env); auto x=m.addVar(-10,10,0,GRB_CONTINUOUS), y=m.addVar(-10,10,0,GRB_CONTINUOUS);
      // Cross-term coefficient and constant offsets must survive CSC conversion.
      m.addConstr(x+y>=4); m.setObjective((x+y-5)*(x+y-5)+(x-y-1)*(x-y-1)); m.optimize();
      near(x.get(GRB_DoubleAttr_X),3,1e-5); near(y.get(GRB_DoubleAttr_X),2,1e-5); ++count;
    }
    {
      GRBModel m(env); auto x=m.addVar(-10,10,0,GRB_CONTINUOUS), y=m.addVar(-10,10,0,GRB_CONTINUOUS);
      auto a=m.addVar(0,20,0,GRB_CONTINUOUS);
      m.addConstr(a>=y-2); m.addConstr(a>=-(y-2));
      m.setObjective((x-y)*(x-y)+0.01*a); m.optimize();
      near(x.get(GRB_DoubleAttr_X),2,1e-4); near(y.get(GRB_DoubleAttr_X),2,1e-4); ++count;
    }
    {
      GRBModel m(env); auto x=m.addVar(0,1,0,GRB_CONTINUOUS);
      m.addConstr(x>=2); m.setObjective(ScipExpr(x)*x);
      rejects([&]{m.optimize();}); require(m.get(GRB_IntAttr_Status)==GRB_INFEASIBLE,"QP infeasible status");
      rejects([&]{x.get(GRB_DoubleAttr_X);}); ++count;
    }
    {
      // This objective is not convex; it must remain on SCIP, not OSQP.
      GRBModel m(env); auto x=m.addVar(-2,3,0,GRB_CONTINUOUS);
      m.setObjective(-ScipExpr(x)*x); m.optimize();
      near(x.get(GRB_DoubleAttr_X),3); near(m.get(GRB_DoubleAttr_ObjVal),-9); ++count;
    }
    std::cout << "SCIP " << SCIPmajorVersion()<<'.'<<SCIPminorVersion()<<'.'<<SCIPtechVersion()
      << " + OSQP " << osqp_version() << ": "<<count<<" adapter checks passed\n"; return 0;
  } catch (const std::exception& e) { std::cerr<<e.what()<<'\n'; return 1; }
}
