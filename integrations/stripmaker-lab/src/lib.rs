//! Interpret only the pinned StripMaker random forest's restricted branch/leaf grammar.
//! Never execute downloaded C++, load pickle files, approximate features or infer drawing quality.
use sha2::{Digest, Sha256};

pub const REVISION: &str = "810a61a03c2177a61f8e32b8ce72c33dccfac49f";
const MAX_BYTES: usize = 4_000_000;
const MAX_NODES: usize = 100_000;
const MAX_DEPTH: usize = 96;

#[derive(Clone, Copy, Debug)]
pub enum Model {
    Local,
    Contextual,
}
impl Model {
    pub fn dimensions(self) -> usize {
        match self {
            Self::Local => 27,
            Self::Contextual => 33,
        }
    }
    pub fn file(self) -> &'static str {
        match self {
            Self::Local => "forest.cpp",
            Self::Contextual => "forest_sec.cpp",
        }
    }
    fn prefix(self) -> &'static str {
        match self {
            Self::Local => "static double clf_tree_",
            Self::Contextual => "static double clf_sec_tree_",
        }
    }
    pub fn sha256(self) -> &'static str {
        match self {
            Self::Local => "21e989dd0df4e77bd809ce383b491466d9ccf98c4cec7191dc4db7d759597efc",
            Self::Contextual => "1a883ae603bb98757b6812e0e7e8b059eb0f6c6b6d4bb8c0a872f68264cc7d6a",
        }
    }
}

#[derive(Debug)]
enum Tree {
    Leaf(f64),
    Branch { feature: usize, threshold: f64, left: Box<Tree>, right: Box<Tree> },
}

pub struct Forest {
    model: Model,
    trees: Vec<Tree>,
    nodes: usize,
}
impl Forest {
    pub fn from_bytes(model: Model, bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_BYTES {
            return Err("classifier exceeds byte limit".into());
        }
        // Git on Windows can convert LF to CRLF; pin the canonical author bytes, not the checkout EOL.
        let source = std::str::from_utf8(bytes).map_err(|_| "classifier must be UTF-8")?.replace("\r\n", "\n");
        let hash = format!("{:x}", Sha256::digest(source.as_bytes()));
        if hash != model.sha256() {
            return Err(format!("classifier hash mismatch: {hash}"));
        }
        let mut trees = Vec::new();
        let mut nodes = 0;
        for (index, fragment) in source.split(model.prefix()).skip(1).enumerate() {
            let (signature, body) = fragment.split_once('{').ok_or("missing tree body")?;
            if signature.trim() != format!("{index}(const double* feature_vec)") {
                return Err("unexpected tree signature/order".into());
            }
            let mut parser = Parser { rest: body, dimensions: model.dimensions(), nodes: &mut nodes };
            let tree = parser.tree(0)?;
            parser.take("}")?;
            trees.push(tree);
            if trees.len() > 150 {
                return Err("too many trees".into());
            }
        }
        if trees.len() != 150 {
            return Err("expected 150 author trees".into());
        }
        Ok(Self { model, trees, nodes })
    }
    pub fn nodes(&self) -> usize {
        self.nodes
    }
    pub fn dimensions(&self) -> usize {
        self.model.dimensions()
    }
    pub fn predict(&self, features: &[f64]) -> Result<f64, String> {
        if features.len() != self.dimensions() || features.iter().any(|v| !v.is_finite()) {
            return Err(format!("expected {} finite features in exact author order", self.dimensions()));
        }
        let mut total = 0.0;
        for tree in &self.trees {
            let mut current = tree;
            loop {
                match current {
                    Tree::Leaf(value) => {
                        total += value;
                        break;
                    }
                    Tree::Branch { feature, threshold, left, right } => {
                        let value = features.get(*feature).ok_or("invalid feature index")?;
                        current = if value <= threshold { left } else { right };
                    }
                }
            }
        }
        Ok(total / 150.0)
    }
    /// Synthetic branch-boundary vectors for inference parity only; not a drawing dataset.
    pub fn probes(&self) -> Vec<Vec<f64>> {
        let mut out = vec![vec![0.0; self.dimensions()]];
        let mut seed = 0x5a173c91u64;
        for _ in 0..128 {
            let mut row = Vec::new();
            for _ in 0..self.dimensions() {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                row.push((seed >> 32) as f64 / u32::MAX as f64 * 200.0 - 50.0);
            }
            out.push(row);
        }
        for tree in &self.trees {
            if let Tree::Branch { feature, threshold, .. } = tree {
                for value in [threshold.next_down(), *threshold, threshold.next_up()] {
                    let mut row = vec![0.0; self.dimensions()];
                    if let Some(cell) = row.get_mut(*feature) {
                        *cell = value;
                    }
                    out.push(row);
                }
            }
        }
        out
    }
}

