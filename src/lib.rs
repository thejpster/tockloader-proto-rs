//! Implements the Tockloader protocol.
//!
//! TockOS applications are loaded with `tockloader`.
//! This speaks to the TockOS bootloader using a specific
//! protocol. This crate implements that protocol so
//! that you can write future tockloader compatible bootloaders
//! in Rust!
//#![no_std]

const ESCAPE_CHAR: u8 = 0xFC;

const CMD_PING: u8 = 0x01;
const CMD_INFO: u8 = 0x03;
const CMD_ID: u8 = 0x04;
const CMD_RESET: u8 = 0x05;
const CMD_EPAGE: u8 = 0x06;
const CMD_WPAGE: u8 = 0x07;
const CMD_XEBLOCK: u8 = 0x08;
const CMD_XWPAGE: u8 = 0x09;
const CMD_CRCRX: u8 = 0x10;
const CMD_RRANGE: u8 = 0x11;
const CMD_XRRANGE: u8 = 0x12;
const CMD_SATTR: u8 = 0x13;
const CMD_GATTR: u8 = 0x14;
const CMD_CRCIF: u8 = 0x15;
const CMD_CRCEF: u8 = 0x16;
const CMD_XEPAGE: u8 = 0x17;
const CMD_XFINIT: u8 = 0x18;
const CMD_CLKOUT: u8 = 0x19;
const CMD_WUSER: u8 = 0x20;
const CMD_CHANGE_BAUD: u8 = 0x21;

// const RES_OVERFLOW: u8    = 0x10;
const RES_PONG: u8 = 0x11;
const RES_BADADDR: u8 = 0x12;
// const RES_INTERROR: u8    = 0x13;
// const RES_BADARGS: u8     = 0x14;
const RES_OK: u8 = 0x15;
const RES_UNKNOWN: u8 = 0x16;
// const RES_XFTIMEOUT: u8   = 0x17;
// const RES_XFEPE: u8       = 0x18;
// const RES_CRCRX: u8       = 0x19;
// const RES_RRANGE: u8      = 0x20;
// const RES_XRRANGE: u8     = 0x21;
// const RES_GATTR: u8       = 0x22;
// const RES_CRCIF: u8       = 0x23;
// const RES_CRCXF: u8       = 0x24;
// const RES_INFO: u8        = 0x25;
// const RES_CHANGE_BAUD_FAIL: u8 = 0x26;

enum DecoderState {
    Loading,
    Escape,
}

#[derive(Debug)]
pub enum BaudMode {
    Set, // 0x01
    Verify, // 0x02
}

