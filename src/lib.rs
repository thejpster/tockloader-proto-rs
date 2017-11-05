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
// const CMD_ID: u8          = 0x04;
const CMD_RESET: u8 = 0x05;
const CMD_EPAGE: u8 = 0x06;
const CMD_WPAGE: u8 = 0x07;
// const CMD_XEBLOCK: u8     = 0x08;
// const CMD_XWPAGE: u8      = 0x09;
// const CMD_CRCRX: u8       = 0x10;
// const CMD_RRANGE: u8      = 0x11;
// const CMD_XRRANGE: u8     = 0x12;
// const CMD_SATTR: u8       = 0x13;
// const CMD_GATTR: u8       = 0x14;
// const CMD_CRCIF: u8       = 0x15;
// const CMD_CRCEF: u8       = 0x16;
// const CMD_XEPAGE: u8      = 0x17;
// const CMD_XFINIT: u8      = 0x18;
// const CMD_CLKOUT: u8      = 0x19;
// const CMD_WUSER: u8       = 0x20;
// const CMD_CHANGE_BAUD: u8 = 0x21;

// const RES_OVERFLOW: u8    = 0x10;
const RES_PONG: u8 = 0x11;
// const RES_BADADDR: u8     = 0x12;
// const RES_INTERROR: u8    = 0x13;
// const RES_BADARGS: u8     = 0x14;
// const RES_OK: u8          = 0x15;
// const RES_UNKNOWN: u8     = 0x16;
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

/// Commands and Reponses supported by the protocol
#[derive(Debug)]
pub enum CommandReponse<'a> {
    PingCmd, // CMD_PING
    InfoCmd, // CMD_INFO
    ResetCmd, // CMD_RESET
    ErasePageCmd { address: u32 }, // CMD_EPAGE
    WritePageCmd { address: u32, data: &'a [u8] }, // CMD_WPAGE
    UnknownCmd, // Not seen on the wire

    PingRsp, // RES_PONG
    OkRsp, // 0x15
    BadAddressRsp, // 0x12
    UnknownRsp, // 0x16
}

/// The `Decoder` takes bytes and gives you `CommandReponse`s.
pub struct Decoder {
    state: DecoderState,
    buffer: [u8; 520],
    count: usize,
}

/// The `Encoder` takes a `CommandReponse` and gives you bytes.
pub struct Encoder<'a> {
    command: &'a CommandReponse<'a>,
    count: usize,
}

impl Decoder {
    pub fn new() -> Decoder {
        Decoder {
            state: DecoderState::Loading,
            buffer: [0u8; 520],
            count: 0,
        }
    }

    pub fn reset(&mut self) {
        self.count = 0;
    }

