extern crate capnpc;

fn main() {
    ::capnpc::compile(&::std::path::Path::new("."),
                      &[&::std::path::Path::new("explorers.capnp")]).unwrap();
}