/// Commands supported by the protocol. A bootloader will decode these and a
/// flash tool will encode them.
#[derive(Debug)]
pub enum Command<'a> {
    /// Send a PING to the bootloader. It will drop its hp buffer and send
    /// back a PONG.
    Ping,
    /// Get info about the bootloader. The result is one byte of length, plus
    /// length bytes of string, followed by 192-length zeroes.
    Info,
    /// Get the Unique ID. Result is 8 bytes of unique ID.
    Id,
    /// Reset all TX and RX buffers.
    Reset,
    /// Erase a page. The RX buffer should contain the address of the start of
    /// the 512 byte page. Any non-page-aligned addresses will result in
    /// RES_BADADDR. This command is not required before writing a page, it is
    /// just an optimisation. It is particularly quick for already empty pages.
    ErasePage { address: u32 },
    /// Write a page in internal flash. The RX buffer should contain the 4
    /// byte address of the start of the page, followed by 512 bytes of page.
    WritePage { address: u32, data: &'a [u8] },
    /// Erase a block of pages in ex flash. The RX buffer should contain the
    /// address of the start of the block. Each block is 8 pages, so 2048
    /// bytes.
    EraseExBlock { address: u32 },
    /// Write a page to ex flash. The RX buffer should contain the address of
    /// the start of the 256 byte page, followed by 256 bytes of page.
    WriteExPage { address: u32, data: &'a [u8] },
    /// Get the length and CRC of the RX buffer. The response is two bytes of
    /// little endian length, followed by 4 bytes of crc32.
    CrcRxBuffer,
    /// Read a range from internal flash. The RX buffer should contain a 4
    /// byte address followed by 2 bytes of length. The response will be
    /// length bytes long.
    ReadRange { address: u32, length: u16 },
    /// Read a range from external flash. The RX buffer should contain a 4
    /// byte address followed by 2 bytes of length. The response will be
    /// length bytes long.
    ExReadRange { address: u32, length: u16 },
    /// Write a payload attribute. The RX buffer should contain a one byte
    /// index, 8 bytes of key (null padded), one byte of value length, and
    /// valuelength value bytes. valuelength must be less than or equal to 55.
    /// The value may contain nulls.
    ///
    /// The attribute index must be less than 16.
    SetAttr {
        index: u8,
        key: [u8; 8],
        value: &'a [u8],
    },
    /// Get a payload attribute. The RX buffer should contain a 1 byte index.
    /// The result is 8 bytes of key, 1 byte of value length, and 55 bytes of
    /// potential value. You must discard 55-valuelength bytes from the end
    /// yourself.
    GetAttr { index: u8 },
    /// Get the CRC of a range of internal flash. The RX buffer should contain
    /// a four byte address and a four byte range. The result will be a four
    /// byte crc32.
    CrcIntFlash { address: u32, length: u32 },
    /// Get the CRC of a range of external flash. The RX buffer should contain
    /// a four byte address and a four byte range. The result will be a four
    /// byte crc32.
    CrcExFlash { address: u32, length: u32 },
    /// Erase a page in external flash. The RX buffer should contain a 4 byte
    /// address pointing to the start of the 256 byte page.
    EraseExPage { address: u32 },
    /// Initialise the external flash chip. This sets the page size to 256b.
    ExFlashInit,
    /// Go into an infinite loop with the 32khz clock present on pin PA19
    /// (GP6) this is used for clock calibration.
    ClockOut,
    /// Write the flash user pages (first 4 bytes is first page, second 4
    /// bytes is second page, little endian).
    WriteFlashUserPages { page1: u32, page2: u32 },
    /// Change the baud rate of the bootloader. The first byte is 0x01 to set
    /// a new baud rate. The next 4 bytes are the new baud rate. To allow the
    /// bootloader to verify that the new baud rate works, the host must call
    /// this command again with the first byte of 0x02 and the next 4 bytes of
    /// the new baud rate. If the next command does not match this, the
    /// bootloader will revert to the old baud rate.
    ChangeBaud { mode: BaudMode, baud: u32 },

    /// This is not seen on the wire but may be returned if we don't
    /// understand something we did receive on the wire.
    Unknown,
}

/// Reponses supported by the protocol. A bootloader will encode these
/// and a flash tool will decode them.
#[derive(Debug)]
pub enum Response<'a> {
    Overflow, // RES_OVERFLOW
    Pong, // RES_PONG
    BadAddress, // RES_BADADDR
    InternalError, // RES_INTERROR
    BadArguments, // RES_BADARGS
    Ok, // RES_OK
    Unknown, // RES_UNKNOWN
    ExFlashTimeout, // RES_XFTIMEOUT
    ExFlashPageError, // RES_XFEPE ??
    CrcRx, // RES_CRCRX
    ReadRange { data: &'a [u8] }, // RES_RRANGE
    ExReadRange, // RES_XRRANGE
    GetAttr, // RES_GATTR
    CrcIntFlash, // RES_CRCIF
    CrcExFlash, // RES_CRCXF
    Info, // RES_INFO
    ChangeBaudFail, // RES_CHANGE_BAUD_FAIL
}

/// The `ComandDecoder` takes bytes and gives you `Command`s.
pub struct CommandDecoder {
    state: DecoderState,
    buffer: [u8; 520],
    count: usize,
}

/// The `ResponseDecoder` takes bytes and gives you `Responses`s.
pub struct ResponseDecoder {
    state: DecoderState,
    buffer: [u8; 520],
    count: usize,
}

/// The `CommandEncoder` takes a `Command` and gives you bytes.
pub struct CommandEncoder<'a> {
    command: &'a Command<'a>,
    count: usize,
}

/// The `ResponseEncoder` takes a `Response` and gives you bytes.
pub struct ResponseEncoder<'a> {
    response: &'a Response<'a>,
    count: usize,
}

impl CommandDecoder {
    /// Create a new `CommandDecoder`.
    ///
    /// The decoder is fed bytes with the `receive` method.
    pub fn new() -> CommandDecoder {
        CommandDecoder {
            state: DecoderState::Loading,
            buffer: [0u8; 520],
            count: 0,
        }
    }

    /// Empty the RX buffer.
    pub fn reset(&mut self) {
        self.count = 0;
    }

