use anyhow::{Context, Result};
use log::Level::Info;
use log::{debug, error, info, log_enabled};
use std::time::{Duration, SystemTime};
use tokio::io::{
    split, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf,
};
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tokio_serial::SerialStream;
use tokio_serial::{FlowControl, SerialPortType};

extern crate core;
use core::ffi::c_int;

extern "C" {
    fn init_hdlc();
    /*
     * Wraps a PPP packet into an HDLC frame and write it to a buffer.
     *
     * @param[out] frame    The buffer to store the encoded frame.
     * @param[in]  frmsize  The output buffer size.
     * @param[in]  packet   The buffer containing the packet.
     * @param[in]  pktsize  The input packet size.
     * @return              the number of bytes written to the buffer (i.e. the
     *                      HDLC-encoded frame length) or ERR_HDLC_BUFFER_TOO_SMALL
     *                      if the output buffer is too small
     *
     *   ssize_t hdlc_encode(uint8_t *frame, size_t frmsize,
     *       const uint8_t *packet, size_t pktsize)
     */
    fn hdlc_encode(frame: *mut u8, frmsize: usize, packet: *const u8, pktsize: usize) -> isize;

    /*
     * Finds the first frame in a buffer, starting search at start.
     *
     * @param[in]     buffer   The input buffer.
     * @param[in]     bufsize  The input buffer size.
     * @param[in,out] start    Offset of the beginning of the first frame in the buffer.
     * @return                 the length of the first frame or ERR_HDLC_NO_FRAME_FOUND
     *                         if no frame is found.
     *
     *    ssize_t hdlc_find_frame(const uint8_t *buffer, size_t bufsize, off_t *start)
     */
    fn hdlc_find_frame(buffer: *const u8, bufsize: usize, start: *mut u8);

    /*
     * Extracts the first PPP packet found in the input buffer.
     *
     * The frame should be passed without its surrounding Flag Sequence (0x7e) bytes.
     *
     * @param[in]  frame    The buffer containing the encoded frame.
     * @param[in]  frmsize  The input buffer size.
     * @param[out] packet   The buffer to store the decoded packet.
     * @param[in]  pktsize  The output packet buffer size.
     * @return              the number of bytes written to the output packet
     *                      buffer, or < 0 in case of error.
     */
    fn hdlc_decode(frame: *const u8, frmsize: usize, packet: *mut u8, pktsize: usize) -> isize;
}

#[derive(Debug, Clone)]
enum Msg {
    Line(String),
    Buf(Vec<u8>),
}

///////// Custom error type ///////////
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct WtfError;

impl Error for WtfError {}

impl fmt::Display for WtfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WtfError: WTF!")
    }
}
// Note: This is how to make a WtfError:
//     let r: Result<()> = Result::Err(WtfError {}.into());
//
//////////////

fn epoch_seconds() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs())
}

// The PPP start sequence.
const FLAG: u8 = 0x7e;
const CONTROL_ESCAPE: u8 = 0x7d;

async fn writer(mut writer: impl tokio::io::AsyncWrite + Unpin) -> Result<()> {
    //    async fn writer(mut writer: WriteHalf<SerialStream>) -> Result<()> {
    let mut words_of_cato: Vec<u8> = "Carthāgō dēlenda est\n".into();
    let mut words_of_jack: Vec<u8> = "All work and no play makes Jack a dull boy.\n".into();

    let mut msg: Vec<u8> = vec![];
    msg.push(FLAG);
    msg.append(&mut words_of_cato);
    msg.push(FLAG);
    msg.push(FLAG);
    msg.append(&mut words_of_jack);

    writer
        .write(b"Bytes before FLAG should be ignored.")
        .await
        .context("Error on writing")?;

    loop {
        writer.write(&msg).await.context("Error on writing")?;
        sleep(Duration::from_millis(100)).await;
    }
}

async fn line_reader(reader: ReadHalf<SerialStream>, tx: Sender<Msg>) -> Result<()> {
    let buf_reader: BufReader<ReadHalf<SerialStream>> = BufReader::new(reader);

    let mut lines: tokio::io::Lines<BufReader<ReadHalf<SerialStream>>> = buf_reader.lines();
    loop {
        match lines.next_line().await.context("Error on reading line")? {
            Some(line) => {
                //info!("Serial got: {:?}", line);
                tx.send(Msg::Line(line)).await?;
            }
            None => error!("Serial got: empty line."),
        }
    }
}

