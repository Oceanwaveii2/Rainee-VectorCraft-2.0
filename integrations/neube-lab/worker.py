"""Machine-local NeuBE research worker. Rust editor never imports Python or checkpoints."""
import functools, hashlib, json, os, sys, time
from pathlib import Path
ROOT=Path("D:/DevData/Rainee/research/NeuBE")
MODEL=ROOT/"models/neube/style1/snapshot.pkl"
if hashlib.sha256(MODEL.read_bytes()).hexdigest()!="35f276c79c99fca68a851011ade68721c3bbb8db33fbeabcd514e32dfe61278f":
    raise RuntimeError("NeuBE checkpoint changed; re-audit before loading")
sys.path.insert(0,str(ROOT))
sys.path.insert(0,str(ROOT/"thirdparty/stylegan2_ada_pytorch"))
os.environ.setdefault("TORCH_EXTENSIONS_DIR","D:/DevData/Rainee/cache/torch-extensions")
import numpy as np
import torch
from PIL import Image, ImageDraw
sys.path.insert(0,str(Path(__file__).resolve().parent))
from appearance_support import constrain_appearance
# Use author's reference PyTorch kernels on CUDA, not a CUDA-toolkit JIT extension.
from torch_utils.ops import upfirdn2d, bias_act
def reference(fn):
    @functools.wraps(fn)
    def wrapped(*args,**kwargs):
        kwargs["impl"]="ref"
        return fn(*args,**kwargs)
    return wrapped
for name in ["upfirdn2d","filter2d","upsample2d","downsample2d"]:
    setattr(upfirdn2d,name,reference(getattr(upfirdn2d,name)))
bias_act.bias_act=reference(bias_act.bias_act)
# Legacy author checkpoints require trusted-object deserialization; confined to this worker.
original_load=torch.load
def legacy_load(*args,**kwargs):
    kwargs.setdefault("weights_only",False)
    return original_load(*args,**kwargs)
torch.load=legacy_load
from forger.ui.brush import PaintEngineFactory, PaintingHelper, GanBrushOptions
from forger.viz.style_transfer import generate_stitching_crops
def main():
    inp=Path(sys.argv[1]); output=Path(sys.argv[2])
    job=json.loads(inp.read_text(encoding="utf-8-sig"))
    width,height=job["size"]
    if not 32<=width<=512 or not 32<=height<=512: raise ValueError("Raster preview dimensions outside 32..512")
    if not 1<=len(job["strokes"])<=64: raise ValueError("Invalid stroke count")
    geom=Image.new("L",(width,height),255); draw=ImageDraw.Draw(geom)
    x0,y0,scale=job["mapping"]
    for stroke in job["strokes"]:
        points=stroke["points"]
        if not 2<=len(points)<=4096: raise ValueError("Invalid sample count")
        coords=[((x-x0)*scale,(y-y0)*scale) for x,y in points]
        draw.line(coords,fill=0,width=max(1,round(stroke["width"]*scale)),joint="curve")
    device=torch.device("cuda:0")
    if not torch.cuda.is_available(): raise RuntimeError("CUDA PyTorch device unavailable")
    torch.cuda.reset_peak_memory_stats(); begin=time.perf_counter()
    engine=PaintEngineFactory.create(gan_checkpoint=str(MODEL),device=device)
    helper=PaintingHelper(engine); helper.set_render_mode("clear")
    seed=int(job.get("seed",594))
    options=GanBrushOptions(); options.set_style(engine.random_style(seed),seed)
    options.enable_uvs_mapping=True
    color=job.get("color",[0.1,0.25,0.65])
    options.set_color(0,torch.tensor(color,dtype=torch.float32)); options.set_color(1,torch.tensor(color,dtype=torch.float32))
    margin=10
    padded=np.full((height+margin,width+margin,1),255,dtype=np.uint8)
    padded[margin:,margin:,0]=np.asarray(geom)
    crops,padded=generate_stitching_crops(padded,engine.patch_width,mode="all",overlap_margin=margin*2)
    helper.make_new_canvas(padded.shape[0],padded.shape[1],feature_blending=2)
    result=np.zeros((*padded.shape[:2],4),dtype=np.uint8)
    with torch.inference_mode():
        for crop in crops:
            y,x=crop[0],crop[1]
            options.set_position(int(x),int(y))
            patch=255-padded[y:y+engine.patch_width,x:x+engine.patch_width,:]
            rendered,_,meta=helper.render_stroke(patch,None,options,meta={"x":int(x),"y":int(y),"crop_margin":margin})
            yy,xx=meta["y"],meta["x"]
            result[yy:yy+rendered.shape[0],xx:xx+rendered.shape[1]]=rendered
    result=result[margin:margin+height,margin:margin+width]
    result,clipped=constrain_appearance(result,geom)
    Image.fromarray(result,"RGBA").save(output)
    print(json.dumps({"backend":"NeuBE style1","seed":seed,"gpu":torch.cuda.get_device_name(0),"seconds":time.perf_counter()-begin,"peak_gpu_bytes":torch.cuda.max_memory_allocated(),"dimensions":[width,height],"checkpoint_sha256":hashlib.sha256(MODEL.read_bytes()).hexdigest(),"kernels":"author PyTorch reference on CUDA","support":"12px dilation with 2px soft edge","background_pixels_removed":clipped}))
if __name__=="__main__": main()
