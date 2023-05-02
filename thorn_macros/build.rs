use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    let mut keywords = phf_codegen::Set::new();

    for keyword in include_str!("./keywords.txt").split_whitespace() {
        keywords.entry(keyword);
    }

    write!(
        &mut file,
        "static KEYWORDS: phf::Set<&'static str> = {}",
        keywords.build()
    )?;

    write!(&mut file, ";\n")?;

    Ok(())
}
