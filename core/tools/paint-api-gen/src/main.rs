use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let check = match env::args().nth(1).as_deref() {
        None => false,
        Some("--check") => true,
        Some(_) => return Err("usage: paint-api-gen [--check]".into()),
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let crate_dir = root.join("core/crates/paint-ffi");
    let header = root.join("ui/include/paint_api.h");
    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml"))?;
    let bindings = cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()?;
    let mut generated = Vec::new();
    bindings.write(&mut generated);
    if check {
        if fs::read(&header)? != generated {
            return Err("paint_api.h drifted; run cargo run -p paint-api-gen --locked".into());
        }
        println!("Generated C header is up to date.");
    } else {
        fs::create_dir_all(header.parent().ok_or("header directory missing")?)?;
        fs::write(&header, generated)?;
        println!("Generated {}", header.display());
    }
    Ok(())
}
