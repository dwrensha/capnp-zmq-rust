extern crate capnp;
extern crate zmq;

fn slice_cast<'a, T, V>(s : &'a [T]) -> &'a [V] {
    unsafe {
        ::std::slice::from_raw_parts(::std::mem::transmute(s.as_ptr()),
                                     s.len() * std::mem::size_of::<T>() / std::mem::size_of::<V>())
    }
}

pub fn frames_to_segments<'a>(frames : &'a [zmq::Message] ) -> Vec<&'a [::capnp::Word]> {

    let mut result : Vec<&'a [::capnp::Word]> = Vec::new();
    for frame in frames.iter() {
        unsafe {
            let slice = frame.with_bytes(|v|
                    std::slice::from_raw_parts(v.as_ptr(), v.len() / 8));

            // TODO check whether bytes are aligned on a word boundary.
            // If not, copy them into a new buffer. Who will own that buffer?

            result.push(std::mem::transmute(slice));
        }
    }

    return result;
}

pub fn recv(socket : &mut zmq::Socket) -> Result<Vec<zmq::Message>, zmq::Error> {
    let mut frames = Vec::new();
    loop {
        match socket.recv_msg(0) {
            Ok(m) => frames.push(m),
            Err(e) => return Err(e)
        }
        match socket.get_rcvmore() {
            Ok(true) => (),
            Ok(false) => return Ok(frames),
            Err(e) => return Err(e)
        }
    }
}

pub fn send<U: ::capnp::message::MessageBuilder>(socket : &mut zmq::Socket,
                                                 message : &mut U) -> Result<(), zmq::Error> {

    let segments = message.get_segments_for_output();
    for ii in 0..segments.len() {
        let flags = if ii == segments.len() - 1 { 0 } else { zmq::SNDMORE };
        match socket.send(slice_cast(segments[ii]), flags) {
            Ok(_) => {}
            Err(_) => {panic!();} // XXX
        }
    }
    Ok(())
}
