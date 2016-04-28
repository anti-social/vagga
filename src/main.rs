#[allow(unused)]

use std::env;

extern crate shaman;
extern crate libc;
extern crate nix;
extern crate rand;
extern crate rustc_serialize;
extern crate env_logger;
extern crate argparse;
extern crate quire;
extern crate time;
extern crate unshare;
extern crate libmount;
extern crate signal;
extern crate regex;
extern crate scan_dir;
extern crate zip;
extern crate tar;
extern crate flate2;
extern crate bzip2;
extern crate xz2;
#[macro_use] extern crate matches;
#[macro_use] extern crate mopa;
#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;

#[macro_use] mod macros;
mod config;
mod container;
mod file_util;
mod path_util;
mod process_util;
mod tty_util;
mod options;
mod digest;
mod build_step;

// Commands
mod launcher;
mod network;
mod setup_netns;
mod version;
mod wrapper;
mod builder;

fn main() {
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "warn");
    }
    env_logger::init().unwrap();
    match env::args().next().as_ref().map(|x| &x[..]) {
        Some("vagga") => launcher::main(),
        Some("vagga_launcher") => launcher::main(),
        Some("vagga_network") => network::main(),
        Some("vagga_setup_netns") => setup_netns::main(),
        Some("vagga_version") => version::main(),
        Some("vagga_wrapper") => wrapper::main(),
        Some("vagga_build") => builder::main(),
        _ => launcher::main(),
    }
}
