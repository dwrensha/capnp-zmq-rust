#![crate_name="zmq-explorers"]
#![crate_type = "bin"]


extern crate capnp;
extern crate capnp_zmq;
extern crate libc;
extern crate rand;
extern crate time;
extern crate zmq;

pub mod explorers_capnp {
  include!(concat!(env!("OUT_DIR"), "/explorers_capnp.rs"));
}

pub mod explorer;
pub mod collector;
pub mod viewer;


fn usage(s : &str) {
    println!("usage: {} [explorer|collector|viewer]", s);
    std::os::set_exit_status(1);
}

pub fn main() {

    let args = std::os::args();

    if args.len() < 2 {
        usage(args[0].as_slice());
        return;
    }

    let result = match args[1].as_slice() {
        "explorer" => explorer::main(),
        "collector" => collector::main(),
        "viewer" => viewer::main(),
        _ => { usage(args[0].as_slice()); Ok(()) }
    };

    match result {
        Ok(()) => {}
        Err(e) => {
            std::os::set_exit_status(1);
            println!("{}", e)
        },
    }

}
