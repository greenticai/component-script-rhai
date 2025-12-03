use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).expect("Usage: hash-wasm <path-to-wasm>");
    let data = fs::read(&path)?;
    let hash = blake3::hash(&data);
    println!("blake3:{hash}");
    Ok(())
}
