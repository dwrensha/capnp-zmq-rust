use capnp;
use capnp::message::MessageBuilder;
use zmq;
use std;
use rand::Rng;
use capnp_zmq;
use explorers_capnp::observation;
use time;

struct Pixel {
    red : u8,
    green : u8,
    blue : u8
}

impl Copy for Pixel {}

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
        match std::fs::File::open(file) {
            Err(_e) => panic!("could not open"),
            Ok(reader) => {
                let mut buffered = reader; // TODO make this buffered.
                unimplemented!();
                /*
                match buffered.read_line() {
                    Ok(s) => {
                        assert!(s.as_slice().trim() == "P6");
                    }
                    Err(_e) => panic!("premature end of file")
                }
                let (width, height) : (u32, u32) = match buffered.read_line() {
                    Ok(s) => {
                        let dims : Vec<&str> = s.as_slice().split(' ').collect();
                        if dims.len() != 2 { panic!("could not read dimensions") }
                        (::std::str::FromStr::from_str(dims[0].trim()).unwrap(),
                         ::std::str::FromStr::from_str(dims[1].trim()).unwrap())
                    }
                    Err(_e) => { panic!("premature end of file") }
                };
                match buffered.read_line() {
                    Ok(s) => { assert!(s.as_slice().trim() == "255") }
                    Err(_e) => panic!("premature end of file")
                }

                let mut result = Image { width : width, height : height, pixels : Vec::new() };
                for _ in 0..width * height {
                    result.pixels.push(
                        Pixel {
                            red : unimplemented!(), //try!(buffered.read_u8()),
                            green : unimplemented!(), //try!(buffered.read_u8()),
                            blue : unimplemented!(), //try!(buffered.read_u8())
                        });
                }
                return Ok(result);
                 */
            }
        }
    }

    fn get_pixel(&self, x : u32, y : u32) -> Pixel {
        assert!(x < self.width);
        assert!(y < self.height);
        self.pixels[((y * self.width) + x) as usize]
    }

    fn take_measurement(&self, x : f32, y : f32, mut obs : observation::Builder) {
        use std::num::Float;
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
        warning.push_str(rng.gen_ascii_chars().take(8).collect::<String>().as_slice());
        warning.push_str(" ");
        warning.push_str(*rng.choose(&WORDS).unwrap());
        obs.init_diagnostic().set_warning(warning.as_slice());
    }
}

pub fn main () -> Result<(), zmq::Error> {

    let args = std::os::args();
    if args.len() != 3 {
        println!("usage: {:?} explorer [filename]", args.get(0));
        return Ok(());
    }

    let image = Image::load(&std::path::Path::new(args[2].as_slice())).unwrap();

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

        // XXX we need thread::sleep
        //std::io::timer::sleep(std::time::Duration::milliseconds(5));
    }

}
