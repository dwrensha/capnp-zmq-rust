use capnp;
use zmq;
use capnp_zmq;

static GRID_WIDTH : u32 = 120;
static GRID_HEIGHT : u32 = 120;

pub fn main() -> Result<(), zmq::Error> {
    use explorers_capnp::{observation, grid};
    use capnp::message::{MessageReader, MessageBuilder};

    let mut context = zmq::Context::new();
    let mut subscriber = try!(context.socket(zmq::SUB));
    let mut responder = try!(context.socket(zmq::REP));

    try!(subscriber.bind("tcp://*:5555"));
    try!(subscriber.set_subscribe(&[]));
    try!(responder.bind("tcp://*:5556"));

    let mut poll_items = [responder.as_poll_item(zmq::POLLIN),
                          subscriber.as_poll_item(zmq::POLLIN)];

    let mut message = capnp::message::MallocMessageBuilder::new_default();

    // We hold onto a single message builder, modify it as
    // updates come in, and send it out when requested.
    // *Caution*: due to Cap'n Proto's arena allocation
    // scheme, this usage pattern could waste memory if these
    // updates caused allocations in the message. Fortunately,
    // the updates only change the values of existing numeric
    // fields, so no allocation occurs. If that were not the
    // case, we could occasionally garbage collect by copying
    // to a fresh message builder.

    {
        let grid = message.init_root::<grid::Builder>();
        let mut cells = grid.init_cells(GRID_WIDTH);
        for ii in 0..cells.len() {
            cells.borrow().init(ii, GRID_HEIGHT);
        }
    }

    loop {

        try!(zmq::poll(&mut poll_items, -1));

        if (poll_items[0].get_revents() & zmq::POLLIN) != 0 {

            try!(responder.recv_msg(0));
            try!(capnp_zmq::send(&mut responder, & mut message));

        } else if (poll_items[1].get_revents() & zmq::POLLIN) != 0 {

            // there's a new observation waiting for us

            let frames = try!(capnp_zmq::recv(&mut subscriber));
            let segments = capnp_zmq::frames_to_segments(&frames);
            let reader = capnp::message::SegmentArrayMessageReader::new(
                &segments,
                capnp::message::DEFAULT_READER_OPTIONS);
            let obs = reader.get_root::<observation::Reader>().unwrap();

            if obs.get_x() >= 1.0 || obs.get_x() < 0.0 ||
                obs.get_y() >= 1.0 || obs.get_y() < 0.0 {
                println!("out of range");
                continue;
            }

            match obs.get_diagnostic().which() {
                Ok(observation::diagnostic::Ok(())) => {}
                Ok(observation::diagnostic::Warning(s)) => {
                    println!("received diagnostic: {}", s.unwrap());
                }
                Err(_) => {}
            }

            let x = (obs.get_x() * GRID_WIDTH as f32).floor() as u32;
            let y = (obs.get_y() * GRID_HEIGHT as f32).floor() as u32;

            {
                let mut grid = message.get_root::<grid::Builder>().unwrap();
                grid.set_latest_timestamp(obs.get_timestamp());
                let number_of_updates = grid.borrow().get_number_of_updates();
                grid.set_number_of_updates(number_of_updates + 1);
                let cells = grid.get_cells().unwrap();


                let mut cell = cells.get(x).unwrap().get(y);
                cell.set_latest_timestamp(obs.get_timestamp());

                let n = cell.borrow().get_number_of_updates();

                let mean_red = cell.borrow().get_mean_red();
                cell.set_mean_red((n as f32 * mean_red + obs.get_red() as f32) / (n + 1) as f32);
                let mean_green = cell.borrow().get_mean_green();
                cell.set_mean_green(
                    (n as f32 * mean_green + obs.get_green() as f32) / (n + 1) as f32);

                let mean_blue = cell.borrow().get_mean_blue();
                cell.set_mean_blue((n as f32 * mean_blue + obs.get_blue() as f32) / (n + 1) as f32);
                cell.set_number_of_updates(n + 1);
            }
        }
    }
}
