use capnp;
use zmq;
use capnp_zmq;
use std;
use time;
use explorers_capnp::grid;

enum OutputMode {
    Colors,
    Confidence
}

fn write_ppm(path : &std::path::Path, grid : grid::Reader, mode : OutputMode) -> std::io::Result<()> {
    use std::io::Write;
    use std::num::Float;
    println!("writing to file: {:?}", path);
    let writer = try!(std::fs::File::create(path));
    let mut buffered = ::std::io::BufWriter::new(writer);
    try!(buffered.write_all(b"P6\n"));

    let cells = grid.get_cells().unwrap();
    let width = cells.len();
    assert!(width > 0);
    let height = cells.get(0).unwrap().len();

    try!(buffered.write(format!("{} {}\n", width, height).as_bytes()));
    try!(buffered.write(b"255\n"));

    for x in 0..width {
        assert!(cells.get(x).unwrap().len() == height);
    }

    // Urgh. The cells are stored as columns, but ppm format wants the pixels in row-major order. So
    // we end up dereferencing pointers more than we'd like here. This is why We see the traversal
    // limit to the maximum in the main loop.
    for y in 0..height {
        for x in 0..width {
            let cell = cells.get(x).unwrap().get(y);

            match mode {
                OutputMode::Colors => {
                    try!(buffered.write_all(&[(cell.get_mean_red()).floor() as u8]));
                    try!(buffered.write_all(&[cell.get_mean_green().floor() as u8]));
                    try!(buffered.write_all(&[cell.get_mean_blue().floor() as u8]));
                }
                OutputMode::Confidence => {
                    let mut age = time::now().to_timespec().sec - cell.get_latest_timestamp();
                    if age < 0 { age = 0 };
                    age *= 25;
                    if age > 255 { age = 255 };
                    age = 255 - age;

                    let mut n = cell.get_number_of_updates();
                    n *= 10;
                    if n > 255 { n = 255 };

                    try!(buffered.write_all(&[0]));

                    try!(buffered.write_all(&[n as u8]));

                    try!(buffered.write_all(&[age as u8]));
                }
            }
        }
    }
    try!(buffered.flush());
    Ok(())
}

pub fn main() -> Result<(), zmq::Error> {
    use capnp::message::MessageReader;

    let mut context = zmq::Context::new();
    let mut requester = try!(context.socket(zmq::REQ));

    try!(requester.connect("tcp://localhost:5556"));

    let mut c : u32 = 0;

    loop {
        try!(requester.send(&[], 0));

        let frames = try!(capnp_zmq::recv(&mut requester));
        let segments = capnp_zmq::frames_to_segments(&frames);
        let reader = capnp::message::SegmentArrayMessageReader::new(
            &segments,
            *capnp::message::ReaderOptions::new().traversal_limit_in_words(0xffffffffffffffff));

        let grid = reader.get_root::<grid::Reader>().unwrap();

        println!("timestamp: {}", grid.get_latest_timestamp());
        println!("word count: {}", grid.total_size().unwrap().word_count);

        write_ppm(&std::path::Path::new(&format!("colors{:05}.ppm", c)),
                  grid, OutputMode::Colors).unwrap();

        write_ppm(&std::path::Path::new(&format!("conf{:05}.ppm", c)),
                  grid, OutputMode::Confidence).unwrap();

        c += 1;

        // Sleep for five seconds.
        unsafe { ::libc::funcs::posix88::unistd::sleep(5); }
    }
}
