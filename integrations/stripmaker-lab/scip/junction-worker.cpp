// Isolated research adapter; never linked into the Rust editor.
#include "classifier.h"
#include "junction_features.h"
#include "bvh.h"
#include <nlohmann/json.hpp>
#include <fstream>
#include <iostream>
#include <cmath>
using nlohmann::json;
using namespace sketching;
int main(int argc, char** argv) {
  try {
    if(argc != 3) throw std::runtime_error("Expected input.json output.json");
    std::ifstream in(argv[1]); if(!in) throw std::runtime_error("Input missing");
    auto j=json::parse(in); auto input=j.at("strokes");
    if(!input.is_array() || input.size()<2 || input.size()>64) throw std::runtime_error("Select 2-64 strokes");
    double gap=j.value("gap",8.0); if(!std::isfinite(gap)||gap<=0||gap>32) throw std::runtime_error("Invalid gap");
    std::vector<Stroke> strokes; strokes.reserve(input.size()); size_t total=0;
    for(auto& item: input) {
      auto points=item.at("points"); double width=item.at("width"); total+=points.size();
      if(points.size()<2||points.size()>4096||total>16384||!std::isfinite(width)||width<=0) throw std::runtime_error("Invalid stroke limits");
      strokes.emplace_back((Index)points.size(),false); auto& s=strokes.back();
      for(size_t k=0;k<points.size();++k) {
        double x=points[k][0],y=points[k][1];
        if(!std::isfinite(x)||!std::isfinite(y)||std::abs(x)>1e7||std::abs(y)>1e7) throw std::runtime_error("Invalid point");
        s.x((Index)k)=x; s.y((Index)k)=y; s.width((Index)k)=width;
      }
      if(s.length()<1e-6) throw std::runtime_error("Degenerate stroke");
    }
    PolylineBVH bvh(span<const Stroke>(strokes.data(),strokes.size()));
    auto ee=get_end_end_features(); auto es=get_end_stroke_features();
    for(auto& f:ee) f->init(bvh); for(auto& f:es) f->init(bvh);
    json output=json::array(); size_t evaluations=0;
    auto score=[&](size_t a,bool tail,size_t b,double t,const Vec2& q) {
      const auto& s1=strokes[a]; const auto& s2=strokes[b]; auto p=s1.pos_norm(tail?1.0:0.0);
      double dx=p.x()-q.x(),dy=p.y()-q.y(),dist=std::hypot(dx,dy);
      if(dist<=0.001||dist>gap||evaluations>=4096) return;
      bool endpoint=(t==0.0||t==1.0); double prob=0;
      if(endpoint) {
        EndEndFeatures f{}; for(size_t i=0;i<ee.size();++i) f.data_[i]=(*ee[i])(s1,tail?s1.length():0.0,a,s2,t*s2.length(),b);
        for(double value:f.data_) if(!std::isfinite(value)) return;
        prob=clf_endpoint(f);
      } else {
        EndStrokeFeatures f{}; for(size_t i=0;i<es.size();++i) f.data_[i]=(*es[i])(s1,tail?s1.length():0.0,a,s2,t*s2.length(),b);
        for(double value:f.data_) if(!std::isfinite(value)) return;
        prob=clf_tjunction(f);
      }
      ++evaluations; if(!std::isfinite(prob)) return;
      output.push_back({{"a",a},{"tail",tail},{"b",b},{"t",t},{"point",{q.x(),q.y()}},{"confidence",std::clamp(prob,0.0,1.0)},{"kind",endpoint?"endpoint":"t-junction"}});
    };
    for(size_t a=0;a<strokes.size();++a) for(bool tail:{false,true}) for(size_t b=0;b<strokes.size();++b) {
      if(a==b) continue;
      const auto& s=strokes[b]; auto p=strokes[a].pos_norm(tail?1.0:0.0);
      // Endpoint pairs are evaluated once, separately from interior projection.
      if(a<b) { score(a,tail,b,0.0,s.xy(0)); score(a,tail,b,1.0,s.xy(Back)); }
      double best=1e30,arc=0,best_arc=0; Vec2 nearest;
      for(Index k=1;k<s.size();++k) {
        auto u=s.xy(k-1),v=s.xy(k); double vx=v.x()-u.x(),vy=v.y()-u.y(),len=std::hypot(vx,vy);
        double t=len>0?std::clamp(((p.x()-u.x())*vx+(p.y()-u.y())*vy)/(len*len),0.0,1.0):0;
        Vec2 q(u.x()+vx*t,u.y()+vy*t); double d=std::hypot(p.x()-q.x(),p.y()-q.y());
        if(d<best) {best=d;nearest=q;best_arc=arc+t*len;} arc+=len;
      }
      double t=best_arc/s.length(); if(t>1e-5&&t<1.0-1e-5) score(a,tail,b,t,nearest);
    }
    std::ofstream out(argv[2]); out<<json({{"schema",1},{"evaluations",evaluations},{"suggestions",output}}).dump();
    if(!out) throw std::runtime_error("Output write failed"); return 0;
  } catch(const std::exception& e) {std::cerr<<e.what()<<'\n';return 1;}
}