    pub fn receive(&mut self, ch: u8) -> Option<CommandReponse> {
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

    fn handle_loading(&mut self, ch: u8) -> Option<CommandReponse> {
        if ch == ESCAPE_CHAR {
            self.state = DecoderState::Escape;
        } else {
            self.load_char(ch);
        }
        None
    }

    fn handle_escape(&mut self, ch: u8) -> Option<CommandReponse> {
        self.state = DecoderState::Loading;
        let result = match ch {
            ESCAPE_CHAR => {
                // Double escape means just load an escape
                self.load_char(ch);
                None
            }
            CMD_PING => Some(CommandReponse::PingCmd),
            RES_PONG => Some(CommandReponse::PingRsp),
            CMD_INFO => Some(CommandReponse::InfoCmd),
            CMD_RESET => Some(CommandReponse::ResetCmd),
            CMD_EPAGE => {
                let num_expected_bytes: usize = 4;
                if self.count >= num_expected_bytes {
                    // Little-endian address in buffer
                    let start = self.count - num_expected_bytes;
                    let addr = Self::parse_u32(&self.buffer[start..start + 4]);
                    Some(CommandReponse::ErasePageCmd { address: addr })
                } else {
                    Some(CommandReponse::UnknownCmd)
                }
            }
            CMD_WPAGE => {
                let num_expected_bytes: usize = 512 + 4;
                if self.count >= num_expected_bytes {
                    // Little-endian address in buffer
                    let start = self.count - num_expected_bytes;
                    let payload = &self.buffer[start..start + num_expected_bytes];
                    let addr = Self::parse_u32(&payload[0..4]);
                    Some(CommandReponse::WritePageCmd {
                        address: addr,
                        data: &payload[4..num_expected_bytes],
                    })
                } else {
                    Some(CommandReponse::UnknownCmd)
                }
            }
            _ => None,
        };
        if result.is_some() {
            self.count = 0;
        }
        result
    }

    fn parse_u32(data: &[u8]) -> u32 {
        println!("Parsing: {:?}", data);
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
}

impl<'a> Encoder<'a> {
    pub fn new(command: &'a CommandReponse) -> Encoder<'a> {
        Encoder {
            command: command,
            count: 0,
        }
    }

    pub fn next(&mut self) -> Option<u8> {
        let (inc, result) = match self.command {
            &CommandReponse::PingCmd => self.render_ping_cmd(),
            &CommandReponse::InfoCmd => self.render_info_cmd(),
            &CommandReponse::ResetCmd => self.render_reset_cmd(),
            &CommandReponse::ErasePageCmd { address } => self.render_erasepage_cmd(address),
            &CommandReponse::WritePageCmd { address, data } => {
                self.render_writepage_cmd(address, data)
            }
            &CommandReponse::UnknownCmd => self.render_unknown_cmd(),
            &CommandReponse::PingRsp => self.render_ping_rsp(),
            &CommandReponse::OkRsp => self.render_ok_rsp(),
            &CommandReponse::BadAddressRsp => self.render_badaddress_rsp(),
            &CommandReponse::UnknownRsp => self.render_unknown_rsp(),
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

    pub fn render_ping_cmd(&mut self) -> (usize, Option<u8>) {
        match self.count {
            0 => (1, Some(ESCAPE_CHAR)), // Escape
            1 => (1, Some(CMD_PING)), // Command
            _ => (0, None),
        }
    }

    pub fn render_info_cmd(&mut self) -> (usize, Option<u8>) {
        match self.count {
            0 => (1, Some(ESCAPE_CHAR)), // Escape
            1 => (1, Some(CMD_INFO)), // Command
            _ => (0, None),
        }
    }

    pub fn render_reset_cmd(&mut self) -> (usize, Option<u8>) {
        match self.count {
            0 => (1, Some(ESCAPE_CHAR)), // Escape
            1 => (1, Some(CMD_RESET)), // Command
            _ => (0, None),
        }
    }

    pub fn render_erasepage_cmd(&mut self, address: u32) -> (usize, Option<u8>) {
        match self.count {
            0...3 => Self::render_u32(self.count, address),
            4 => (1, Some(ESCAPE_CHAR)), // Escape
            5 => (1, Some(CMD_EPAGE)), // Command
            _ => (0, None),
        }
    }

    pub fn render_writepage_cmd(&mut self, address: u32, data: &[u8]) -> (usize, Option<u8>) {
        match self.count {
            0...3 => Self::render_u32(self.count, address),
            4...515 => Self::render_buffer(self.count - 4, data),
            516 => (1, Some(ESCAPE_CHAR)), // Escape
            517 => (1, Some(CMD_WPAGE)), // Command
            _ => (0, None),
        }
    }

    pub fn render_unknown_cmd(&mut self) -> (usize, Option<u8>) {
        (0, None)
    }

    pub fn render_ping_rsp(&mut self) -> (usize, Option<u8>) {
        match self.count {
            0 => (1, Some(ESCAPE_CHAR)), // Escape
            1 => (1, Some(RES_PONG)), // Response
            _ => (0, None),
        }
    }

    pub fn render_ok_rsp(&mut self) -> (usize, Option<u8>) {
        (0, None)
    }

    pub fn render_badaddress_rsp(&mut self) -> (usize, Option<u8>) {
        (0, None)
    }

    pub fn render_unknown_rsp(&mut self) -> (usize, Option<u8>) {
        (0, None)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_ping_cmd_decode() {
        let mut p = Decoder::new();
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
            CommandReponse::PingCmd => {}
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_ping_cmd_encode() {
        let cmd = CommandReponse::PingCmd;
        let mut e = Encoder::new(&cmd);
        assert_eq!(e.next(), Some(ESCAPE_CHAR));
        assert_eq!(e.next(), Some(CMD_PING));
        assert_eq!(e.next(), None);
        assert_eq!(e.next(), None);
    }

    #[test]
    fn check_ping_rsp_decode() {
        let mut p = Decoder::new();
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
            CommandReponse::PingRsp => {}
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_ping_rsp_encode() {
        let cmd = CommandReponse::PingRsp;
        let mut e = Encoder::new(&cmd);
        assert_eq!(e.next(), Some(ESCAPE_CHAR));
        assert_eq!(e.next(), Some(RES_PONG));
        assert_eq!(e.next(), None);
        assert_eq!(e.next(), None);
    }

    #[test]
    fn check_info_cmd_decode() {
        let mut p = Decoder::new();
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
            CommandReponse::InfoCmd => {}
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_info_cmd_encode() {
        let cmd = CommandReponse::InfoCmd;
        let mut e = Encoder::new(&cmd);
        assert_eq!(e.next(), Some(ESCAPE_CHAR));
        assert_eq!(e.next(), Some(CMD_INFO));
        assert_eq!(e.next(), None);
        assert_eq!(e.next(), None);
    }

    #[test]
    fn check_write_page_cmd_decode() {
        let mut p = Decoder::new();
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
            CommandReponse::WritePageCmd {
                address: addr,
                data: ref page,
            } => {
                assert_eq!(addr, 0xDEADBEEF);
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
        let cmd = CommandReponse::WritePageCmd {
            address: 0xDEADBEEF,
            data: &buffer,
        };
        let mut e = Encoder::new(&cmd);
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
        let mut p = Decoder::new();
        p.receive(0xEF);
        p.receive(0xBE);
        p.receive(0xAD);
        p.receive(0xDE);
        p.receive(ESCAPE_CHAR); // Escape
        let o = p.receive(CMD_EPAGE); // ErasePage
        match o.unwrap() {
            CommandReponse::ErasePageCmd { address: addr } => {
                assert_eq!(addr, 0xDEADBEEF);
            }
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_erase_page_cmd_encode() {
        let cmd = CommandReponse::ErasePageCmd { address: 0xDEADBEEF };
        let mut e = Encoder::new(&cmd);
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
        let mut p = Decoder::new();
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
            CommandReponse::ResetCmd => {}
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_reset_cmd_encode() {
        let cmd = CommandReponse::ResetCmd;
        let mut e = Encoder::new(&cmd);
        assert_eq!(e.next(), Some(ESCAPE_CHAR));
        assert_eq!(e.next(), Some(CMD_RESET));
        assert_eq!(e.next(), None);
        assert_eq!(e.next(), None);
    }


}