    /// Process incoming bytes.
    ///
    /// The decoder is fed bytes with the `receive` method. If not enough
    /// bytes have been seen, this function returns `None`. Once enough bytes
    /// have been seen, it returns `Some(Command)` containing the
    /// decoded Command (or Response).
    pub fn receive(&mut self, ch: u8) -> Option<Command> {
        match self.state {
            DecoderState::Loading => self.handle_loading(ch),
            DecoderState::Escape => self.handle_escape(ch),
        }
    }

    fn load_char(&mut self, ch: u8) {
        if self.count < self.buffer.len() {
            self.buffer[self.count] = ch;
            self.count = self.count + 1;
        }
    }

    fn handle_loading(&mut self, ch: u8) -> Option<Command> {
        if ch == ESCAPE_CHAR {
            self.state = DecoderState::Escape;
        } else {
            self.load_char(ch);
        }
        None
    }

    fn handle_escape(&mut self, ch: u8) -> Option<Command> {
        self.state = DecoderState::Loading;
        let result = match ch {
            ESCAPE_CHAR => {
                // Double escape means just load an escape
                self.load_char(ch);
                None
            }
            CMD_PING => Some(Command::Ping),
            CMD_INFO => Some(Command::Info),
            CMD_ID => Some(Command::Id),
            CMD_RESET => Some(Command::Reset),
            CMD_EPAGE => {
                let num_expected_bytes: usize = 4;
                if self.count >= num_expected_bytes {
                    // Little-endian address in buffer
                    let start = self.count - num_expected_bytes;
                    let address = parse_u32(&self.buffer[start..start + 4]);
                    Some(Command::ErasePage { address })
                } else {
                    Some(Command::Unknown)
                }
            }
            CMD_WPAGE => {
                let num_expected_bytes: usize = 512 + 4;
                if self.count >= num_expected_bytes {
                    // Little-endian address in buffer
                    let start = self.count - num_expected_bytes;
                    let payload = &self.buffer[start..start + num_expected_bytes];
                    let address = parse_u32(&payload[0..4]);
                    Some(Command::WritePage {
                        address: address,
                        data: &payload[4..num_expected_bytes],
                    })
                } else {
                    Some(Command::Unknown)
                }
            }
            CMD_XEBLOCK => {
                let num_expected_bytes: usize = 4;
                if self.count >= num_expected_bytes {
                    // Little-endian address in buffer
                    let start = self.count - num_expected_bytes;
                    let address = parse_u32(&self.buffer[start..start + 4]);
                    Some(Command::EraseExBlock { address })
                } else {
                    Some(Command::Unknown)
                }
            }
            CMD_XWPAGE => {
                let num_expected_bytes: usize = 512 + 4;
                if self.count >= num_expected_bytes {
                    // Little-endian address in buffer
                    let start = self.count - num_expected_bytes;
                    let payload = &self.buffer[start..start + num_expected_bytes];
                    let address = parse_u32(&payload[0..4]);
                    Some(Command::WriteExPage {
                        address: address,
                        data: &payload[4..num_expected_bytes],
                    })
                } else {
                    Some(Command::Unknown)
                }
            }
            CMD_CRCRX => Some(Command::CrcRxBuffer),
            CMD_RRANGE => {
                let num_expected_bytes: usize = 6;
                if self.count >= num_expected_bytes {
                    // Little-endian address in buffer
                    let start = self.count - num_expected_bytes;
                    let address = parse_u32(&self.buffer[start..start + 4]);
                    let length = parse_u16(&self.buffer[start + 4..start + 6]);
                    Some(Command::ReadRange {
                        address: address,
                        length: length,
                    })
                } else {
                    Some(Command::Unknown)
                }
            }
            _ => None,
        };
        // A command signifies the end of the buffer
        if result.is_some() {
            self.count = 0;
        }
        result
    }
}

impl ResponseDecoder {
    /// Create a new `ResponseDecoder`.
    ///
    /// The decoder is fed bytes with the `receive` method.
    pub fn new() -> ResponseDecoder {
        ResponseDecoder {
            state: DecoderState::Loading,
            buffer: [0u8; 520],
            count: 0,
        }
    }

    /// Empty the RX buffer.
    pub fn reset(&mut self) {
        self.count = 0;
    }

    /// Process incoming bytes.
    ///
    /// The decoder is fed bytes with the `receive` method. If not enough
    /// bytes have been seen, this function returns `None`. Once enough bytes
    /// have been seen, it returns `Some(Response)` containing the
    /// decoded Response.
    pub fn receive(&mut self, ch: u8) -> Option<Response> {
        match self.state {
            DecoderState::Loading => self.handle_loading(ch),
            DecoderState::Escape => self.handle_escape(ch),
        }
    }

