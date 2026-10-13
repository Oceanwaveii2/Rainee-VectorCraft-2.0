//! Restricted CPU evaluator for the pinned AOSP FLOAT32 motion model, not a general TFLite importer.
//! Schema: TensorFlow v2.20.0 tensorflow/compiler/mlir/lite/schema/schema.fbs.
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, path::Path};

pub const SHA256: &str = "66dd372bbbc7717cf64067c1257117d35d6a2c727506ea39cb1cb51b83d03bd7";
type Result<T> = std::result::Result<T, String>;

struct Fb<'a>(&'a [u8]);
impl Fb<'_> {
    fn bytes(&self, at: usize, size: usize) -> Result<&[u8]> {
        self.0.get(at..at.checked_add(size).ok_or("Offset overflow")?).ok_or_else(|| "Truncated FlatBuffer".into())
    }
    fn u32(&self, at: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(at, 4)?.try_into().map_err(|_| "Bad u32")?))
    }
    fn i32(&self, at: usize) -> Result<i32> {
        Ok(self.u32(at)? as i32)
    }
    fn u16(&self, at: usize) -> Result<usize> {
        Ok(u16::from_le_bytes(self.bytes(at, 2)?.try_into().map_err(|_| "Bad u16")?) as usize)
    }
    fn field(&self, table: usize, index: usize) -> Result<Option<usize>> {
        let vtable = usize::try_from(table as i64 - i64::from(self.i32(table)?)).map_err(|_| "Bad vtable")?;
        let slot = 4 + index * 2;
        if slot + 2 > self.u16(vtable)? {
            return Ok(None);
        }
        let offset = self.u16(vtable + slot)?;
        Ok((offset != 0).then_some(table + offset))
    }
    fn indirect(&self, at: usize) -> Result<usize> {
        at.checked_add(self.u32(at)? as usize).ok_or_else(|| "Offset overflow".into())
    }
    fn scalar(&self, table: usize, field: usize, default: i32) -> Result<i32> {
        self.field(table, field)?.map_or(Ok(default), |at| self.i32(at))
    }
    fn byte(&self, table: usize, field: usize) -> Result<u8> {
        self.field(table, field)?.map_or(Ok(0), |at| Ok(*self.bytes(at, 1)?.first().ok_or("Missing byte")?))
    }
    fn vector(&self, table: usize, field: usize, width: usize) -> Result<Vec<usize>> {
        let Some(at) = self.field(table, field)? else {
            return Ok(Vec::new());
        };
        let start = self.indirect(at)?;
        let len = self.u32(start)? as usize;
        if len > 200_000 {
            return Err("Vector limit exceeded".into());
        }
        self.bytes(start + 4, len.checked_mul(width).ok_or("Vector overflow")?)?;
        Ok((0..len).map(|i| start + 4 + i * width).collect())
    }
    fn tables(&self, table: usize, field: usize) -> Result<Vec<usize>> {
        self.vector(table, field, 4)?.into_iter().map(|at| self.indirect(at)).collect()
    }
    fn indices(&self, table: usize, field: usize) -> Result<Vec<usize>> {
        self.vector(table, field, 4)?.into_iter().map(|at| usize::try_from(self.i32(at)?).map_err(|_| "Negative tensor index".into())).collect()
    }
    fn string(&self, table: usize, field: usize) -> Result<String> {
        let at = self.indirect(self.field(table, field)?.ok_or("Missing string")?)?;
        let len = self.u32(at)? as usize;
        std::str::from_utf8(self.bytes(at + 4, len)?).map(str::to_owned).map_err(|e| e.to_string())
    }
}

