fn main() -> Result<(), String> {
    let mut m = vectorcraft_tools::motion_model::Model::load(std::path::Path::new(vectorcraft_tools::learned_prediction::MODEL_PATH))?;
    let mut cases = Vec::new();
    for k in 0..48 {
        let f: [Vec<f32>; 5] = std::array::from_fn(|c| {
            (0..5)
                .map(|i| match c {
                    0 => 0.2 + (k % 8) as f32 * 2.5 + i as f32 * 0.1,
                    1 => (k % 5) as f32 * 0.15 - 0.3,
                    2 => 0.1 + (k % 9) as f32 * 0.1,
                    3 => (k % 6) as f32 * 0.2,
                    _ => (k % 7) as f32 * 0.5 - 1.5,
                })
                .collect()
        });
        cases.push(serde_json::json!({"features":f,"output":m.infer(&f)?}));
    }
    println!("{}", serde_json::json!({"sha256":vectorcraft_tools::motion_model::SHA256,"cases":cases}));
    Ok(())
}