// PPP framing taken from here:
// http://www.acacia-net.com/wwwcla/protocol/ip_ppp.htm

enum FramerState {
    Frame,
    Escaped,
    Flag,
}

struct Framer {
    frame: Vec<u8>,
    state: FramerState,
}
use std::mem;

impl Framer {
    fn new() -> Self {
        Framer {
            frame: Vec::<u8>::new(),
            state: FramerState::Flag,
        }
    }

    fn find_frame(&mut self, byte: u8) -> Option<Vec<u8>> {
        match self.state {
            FramerState::Flag if byte == FLAG => {
                self.state = FramerState::Frame;
                None
            }
            FramerState::Flag => None,
            FramerState::Frame if byte == FLAG => {
                // Frame is complete, ship it out.
                if !self.frame.is_empty() {
                    let mut new_frame = Vec::<u8>::new();
                    mem::swap(&mut self.frame, &mut new_frame);
                    return Some(new_frame);
                }
                None
            }
            FramerState::Frame if byte == CONTROL_ESCAPE => {
                // Discard the control escape sequence
                self.state = FramerState::Escaped;
                None
            }
            FramerState::Frame => {
                // Collect frame bytes.
                self.frame.push(byte);
                None
            }
            FramerState::Escaped => {
                // Collect escaped frame byte.
                self.frame.push(byte ^ 0x20);
                self.state = FramerState::Frame;
                None
            }
        }
    }
}

async fn frame_reader(
    mut reader: impl tokio::io::AsyncRead + Unpin,
    tx: Sender<Msg>,
) -> Result<()> {
    let mut buf: [u8; 1] = [0];
    let mut framer = Framer::new();
    loop {
        reader.read(&mut buf).await.context("Error on read")?;
        let byte = buf[0];
        if let Some(frame) = framer.find_frame(byte) {
            tx.send(Msg::Buf(frame.clone())).await?;
        }
    }
}

async fn printer(mut rx: Receiver<Msg>) {
    loop {
        let msg: Option<Msg> = rx.recv().await;
        if let Some(msg) = msg {
            match msg {
                Msg::Line(line) => info!("{}", line),
                Msg::Buf(buf) => info!("{:?}", buf),
            }
        }
    }
}

fn open_serial(
    path: String,
    baud_rate: u32,
) -> Result<(ReadHalf<SerialStream>, WriteHalf<SerialStream>)> {
    let port_builder: tokio_serial::SerialPortBuilder =
        tokio_serial::new(path.clone(), baud_rate).flow_control(FlowControl::None);

    let stream: SerialStream =
        SerialStream::open(&port_builder).context(format!("Failed to open serial port {path}"))?;
    Ok(split(stream))
}

async fn test_serial(path: String, baud_rate: u32) -> Result<()> {
    type SerialReader = ReadHalf<SerialStream>;
    type SerialWriter = WriteHalf<SerialStream>;
    type SerialStreamHalves = (SerialReader, SerialWriter);

    println!("Using serial port: {path} at {baud_rate} baud.");

    loop {
        let (tx, rx) = mpsc::channel(32);

        let res = open_serial(path.clone(), baud_rate);
        match res {
            Ok(serial_stream_halves) => {
                let (read_half, write_half) = serial_stream_halves;
                let reader = BufReader::new(read_half);

                select! {
                    val = writer(write_half) => error!("writer completed with: {val:?}"),

                    val = frame_reader(reader, tx) => error!("reader completed with: {val:?}"),

                    _ = printer(rx) => {}
                }
            }
            Err(e) => {
                error!("{:?}", e);
                sleep(Duration::from_millis(1000)).await;
            }
        }
    }
}

use libc::size_t;