struct Parser<'a, 'b> {
    rest: &'a str,
    dimensions: usize,
    nodes: &'b mut usize,
}
impl Parser<'_, '_> {
    fn take(&mut self, token: &str) -> Result<(), String> {
        self.rest = self.rest.trim_start();
        self.rest = self.rest.strip_prefix(token).ok_or_else(|| format!("expected {token}"))?;
        Ok(())
    }
    fn number(&mut self) -> Result<f64, String> {
        self.rest = self.rest.trim_start();
        let end = self.rest.find(|c: char| !c.is_ascii_digit() && !matches!(c, '.' | '-' | '+' | 'e' | 'E')).unwrap_or(self.rest.len());
        let token = self.rest.get(..end).ok_or("invalid number")?;
        let value: f64 = token.parse().map_err(|_| "invalid number")?;
        if !value.is_finite() {
            return Err("nonfinite number".into());
        }
        self.rest = self.rest.get(end..).ok_or("invalid number boundary")?;
        Ok(value)
    }
    fn tree(&mut self, depth: usize) -> Result<Tree, String> {
        *self.nodes += 1;
        if depth > MAX_DEPTH || *self.nodes > MAX_NODES {
            return Err("tree complexity limit exceeded".into());
        }
        if self.rest.trim_start().starts_with("return") {
            self.take("return")?;
            let value = self.number()?;
            self.take(";")?;
            if !(0.0..=1.0).contains(&value) {
                return Err("invalid leaf probability".into());
            }
            return Ok(Tree::Leaf(value));
        }
        self.take("if")?;
        self.take("(")?;
        self.take("feature_vec")?;
        self.take("[")?;
        let feature = self.number()?;
        if feature < 0.0 || feature.fract() != 0.0 || feature >= self.dimensions as f64 {
            return Err("invalid feature index".into());
        }
        self.take("]")?;
        self.take("<=")?;
        let threshold = self.number()?;
        self.take(")")?;
        self.take("{")?;
        let left = self.tree(depth + 1)?;
        self.take("}")?;
        self.take("else")?;
        self.take("{")?;
        let right = self.tree(depth + 1)?;
        self.take("}")?;
        Ok(Tree::Branch { feature: feature as usize, threshold, left: Box::new(left), right: Box::new(right) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(s: &str) -> Result<Tree, String> {
        Parser { rest: s, dimensions: 27, nodes: &mut 0 }.tree(0)
    }
    #[test]
    fn branch_is_inclusive_and_has_only_author_grammar() {
        let tree = parse("if (feature_vec[0] <= 1.5) { return 1.0; } else { return 0.0; }").unwrap();
        let mut forest = Forest { model: Model::Local, trees: vec![], nodes: 3 };
        for _ in 0..149 {
            forest.trees.push(Tree::Leaf(1.0));
        }
        forest.trees.push(tree);
        let mut features = vec![0.0; 27];
        features[0] = 1.5;
        assert_eq!(forest.predict(&features).unwrap(), 1.0);
        features[0] = 1.5f64.next_up();
        assert_eq!(forest.predict(&features).unwrap(), 149.0 / 150.0);
    }
    #[test]
    fn malformed_input_fails_closed() {
        for s in [
            "system(1)",
            "return 2.0;",
            "return NaN;",
            "if(feature_vec[27] <= 1){return 0;}else{return 1;}",
            "if(feature_vec[0.5] <= 1){return 0;}else{return 1;}",
        ] {
            assert!(parse(s).is_err());
        }
    }
    #[test]
    fn changed_models_are_rejected_before_parsing() {
        assert!(Forest::from_bytes(Model::Local, b"return 1;").is_err());
        assert!(Forest::from_bytes(Model::Local, &vec![0; MAX_BYTES + 1]).is_err());
    }
    #[test]
    fn invalid_features_never_reach_tree_indexing() {
        let forest = Forest { model: Model::Local, trees: vec![], nodes: 0 };
        assert!(forest.predict(&[]).is_err());
        assert!(forest.predict(&vec![f64::INFINITY; 27]).is_err());
    }
}
