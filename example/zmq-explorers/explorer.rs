use capnp;
use capnp::message::MessageBuilder;
use zmq;
use std;
use rand::Rng;
use capnp_zmq;
use explorers_capnp::observation;
use time;

#[derive(Clone, Copy)]
struct Pixel {
    red : u8,
    green : u8,
    blue : u8
}

fn fudge(x : u8) -> u8 {
    let error = ::rand::thread_rng().gen_range::<i16>(-60, 60);
    let y = x as i16 + error;
    if y < 0 { return 0; }
    if y > 255 { return 255; }
    return y as u8;
}

struct Image {
    width : u32,
    height : u32,
    pixels : Vec<Pixel>
}

impl Image {

    // quick and dirty parsing of a PPM image
    fn load(file : &std::path::Path) -> std::io::Result<Image> {
        use std::io::{BufRead, Read};
        let file = try!(std::fs::File::open(file));
        let mut buffered = ::std::io::BufReader::new(file);
        let mut line = String::new();
        try!(buffered.read_line(&mut line));
        assert!(line.trim() == "P6");
        line.clear();

        try!(buffered.read_line(&mut line));
        let (width, height) : (u32, u32) = {
            let dims : Vec<&str> = line.split(' ').collect();
            assert!(dims.len() == 2, "could not read dimensions");
            (::std::str::FromStr::from_str(dims[0].trim()).unwrap(),
             ::std::str::FromStr::from_str(dims[1].trim()).unwrap())
        };
        line.clear();

        try!(buffered.read_line(&mut line));
        assert!(line.trim() == "255");
        line.clear();

        let mut result = Image { width : width, height : height, pixels : Vec::new() };
        let mut bytes = buffered.bytes();
        for _ in 0..width * height {
            result.pixels.push(
                Pixel {
                    red : try!(bytes.next().expect("error reading red value")),
                    green : try!(bytes.next().expect("error reading green value")),
                    blue : try!(bytes.next().expect("error reading blue value")),
                });
        }
        return Ok(result);
    }

    fn get_pixel(&self, x : u32, y : u32) -> Pixel {
        assert!(x < self.width);
        assert!(y < self.height);
        self.pixels[((y * self.width) + x) as usize]
    }

    fn take_measurement(&self, x : f32, y : f32, mut obs : observation::Builder) {
        assert!(x >= 0.0); assert!(y >= 0.0); assert!(x < 1.0); assert!(y < 1.0);

        obs.set_timestamp(time::now().to_timespec().sec);
        obs.set_x(x);
        obs.set_y(y);

        let pixel = self.get_pixel((x * self.width as f32).floor() as u32,
                                   (y * self.height as f32).floor() as u32);

        obs.set_red(fudge(pixel.red));
        obs.set_green(fudge(pixel.green));
        obs.set_blue(fudge(pixel.blue));

        add_diagnostic(obs);
    }
}

static WORDS : [&'static str; 20] = [
   "syntax", "personality", "rhymist", "shopwalker", "gooseskin", "overtask",
    "churme", "heathen", "economiser", "radium", "attainable", "nonius", "knaggy",
    "inframedian", "tamperer", "disentitle", "horary", "morsure", "bonnaz", "alien",
];

// With small probability, add a gibberish warning to the observation.
fn add_diagnostic<'a>(obs : observation::Builder<'a>) {
    let mut rng = ::rand::thread_rng();
    if rng.gen_range::<u16>(0, 3000) < 2 {
        let mut warning = String::new();
        warning.push_str(*rng.choose(&WORDS).unwrap());
        warning.push_str(" ");
        warning.push_str(&rng.gen_ascii_chars().take(8).collect::<String>());
        warning.push_str(" ");
        warning.push_str(*rng.choose(&WORDS).unwrap());
        obs.init_diagnostic().set_warning(&warning);
    }
}

pub fn main () -> Result<(), zmq::Error> {

    let args : Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {:?} explorer [filename]", args.get(0));
        return Ok(());
    }

    let image = Image::load(&std::path::Path::new(&args[2])).unwrap();

    let mut context = zmq::Context::new();
    let mut publisher = try!(context.socket(zmq::PUB));
    try!(publisher.connect("tcp://localhost:5555"));

    let mut rng = ::rand::thread_rng();
    let mut x = rng.gen_range::<f32>(0.0, 1.0);
    let mut y = rng.gen_range::<f32>(0.0, 1.0);

    loop {
        x += rng.gen_range::<f32>(-0.01, 0.01);
        y += rng.gen_range::<f32>(-0.01, 0.01);

        if x >= 1.0 { x -= 1.0 }
        if y >= 1.0 { y -= 1.0 }
        if x < 0.0 { x += 1.0 }
        if y < 0.0 { y += 1.0 }

        let mut message = capnp::message::MallocMessageBuilder::new_default();
        {
            let obs = message.init_root::<observation::Builder>();
            image.take_measurement(x, y, obs);
        }
        try!(capnp_zmq::send(&mut publisher, &mut message));

        // TODO switch to thread::sleep once it exists.
        unsafe {
            let req = ::libc::types::os::common::posix01::timespec {
                tv_sec : 0,
                tv_nsec : 5000000, // 5 milliseconds
            };
            ::libc::funcs::posix88::unistd::nanosleep(&req, ::std::ptr::null_mut());
        }
    }

}
