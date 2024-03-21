use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["protos/msg.proto"], &["protos/"])?;
    Ok(())
}