pub async fn serial_port_test(first_port: &str, second_port: &str, baud_rate: u32) -> Result<()> {
    println!("!!!!!!!!!!!!!!!!!!!!! UNSAFE !!!!!!!!!!!!!!!!!!!!!!!!!");
    unsafe {
        init_hdlc();
    }

    let mut frame: Vec<u8> = vec![0; 256];
    let packet_in: Vec<u8> = vec![0x40, 0x41, 0x42, 0x7e, 0x44, 0x45, 0x46, 0x47, 0x48];
    let mut packet_out: Vec<u8> = vec![0; 256];

    fn hdlc_encode_ffi(frame: &mut [u8], packet: &[u8]) -> isize {
        let p_frame = frame.as_mut_ptr();
        let p_packet = packet.as_ptr();
        let size = unsafe {
            let size = hdlc_encode(p_frame, frame.len() as size_t, p_packet, packet.len());
            size
        };
        size
    }

    fn hdlc_decode_ffi(frame: &[u8], packet: &mut [u8]) -> isize {
        let p_frame = frame.as_ptr();
        let p_packet = packet.as_mut_ptr();
        let size = unsafe {
            let size = hdlc_decode(p_frame, frame.len() as size_t, p_packet, packet.len());
            size
        };
        size
    }

    let size = hdlc_encode_ffi(&mut frame, &packet_in) as usize;
    println!("!!!!!!!!!!!! {size} !!!!!!!!!!!!!!!!!");
    println!("{:x?}", frame);

    let size = hdlc_decode_ffi(&frame, &mut packet_out[0..size]);
    println!("!!!!!!!!!!!! {size} !!!!!!!!!!!!!!!!!");
    println!("{:x?}", packet_out);

    select! {
        res = test_serial(first_port.to_string(), baud_rate) => {
            debug!("{:?}", res);
            res
        }
        res = test_serial(second_port.to_string(), baud_rate) => {
            debug!("{:?}", res);
            res
        }
    }
}

pub fn list_serial_ports() -> Result<()> {
    println!("Available serial ports:");

    let ports = tokio_serial::available_ports()?;
    for port in ports {
        let port_name = &port.port_name;
        if port_name.starts_with("/dev/tty.usb") || port_name.starts_with("dev/ttyUSB") {
            println!("Name: {}", port_name);

            if log_enabled!(Info) {
                if let SerialPortType::UsbPort(info) = &port.port_type {
                    if let Some(man) = &info.manufacturer {
                        println!("Manufacturer: {}", man)
                    }
                    if let Some(prod) = &info.product {
                        println!("Product name: {}", prod);
                    }
                    if let Some(sn) = &info.serial_number {
                        println!("Serial number: {}", sn);
                    }
                    println!("Vendor ID {}", info.vid);
                    println!("Product ID {}", info.pid);
                }
            };
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //use crate::serial_port_test::epoch_seconds;
    use crate::serial_port_test::*;
    use std::time::SystemTime;

    #[test]
    fn test_epoch_seconds() {
        let now: u64 = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert_eq!(epoch_seconds().unwrap(), now);
    }

    #[test]
    fn test_find_frame() {
        let message_1: Vec<u8> = vec![0x01, 0x02, 0x03, 0x05];
        let message_2: Vec<u8> = vec![0x06, 0x07, 0x08, 0x09];
        let message_3_in: Vec<u8> = vec![0x0a, 0x0b, 0x7d, 0x5e, 0x0d];
        let message_3_out: Vec<u8> = vec![0x0a, 0x0b, 0x7e, 0x0d];
        let message_4_in: Vec<u8> = vec![0x10, 0x7d, 0x5d, 0x12, 0x13];
        let message_4_out: Vec<u8> = vec![0x10, 0x7d, 0x12, 0x13];

        let mut messages = vec![0x55u8, 0x55u8, 0x55u8, 0x55u8, 0x55u8, 0x55u8].to_vec();
        messages.push(0x7e);
        messages.append(&mut message_1.clone());
        messages.push(0x7e);
        messages.push(0x7e);
        messages.append(&mut message_2.clone());
        messages.push(0x7e);
        messages.append(&mut message_3_in.clone());
        messages.push(0x7e);
        messages.append(&mut message_4_in.clone());
        messages.push(0x7e);
        messages.append(&mut [0xaau8, 0xaau8, 0xaau8, 0xaau8, 0xaau8, 0xaau8].to_vec());

        println!("{:x?}", messages);

        let mut framer = Framer::new();
        let mut frames: Vec<Vec<u8>> = vec![];
        for byte in messages {
            if let Some(x) = framer.find_frame(byte) {
                frames.push(x);
                println!("{:x?}", frames);
            }
        }
        assert_eq!(frames[0], message_1);
        assert_eq!(frames[1], message_2);
        assert_eq!(frames[2], message_3_out);
        assert_eq!(frames[3], message_4_out);
    }
}
