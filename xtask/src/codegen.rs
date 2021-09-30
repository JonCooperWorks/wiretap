use aya_gen::btf_types;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Options {
    #[structopt(short, long)]
    names: Vec<String>,
    #[structopt(short, long, default_value="bpfwall-ebpf/src")]
    bpf_directory: String,
    #[structopt(short, long, default_value="/sys/kernel/btf/vmlinux")]
    vmlinux_path: String
}


pub fn generate(opts: Options) -> Result<(), anyhow::Error> {
    let dir = PathBuf::from(&opts.bpf_directory);
    let vmlinux_path = Path::new(&opts.vmlinux_path);
    let bindings = btf_types::generate(vmlinux_path, &opts.names, true)?;
    println!("Generating bindings {} for {:#?} to {}/bindings.rs", opts.vmlinux_path, opts.names.join(", "), opts.bpf_directory);
    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let mut out = File::create(dir.join("bindings.rs"))?;
    write!(out, "{}", bindings)?;
    Ok(())
}