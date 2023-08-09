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

use crate::hdlc;
use crate::hdlc::*;

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

async fn frame_reader(
    mut reader: impl tokio::io::AsyncRead + Unpin,
    tx: Sender<Msg>,
) -> Result<()> {
    let mut buf: [u8; 1] = [0];
    let mut framer = hdlc::Framer::new();
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
}