    fn load_char(&mut self, ch: u8) {
        if self.count < self.buffer.len() {
            self.buffer[self.count] = ch;
            self.count = self.count + 1;
        }
    }

    fn handle_loading(&mut self, ch: u8) -> Option<Response> {
        if ch == ESCAPE_CHAR {
            self.state = DecoderState::Escape;
        } else {
            self.load_char(ch);
        }
        None
    }

    fn handle_escape(&mut self, ch: u8) -> Option<Response> {
        self.state = DecoderState::Loading;
        let result = match ch {
            ESCAPE_CHAR => {
                // Double escape means just load an escape
                self.load_char(ch);
                None
            }
            RES_PONG => Some(Response::Pong),
            _ => None,
        };
        // A command signifies the end of the buffer
        if result.is_some() {
            self.count = 0;
        }
        result
    }
}

impl<'a> CommandEncoder<'a> {
    /// Create a new `CommandEncoder`.
    ///
    /// The encoder takes a reference to a `Command` to encode. The `next` method
    /// will then supply the encoded bytes one at a time.
    pub fn new(command: &'a Command) -> CommandEncoder<'a> {
        CommandEncoder {
            command: command,
            count: 0,
        }
    }

    /// Supply the next encoded byte. Once all the bytes have been emitted, it
    /// returns `None` forevermore.
    pub fn next(&mut self) -> Option<u8> {
        let (inc, result) = match self.command {
            &Command::Ping => Self::render_basic_cmd(self.count, CMD_PING),
            &Command::Info => Self::render_basic_cmd(self.count, CMD_INFO),
            &Command::Id => Self::render_basic_cmd(self.count, CMD_ID),
            &Command::Reset => Self::render_basic_cmd(self.count, CMD_RESET),
            &Command::ErasePage { address } => self.render_erasepage_cmd(address),
            &Command::WritePage { address, data } => self.render_writepage_cmd(address, data),
            _ => unimplemented!("Not implemented"),
        };
        self.count = self.count + inc;
        result
    }

    fn render_u32(idx: usize, value: u32) -> (usize, Option<u8>) {
        println!("Render u32 {} {}", idx, value);
        match idx {
            0 => (1, Some(value as u8)),
            1 => (1, Some((value >> 8) as u8)),
            2 => (1, Some((value >> 16) as u8)),
            3 => (1, Some((value >> 24) as u8)),
            _ => (0, None),
        }
    }

    fn render_buffer(idx: usize, data: &[u8]) -> (usize, Option<u8>) {
        println!("Render buffer {}", idx);
        if (idx < data.len()) && (idx < 512) {
            (1, Some(data[idx]))
        } else if idx < 512 {
            (1, Some(0xFF)) // pad short data with 0xFFs
        } else {
            (0, None)
        }
    }

    fn render_basic_cmd(count: usize, cmd: u8) -> (usize, Option<u8>) {
        match count {
            0 => (1, Some(ESCAPE_CHAR)), // Escape
            1 => (1, Some(cmd)), // Command
            _ => (0, None),
        }
    }

    fn render_erasepage_cmd(&mut self, address: u32) -> (usize, Option<u8>) {
        match self.count {
            0...3 => Self::render_u32(self.count, address),
            _ => Self::render_basic_cmd(self.count - 4, CMD_EPAGE),
        }
    }

    fn render_writepage_cmd(&mut self, address: u32, data: &[u8]) -> (usize, Option<u8>) {
        match self.count {
            0...3 => Self::render_u32(self.count, address),
            4...515 => Self::render_buffer(self.count - 4, data),
            _ => Self::render_basic_cmd(self.count - 516, CMD_WPAGE),
        }
    }
}

impl<'a> ResponseEncoder<'a> {
    /// Create a new `ResponseEncoder`.
    ///
    /// The encoder takes a reference to a `Command` to encode. The `next` method
    /// will then supply the encoded bytes one at a time.
    pub fn new(response: &'a Response) -> ResponseEncoder<'a> {
        ResponseEncoder {
            response: response,
            count: 0,
        }
    }

    /// Supply the next encoded byte. Once all the bytes have been emitted, it
    /// returns `None` forevermore.
    pub fn next(&mut self) -> Option<u8> {
        let (inc, result) = match self.response {
            &Response::Pong => Self::render_basic_rsp(self.count, RES_PONG),
            &Response::Ok => Self::render_basic_rsp(self.count, RES_OK),
            &Response::BadAddress => Self::render_basic_rsp(self.count, RES_BADADDR),
            &Response::Unknown => Self::render_basic_rsp(self.count, RES_UNKNOWN),
            _ => unimplemented!("Not implemented"),
        };
        self.count = self.count + inc;
        result
    }

    fn render_u32(idx: usize, value: u32) -> (usize, Option<u8>) {
        println!("Render u32 {} {}", idx, value);
        match idx {
            0 => (1, Some(value as u8)),
            1 => (1, Some((value >> 8) as u8)),
            2 => (1, Some((value >> 16) as u8)),
            3 => (1, Some((value >> 24) as u8)),
            _ => (0, None),
        }
    }

    fn render_buffer(idx: usize, data: &[u8]) -> (usize, Option<u8>) {
        println!("Render buffer {}", idx);
        if (idx < data.len()) && (idx < 512) {
            (1, Some(data[idx]))
        } else if idx < 512 {
            (1, Some(0xFF)) // pad short data with 0xFFs
        } else {
            (0, None)
        }
    }

    fn render_basic_rsp(count: usize, cmd: u8) -> (usize, Option<u8>) {
        match count {
            0 => (1, Some(ESCAPE_CHAR)), // Escape
            1 => (1, Some(cmd)), // Command
            _ => (0, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_ping_cmd_decode() {
        let mut p = CommandDecoder::new();
        {
            // Garbage should be ignored
            let o = p.receive(0xFF);
            assert!(o.is_none());
        }
        {
            let o = p.receive(ESCAPE_CHAR);
            assert!(o.is_none());
        }
        let o = p.receive(CMD_PING);
        match o.unwrap() {
            Command::Ping => {}
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_ping_cmd_encode() {
        let cmd = Command::Ping;
        let mut e = CommandEncoder::new(&cmd);
        assert_eq!(e.next(), Some(ESCAPE_CHAR));
        assert_eq!(e.next(), Some(CMD_PING));
        assert_eq!(e.next(), None);
        assert_eq!(e.next(), None);
    }

    #[test]
    fn check_pong_rsp_decode() {
        let mut p = ResponseDecoder::new();
        {
            // Garbage should be ignored
            let o = p.receive(0xFF);
            assert!(o.is_none());
        }
        {
            let o = p.receive(ESCAPE_CHAR);
            assert!(o.is_none());
        }
        let o = p.receive(RES_PONG);
        match o.unwrap() {
            Response::Pong => {}
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_pong_rsp_encode() {
        let rsp = Response::Pong;
        let mut e = ResponseEncoder::new(&rsp);
        assert_eq!(e.next(), Some(ESCAPE_CHAR));
        assert_eq!(e.next(), Some(RES_PONG));
        assert_eq!(e.next(), None);
        assert_eq!(e.next(), None);
    }

    #[test]
    fn check_info_cmd_decode() {
        let mut p = CommandDecoder::new();
        {
            let o = p.receive(0xFF);
            assert!(o.is_none());
        }
        {
            let o = p.receive(ESCAPE_CHAR);
            assert!(o.is_none());
        }
        let o = p.receive(CMD_INFO);
        match o.unwrap() {
            Command::Info => {}
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_info_cmd_encode() {
        let cmd = Command::Info;
        let mut e = CommandEncoder::new(&cmd);
        assert_eq!(e.next(), Some(ESCAPE_CHAR));
        assert_eq!(e.next(), Some(CMD_INFO));
        assert_eq!(e.next(), None);
        assert_eq!(e.next(), None);
    }

    #[test]
    fn check_write_page_cmd_decode() {
        let mut p = CommandDecoder::new();
        p.receive(0xEF);
        p.receive(0xBE);
        p.receive(0xAD);
        p.receive(0xDE);
        for i in 0..512 {
            let datum = i as u8;
            p.receive(datum);
            if datum == ESCAPE_CHAR {
                p.receive(datum);
            }
        }
        p.receive(ESCAPE_CHAR); // Escape
        let o = p.receive(CMD_WPAGE); // WriteFlash
        match o.unwrap() {
            Command::WritePage {
                address: address,
                data: ref page,
            } => {
                assert_eq!(address, 0xDEADBEEF);
                assert_eq!(page.len(), 512);
                for i in 0..512 {
                    let datum = i as u8;
                    assert_eq!(datum, page[i as usize]);
                }
            }
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_write_page_cmd_encode() {
        let buffer: [u8; 5] = [0, 1, 2, 3, 4];
        let cmd = Command::WritePage {
            address: 0xDEADBEEF,
            data: &buffer,
        };
        let mut e = CommandEncoder::new(&cmd);
        // 4 byte address, little-endian
        assert_eq!(e.next(), Some(0xEF));
        assert_eq!(e.next(), Some(0xBE));
        assert_eq!(e.next(), Some(0xAD));
        assert_eq!(e.next(), Some(0xDE));
        // 5 bytes of data
        assert_eq!(e.next(), Some(0x00));
        assert_eq!(e.next(), Some(0x01));
        assert_eq!(e.next(), Some(0x02));
        assert_eq!(e.next(), Some(0x03));
        assert_eq!(e.next(), Some(0x04));
        for _ in 0..507 {
            // Padding up to 512 data bytes in the page
            assert_eq!(e.next(), Some(0xFF));
        }
        assert_eq!(e.next(), Some(ESCAPE_CHAR));
        assert_eq!(e.next(), Some(CMD_WPAGE));
        assert_eq!(e.next(), None);
        assert_eq!(e.next(), None);
    }

    #[test]
    fn check_erase_page_cmd_decode() {
        let mut p = CommandDecoder::new();
        p.receive(0xEF);
        p.receive(0xBE);
        p.receive(0xAD);
        p.receive(0xDE);
        p.receive(ESCAPE_CHAR); // Escape
        let o = p.receive(CMD_EPAGE); // ErasePage
        match o.unwrap() {
            Command::ErasePage { address } => {
                assert_eq!(address, 0xDEADBEEF);
            }
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_erase_page_cmd_encode() {
        let cmd = Command::ErasePage { address: 0xDEADBEEF };
        let mut e = CommandEncoder::new(&cmd);
        // 4 byte address, little-endian
        assert_eq!(e.next(), Some(0xEF));
        assert_eq!(e.next(), Some(0xBE));
        assert_eq!(e.next(), Some(0xAD));
        assert_eq!(e.next(), Some(0xDE));
        assert_eq!(e.next(), Some(ESCAPE_CHAR));
        assert_eq!(e.next(), Some(CMD_EPAGE));
        assert_eq!(e.next(), None);
        assert_eq!(e.next(), None);
    }

    #[test]
    fn check_reset_cmd_decode() {
        let mut p = CommandDecoder::new();
        {
            // Garbage should be ignored
            let o = p.receive(0xFF);
            assert!(o.is_none());
        }
        {
            let o = p.receive(ESCAPE_CHAR);
            assert!(o.is_none());
        }
        let o = p.receive(CMD_RESET);
        match o.unwrap() {
            Command::Reset => {}
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_reset_cmd_encode() {
        let cmd = Command::Reset;
        let mut e = CommandEncoder::new(&cmd);
        assert_eq!(e.next(), Some(ESCAPE_CHAR));
        assert_eq!(e.next(), Some(CMD_RESET));
        assert_eq!(e.next(), None);
        assert_eq!(e.next(), None);
    }

    #[test]
    fn check_id_cmd_decode() {
        let mut p = CommandDecoder::new();
        {
            // Garbage should be ignored
            let o = p.receive(0xFF);
            assert!(o.is_none());
        }
        {
            let o = p.receive(ESCAPE_CHAR);
            assert!(o.is_none());
        }
        let o = p.receive(CMD_ID);
        match o.unwrap() {
            Command::Id => {}
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_id_cmd_encode() {
        let cmd = Command::Id;
        let mut e = CommandEncoder::new(&cmd);
        assert_eq!(e.next(), Some(ESCAPE_CHAR));
        assert_eq!(e.next(), Some(CMD_ID));
        assert_eq!(e.next(), None);
        assert_eq!(e.next(), None);
    }
}


/// Convert a four-byte array to a u32 - little endian.
fn parse_u32(data: &[u8]) -> u32 {
    let mut result: u32 = 0;
    result += data[3] as u32;
    result <<= 8;
    result += data[2] as u32;
    result <<= 8;
    result += data[1] as u32;
    result <<= 8;
    result += data[0] as u32;
    result
}

/// Convert a two-byte array to a u16 - little-endian.
fn parse_u16(data: &[u8]) -> u16 {
    let mut result: u16 = 0;
    result += data[1] as u16;
    result <<= 8;
    result += data[0] as u16;
    result
}

//
// End of file
//
