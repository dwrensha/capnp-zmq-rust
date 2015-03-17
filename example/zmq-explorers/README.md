# zmq-explorers: a toy data pipeline

This example illustrates one way to use capnproto-rust with ZeroMQ.

## Prerequisites

Install [libzmq](http://zeromq.org/area:download) and [Cap'n Proto](https://capnproto.org/install.html).

Note that you may run into trouble if libzmq is installed in the same directory
as rustc, as shown in [issue 11195](https://github.com/mozilla/rust/issues/11195).


## Running

In three separate terminals:

```
$ cargo run collector
$ cargo run explorer ~/Desktop/rust_logo.ppm
$ cargo run viewer
```
