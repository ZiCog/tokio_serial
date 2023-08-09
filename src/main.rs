use clap::Parser;
use log::{debug, error};

mod protobuf_experiment;

mod serial_port_test;
use serial_port_test::{list_serial_ports, serial_port_test};

mod crc;
mod hdlc;
mod hdlc_ffi;

/// Simple program to test a serial ports
#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    /// Baud rate
    #[arg(short, long, default_value_t = 921600)]
    baud_rate: u32,

    /// First serial port
    #[arg(short, long, default_value = "/dev/ttyUSB0")]
    first_port: String,

    /// Second Serial port
    #[arg(short, long, default_value = "/dev/ttyUSB1")]
    second_port: String,

    /// List serial ports
    #[arg(short, long, default_value_t = false)]
    list: bool,

    /// Use protobuf messaging.
    #[arg(short, long, default_value_t = false)]
    protobuf: bool,
}

#[tokio::main]
async fn main() {
    let ix: isize = -100;
    let ux = ix as usize;
    println!("!!!!!!!!! {ux}");

    env_logger::init();

    let args: Args = Args::parse();

    conveqs_banner();

    if args.protobuf {
        protobuf_experiment::protobuf_experiment();
    }
    if args.list {
        if let Err(e) = list_serial_ports() {
            error!("{e:?}");
        }
    } else {
        let res = serial_port_test(&args.first_port, &args.second_port, args.baud_rate).await;
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
