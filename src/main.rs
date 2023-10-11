use clap::Parser;
use log::{debug, error};

mod protobuf_experiment;

mod serial_port_test;
use serial_port_test::{list_serial_ports, serial_port_test};

mod crc;
mod gray_code;
mod hdlc;
mod hdlc_ffi;

/// Simple program to test a serial ports
#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    /// Baud rate
    #[arg(short, long, default_value_t = 921600)]
    baud_rate: u32,

    /// Serial port
    #[arg(short, long, default_value = "/dev/ttyUSB0")]
    port: String,

    /// List serial ports
    #[arg(short, long, default_value_t = false)]
    list: bool,
}

use crc::*;
use gray_code::*;
use hdlc::*;
use hdlc_ffi::*;

use std::ffi::CString;
use std::os::raw::c_char;

fn test_protobuf() {
    protobuf_experiment::protobuf_experiment();
}

fn test_gray_code() {
    // An exerise in single track gray codes.
    single_track_gray_code();
}

fn test_hdlc() {
    // Build a "message" containing all possible byte values.
    let mut data: Vec<u8> = vec![];
    for byte in 0x00u8..=0xFFu8 {
        data.push(byte);
    }
    println!("Data in: {:x?}", data);

    init_hdlc_ffi();

    let encoded = hdlc_encode_ffi(&data).unwrap();
    println!("Encoded: {:x?}", encoded);

    let mut framer = Framer::new();
    for byte in encoded {
        if let Some(frame) = framer.find_frame(byte) {
            println!("Data out: {:x?}", &frame);

            let crc = crc(0xffff, &frame);
            println!("CRC: {:x?}", crc);
            assert_eq!(crc, 0xf0b8);
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args: Args = Args::parse();

    conveqs_banner();

    if args.list {
        if let Err(e) = list_serial_ports() {
            error!("{e:?}");
        }
    } else {
        let res = serial_port_test(&args.port, args.baud_rate).await;
        error!("serial_port_test failed with: {:?}", res);
    }
}

use colored::Colorize;
fn conveqs_banner() {
    // Doom font (tweaked) from:  https://patorjk.com/software/taag/#p=display&f=Graffiti&t=Conveqs
    const BANNER_TOP: &str = r"         _____                                     _____
        /  __ \                                   |  _  |
        | /  \/ ___  _ ____   _____  __ _ ___     | | | |_   _
        | |    / _ \| '_ \ \ / / _ \/ _` / __|    | | | | | | |";

    const BANNER_BOT: &str = r"        | \__/\ (_) | | | \ V /  __/ (_| \__ \    \ \_/ / |_| |
         \____/\___/|_| |_|\_/ \___|\__, |___/     \___/ \__, |
                                       | |                __/ |
                                        \|                \__/
";
    let banner: String = format!(
        "{}{}{}{}",
        "\n",
        BANNER_TOP.blue(),
        "\n",
        BANNER_BOT.yellow()
    );
    debug!("{}", banner);
}
