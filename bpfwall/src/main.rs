use aya::Bpf;
use aya::programs::{Xdp, XdpFlags};
use std::{
    convert::{TryFrom,TryInto},
    sync::Arc,
    sync::atomic::{AtomicBool, Ordering},
};
use structopt::StructOpt;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("error: {:#}", e);
    }
}

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short, long)]
    path: String,
    #[structopt(short, long, default_value = "eth0")]
    iface: String,
}

fn try_main() -> Result<(), anyhow::Error> {
    let opt = Opt::from_args();
    let mut bpf = Bpf::load_file(&opt.path)?;
    let program: &mut Xdp = bpf.program_mut("bpfwall")?.try_into()?;
    program.load()?;
    program.attach(&opt.iface, XdpFlags::default())?;
    
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    println!("Waiting for Ctrl-C...");
    while running.load(Ordering::SeqCst) {}
    println!("Exiting...");

    Ok(())
}