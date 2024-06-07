use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

  for entity in ["journal", "account"] {
    tonic_build::configure()
      .file_descriptor_set_path(out_dir.join(format!("{entity}_descriptor.bin")))
      .compile(&[format!("proto/whiterabbit/{entity}/v1/{entity}.proto")], &["proto"])
      .unwrap();
  }

  Ok(())
}
