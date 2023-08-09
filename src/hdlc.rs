// The PPP start sequence.
pub const FLAG: u8 = 0x7e;
pub const CONTROL_ESCAPE: u8 = 0x7d;

enum FramerState {
    Frame,
    Escaped,
    Flag,
}

pub struct Framer {
    frame: Vec<u8>,
    state: FramerState,
}
use std::mem;

impl Framer {
    pub fn new() -> Self {
        Framer {
            frame: Vec::<u8>::new(),
            state: FramerState::Flag,
        }
    }

    pub fn find_frame(&mut self, byte: u8) -> Option<Vec<u8>> {
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

#[cfg(test)]
mod tests {
    //use crate::serial_port_test::epoch_seconds;
    use super::*;

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
