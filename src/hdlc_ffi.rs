extern crate core;
use anyhow::{Context, Result};
use core::ffi::c_int;

use libc::{off_t, size_t, ssize_t};

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
    fn hdlc_encode(frame: *mut u8, frmsize: size_t, packet: *const u8, pktsize: size_t) -> ssize_t;

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
    fn hdlc_find_frame(buffer: *const u8, bufsize: size_t, start: *mut off_t) -> ssize_t;

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
    fn hdlc_decode(frame: *const u8, frmsize: size_t, packet: *mut u8, pktsize: size_t) -> ssize_t;
}

pub fn init_hdlc_ffi() {
    unsafe {
        init_hdlc();
    }
}

pub fn hdlc_encode_ffi(frame: &mut [u8], packet: &[u8]) -> Result<usize, isize> {
    let p_frame = frame.as_mut_ptr();
    let p_packet = packet.as_ptr();
    let res = unsafe { hdlc_encode(p_frame, frame.len() as size_t, p_packet, packet.len()) };
    if res < 0 {
        Err(res)
    } else {
        Ok(res as usize)
    }
}

pub fn hdlc_find_frame_ffi(buffer: &[u8]) -> Result<&[u8], isize> {
    let mut offset: i64 = 0;
    let p_offset = &mut offset as *mut i64;

    let p_buffer = buffer.as_ptr();
    let res = unsafe { hdlc_find_frame(p_buffer, buffer.len(), p_offset) };
    if res < 0 {
        Err(res)
    } else {
        Ok(&buffer[offset as usize..offset as usize + res as usize])
    }
}

pub fn hdlc_decode_ffi(frame: &[u8], packet: &mut [u8]) -> Result<usize, isize> {
    let p_frame = frame.as_ptr();
    let p_packet = packet.as_mut_ptr();
    let res = unsafe { hdlc_decode(p_frame, frame.len() as size_t, p_packet, packet.len()) };
    if res < 0 {
        Err(res)
    } else {
        Ok(res as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hdlc_encode_ffi() {
        init_hdlc_ffi();

        let data: Vec<u8> = vec![0x40, 0x41, 0x42, 0x7e, 0x44, 0x45, 0x46, 0x47, 0x48];
        let mut encoded: Vec<u8> = vec![0; 256];

        let size = hdlc_encode_ffi(&mut encoded, &data).unwrap();

        let res = hdlc_find_frame_ffi(&encoded[0..size]);
        let frame = match res {
            Ok(frame) => frame,
            Err(e) => {
                panic!();
            }
        };

        let mut packet_out: Vec<u8> = vec![0; 256];
        let size = hdlc_decode_ffi(&frame, &mut packet_out[0..size]).unwrap();
        assert_eq!(packet_out[0..size], data);
    }
    #[test]
    fn test_hdlc_find_frame_ffi() {
        init_hdlc_ffi();

        let packet_in: Vec<u8> = vec![0x40, 0x41, 0x42, 0x44, 0x45, 0x46, 0x47, 0x48];
        let res = hdlc_find_frame_ffi(&packet_in);
        match res {
            Ok(_) => panic!(),
            Err(e) => assert_eq!(e, -2),
        }

        let packet_in: Vec<u8> = vec![
            0x55, 0x55, 0x55, 0x7e, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x7e,
            0xaa, 0xaa, 0xaa,
        ];
        let res = hdlc_find_frame_ffi(&packet_in);
        match res {
            Ok(frame) => {
                assert_eq!(frame, &packet_in[4..=12])
            }
            Err(e) => panic!(),
        }
    }
}