#[derive(Clone)]
struct Tensor {
    shape: Vec<usize>,
    values: Vec<f32>,
}
struct Op {
    code: i32,
    inputs: Vec<usize>,
    output: usize,
    approximate: bool,
}
pub struct Model {
    tensors: Vec<Tensor>,
    ops: Vec<Op>,
    pub inputs: BTreeMap<String, usize>,
    pub outputs: BTreeMap<String, usize>,
    pub input_length: usize,
    pub output_length: usize,
}
impl Model {
    pub fn load(path: &Path) -> Result<Self> {
        let mut bytes = Vec::new();
        std::fs::File::open(path).map_err(|e| e.to_string())?.take(1_048_577).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        if bytes.len() != 179_532 || format!("{:x}", Sha256::digest(&bytes)) != SHA256 {
            return Err("Model size/SHA256 differs from pinned AOSP artifact".into());
        }
        Self::parse(&bytes)
    }
    fn parse(bytes: &[u8]) -> Result<Self> {
        let fb = Fb(bytes);
        if fb.bytes(4, 4)? != b"TFL3" {
            return Err("Not TFLite schema 3".into());
        }
        let root = fb.u32(0)? as usize;
        if fb.scalar(root, 0, 0)? != 3 {
            return Err("Unsupported model schema".into());
        }
        let graphs = fb.tables(root, 2)?;
        if graphs.len() != 1 {
            return Err("Only one subgraph supported".into());
        }
        let graph = *graphs.first().ok_or("Missing graph")?;
        let buffers = fb.tables(root, 4)?;
        let mut tensors = Vec::new();
        for table in fb.tables(graph, 0)? {
            if fb.byte(table, 1)? != 0 || fb.byte(table, 5)? != 0 || fb.field(table, 6)?.is_some() {
                return Err("Only dense constant FLOAT32 tensors supported".into());
            }
            let shape = fb.indices(table, 0)?;
            let len = shape.iter().try_fold(1usize, |a, b| a.checked_mul(*b).ok_or("Tensor overflow"))?;
            if len > 200_000 {
                return Err("Tensor too large".into());
            }
            let buffer = *buffers.get(fb.scalar(table, 2, 0)? as usize).ok_or("Missing buffer")?;
            let raw = fb.vector(buffer, 0, 1)?;
            let mut values = vec![0.0; len];
            if !raw.is_empty() {
                if raw.len() != len * 4 {
                    return Err("Float buffer size mismatch".into());
                }
                for (out, chunk) in values.iter_mut().zip(raw.as_chunks::<4>().0) {
                    let at = *chunk.first().ok_or("Missing float")?;
                    *out = f32::from_bits(fb.u32(at)?);
                    if !out.is_finite() {
                        return Err("Nonfinite model weight".into());
                    }
                }
            }
            tensors.push(Tensor { shape, values });
        }
        let codes = fb.tables(root, 1)?.into_iter().map(|table| fb.scalar(table, 3, i32::from(fb.byte(table, 0)?))).collect::<Result<Vec<_>>>()?;
        let mut ops = Vec::new();
        for table in fb.tables(graph, 3)? {
            let code = *codes.get(fb.scalar(table, 0, 0)? as usize).ok_or("Missing opcode")?;
            let mut inputs = Vec::new();
            for (slot, at) in fb.vector(table, 1, 4)?.into_iter().enumerate() {
                let id = fb.i32(at)?;
                if id == -1 && code == 9 && slot == 2 {
                    continue;
                }
                inputs.push(usize::try_from(id).map_err(|_| "Negative tensor index")?);
            }
            let outputs = fb.indices(table, 2)?;
            if outputs.len() != 1 || inputs.iter().chain(outputs.iter()).any(|i| *i >= tensors.len()) {
                return Err("Invalid op tensors".into());
            }
            let output = *outputs.first().ok_or("Missing output")?;
            let options = fb.field(table, 4)?.map(|at| fb.indirect(at)).transpose()?;
            let approximate = match (code, options) {
                (2, Some(o))
                    if matches!(fb.scalar(o, 0, 0)?, 1 | -1)
                        && fb.byte(o, 1)? == 0
                        && inputs.iter().all(|i| tensors.get(*i).is_some_and(|t| t.shape.len() == 2 && t.shape.first() == Some(&1))) =>
                {
                    false
                }
                (9, Some(o)) if fb.byte(o, 0)? == 0 && fb.byte(o, 1)? == 0 && fb.byte(o, 2)? == 0 && matches!(inputs.len(), 2 | 3) => false,
                (150, Some(o)) if inputs.len() == 1 => fb.byte(o, 0)? != 0,
                _ => {
                    return Err(format!(
                        "Unsupported op/options {code}: scalar0={:?}, byte1={:?}, shapes={:?}",
                        options.map(|o| fb.scalar(o, 0, 0)),
                        options.map(|o| fb.byte(o, 1)),
                        inputs.iter().filter_map(|i| tensors.get(*i)).map(|t| &t.shape).collect::<Vec<_>>()
                    ));
                }
            };
            ops.push(Op { code, inputs, output, approximate });
        }
        let signature =
            fb.tables(root, 7)?.into_iter().find(|s| fb.string(*s, 2).is_ok_and(|v| v == "serving_default")).ok_or("Missing serving_default")?;
        let maps = |field| -> Result<BTreeMap<String, usize>> {
            fb.tables(signature, field)?.into_iter().map(|t| Ok((fb.string(t, 0)?, fb.scalar(t, 1, 0)? as usize))).collect()
        };
        let inputs = maps(0)?;
        let outputs = maps(1)?;
        let mut model = Self { tensors, ops, inputs, outputs, input_length: 0, output_length: 0 };
        model.input_length = model.named("r", true)?.values.len();
        model.output_length = model.named("r", false)?.values.len();
        for key in ["r", "phi", "pressure", "tilt", "orientation"] {
            if model.named(key, true)?.values.len() != model.input_length {
                return Err("Input shape mismatch".into());
            }
        }
        for key in ["r", "phi", "pressure"] {
            if model.named(key, false)?.values.len() != model.output_length {
                return Err("Output shape mismatch".into());
            }
        }
        if model.input_length == 0 || model.output_length < 4 {
            return Err("Model horizon too short".into());
        }
        Ok(model)
    }
    fn named(&self, name: &str, input: bool) -> Result<&Tensor> {
        let map = if input { &self.inputs } else { &self.outputs };
        self.tensors.get(*map.get(name).ok_or("Missing named tensor")?).ok_or_else(|| "Bad named tensor".into())
    }
    pub fn infer(&mut self, features: &[Vec<f32>; 5]) -> Result<[Vec<f32>; 3]> {
        for (key, data) in ["r", "phi", "pressure", "tilt", "orientation"].iter().zip(features) {
            if data.len() != self.input_length || data.iter().any(|v| !v.is_finite()) {
                return Err("Invalid inference input".into());
            }
            let id = *self.inputs.get(*key).ok_or("Missing input")?;
            self.tensors.get_mut(id).ok_or("Missing input tensor")?.values.clone_from(data);
        }
        for op in &self.ops {
            let get = |n: usize| -> Result<&Tensor> {
                self.tensors.get(*op.inputs.get(n).ok_or("Missing op input")?).ok_or_else(|| "Bad op tensor".into())
            };
            let values: Vec<f32> = match op.code {
                2 => op
                    .inputs
                    .iter()
                    .map(|id| self.tensors.get(*id).ok_or("Bad concat input"))
                    .collect::<std::result::Result<Vec<_>, _>>()?
                    .iter()
                    .flat_map(|t| t.values.iter().copied())
                    .collect(),
                9 => {
                    let x = get(0)?;
                    let w = get(1)?;
                    let b = if op.inputs.len() == 3 { Some(get(2)?) } else { None };
                    if w.shape.len() != 2
                        || x.values.is_empty()
                        || w.shape.get(1) != Some(&x.values.len())
                        || b.is_some_and(|b| w.shape.first() != Some(&b.values.len()))
                    {
                        return Err("Dense shape mismatch".into());
                    }
                    w.values
                        .chunks_exact(x.values.len())
                        .enumerate()
                        .map(|(i, row)| {
                            row.iter().zip(&x.values).fold(b.and_then(|b| b.values.get(i)).copied().unwrap_or(0.0), |sum, (a, b)| sum + a * b)
                        })
                        .collect()
                }
                150 => get(0)?.values.iter().map(|x| gelu(*x, op.approximate)).collect(),
                _ => return Err("Unsupported op".into()),
            };
            let out = self.tensors.get_mut(op.output).ok_or("Bad output")?;
            if out.values.len() != values.len() || values.iter().any(|v| !v.is_finite()) {
                return Err("Invalid inference output".into());
            }
            out.values = values;
        }
        Ok([self.named("r", false)?.values.clone(), self.named("phi", false)?.values.clone(), self.named("pressure", false)?.values.clone()])
    }
    pub fn describe(&self) -> String {
        format!(
            "AOSP model: {} tensors, {} ops, {} history steps, {} predicted steps",
            self.tensors.len(),
            self.ops.len(),
            self.input_length,
            self.output_length
        )
    }
}
fn gelu(x: f32, approximate: bool) -> f32 {
    if approximate {
        0.5 * x * (1.0 + (0.797_884_6 * (x + 0.044715 * x.powi(3))).tanh())
    } else {
        0.5 * x * (1.0 + libm::erff(x * std::f32::consts::FRAC_1_SQRT_2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gelu_matches_known_values_and_flatbuffers_are_bounded() {
        assert!((gelu(1.0, false) - 0.841_344_7).abs() < 1e-6);
        assert!((gelu(-1.0, false) + 0.158_655_26).abs() < 1e-6);
        for n in 0..32 {
            assert!(Model::parse(&vec![0; n]).is_err());
        }
    }
    #[test]
    fn installed_pinned_model_executes_and_rejects_invalid_features() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../integrations/cintiq-input-probe/models/aosp/motion_predictor_model.tflite");
        if !path.exists() {
            eprintln!("Optional downloaded model absent; run --evaluate-prediction for model acceptance");
            return;
        }
        let mut m = Model::load(&path).unwrap();
        eprintln!("{}", m.describe());
        let mut f = std::array::from_fn(|_| vec![0.0; m.input_length]);
        f[0].fill(4.0);
        f[2].fill(0.5);
        let y = m.infer(&f).unwrap();
        assert!(y.iter().flatten().all(|v| v.is_finite()));
        assert!(y[0].iter().all(|v| *v > 0.2));
        f[0][0] = f32::NAN;
        assert!(m.infer(&f).is_err());
        assert!(Model::load(Path::new("Cargo.toml")).is_err());
    }
}
