//! Implements the Tockloader protocol.
//!
//! TockOS applications are loaded with `tockloader`.
//! This speaks to the TockOS bootloader using a specific
//! protocol. This crate implements that protocol so
//! that you can write future tockloader compatible bootloaders
//! in Rust!
//#![no_std]

enum DecoderState {
    Loading,
    Escape,
}

/// Commands and Reponses supported by the protocol
#[derive(Debug)]
pub enum CommandReponse<'a> {
    PingCmd, // 0x01
    InfoCmd, // 0x03
    ResetCmd, // 0x05
    ErasePageCmd { address: u32 }, // 0x06
    WritePageCmd { address: u32, data: &'a [u8] }, // 0x07
    UnknownCmd, // Not seen on the wire

    PingRsp, // 0x11
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

#[derive(Debug)]
enum EncoderState {
    Escape,
    CommandReponse,
    Data(usize),
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
        if ch == 0xFC {
            self.state = DecoderState::Escape;
        } else {
            self.load_char(ch);
        }
        None
    }

    fn handle_escape(&mut self, ch: u8) -> Option<CommandReponse> {
        self.state = DecoderState::Loading;
        let result = match ch {
            0xFC => {
                // Double escape means just load an escape
                self.load_char(ch);
                None
            }
            0x01 => Some(CommandReponse::PingCmd),
            0x03 => Some(CommandReponse::InfoCmd),
            0x05 => Some(CommandReponse::ResetCmd),
            0x06 => {
                if self.count >= 4 {
                    // Little-endian address in buffer
                    let addr = Self::parse_u32(&self.buffer[self.count - 4..self.count - 1]);
                    Some(CommandReponse::ErasePageCmd { address: addr })
                } else {
                    Some(CommandReponse::UnknownCmd)
                }
            }
            0x07 => {
                let num_expected_bytes: usize = 512 + 4;
                if self.count >= num_expected_bytes {
                    // Little-endian address in buffer
                    let start = self.count - num_expected_bytes;
                    let addr = Self::parse_u32(&self.buffer[start..start + 4]);
                    Some(CommandReponse::WritePageCmd {
                        address: addr,
                        data: &self.buffer[start + 4..start + num_expected_bytes],
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
        match self.command {
            &CommandReponse::PingCmd => self.render_ping_cmd(),
            &CommandReponse::InfoCmd => self.render_info_cmd(),
            &CommandReponse::ResetCmd => self.render_reset_cmd(),
            &CommandReponse::ErasePageCmd { address } => self.render_erasepage_cmd(address),
            &CommandReponse::WritePageCmd {
                address,
                data,
            } => self.render_writepage_cmd(address, data),
            &CommandReponse::UnknownCmd => self.render_unknown_cmd(),
            &CommandReponse::PingRsp => self.render_ping_rsp(),
            &CommandReponse::OkRsp => self.render_ok_rsp(),
            &CommandReponse::BadAddressRsp => self.render_badaddress_rsp(),
            &CommandReponse::UnknownRsp => self.render_unknown_rsp(),
        }
    }

    pub fn render_ping_cmd(&mut self) -> Option<u8> {
        None
    }

    pub fn render_info_cmd(&mut self) -> Option<u8> {
        None
    }

    pub fn render_reset_cmd(&mut self) -> Option<u8> {
        None
    }

    pub fn render_erasepage_cmd(&mut self, _address: u32) -> Option<u8> {
        None
    }

    pub fn render_writepage_cmd(&mut self, _address: u32, _data: &[u8]) -> Option<u8> {
        None
    }

    pub fn render_unknown_cmd(&mut self) -> Option<u8> {
        None
    }

    pub fn render_ping_rsp(&mut self) -> Option<u8> {
        None
    }

    pub fn render_ok_rsp(&mut self) -> Option<u8> {
        None
    }

    pub fn render_badaddress_rsp(&mut self) -> Option<u8> {
        None
    }

    pub fn render_unknown_rsp(&mut self) -> Option<u8> {
        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_ping() {
        let mut p = Decoder::new();
        {
            let o = p.receive(0xFF);
            assert!(o.is_none());
        }
        {
            let o = p.receive(0xFC);
            assert!(o.is_none());
        }
        let o = p.receive(0x01);
        match o.unwrap() {
            CommandReponse::PingCmd => {}
            e => panic!("Did not expect: {:?}", e),
        }
    }

    #[test]
    fn check_info() {
        let mut p = Decoder::new();
        {
            let o = p.receive(0xFF);
            assert!(o.is_none());
        }
        {
            let o = p.receive(0xFC);
            assert!(o.is_none());
        }
        let o = p.receive(0x03);
        match o.unwrap() {
            CommandReponse::InfoCmd => {}
            e => panic!("Did not expect: {:?}", e),
        }
    }

    fn make_byte(index: u32) -> u8 {
        (index & 0xFF) as u8
    }

    #[test]
    fn check_write() {
        let mut p = Decoder::new();
        p.receive(0xEF);
        p.receive(0xBE);
        p.receive(0xAD);
        p.receive(0xDE);
        for i in 0..512 {
            let datum = make_byte(i);
            p.receive(datum);
            if datum == 0xFC {
                p.receive(datum);
            }
        }
        p.receive(0xFC); // Escape
        let o = p.receive(0x07); // WriteFlash
        match o.unwrap() {
            CommandReponse::WritePageCmd {
                address: addr,
                data: ref page,
            } => {
                assert_eq!(addr, 0xDEADBEEF);
                assert_eq!(page.len(), 512);
                for i in 0..512 {
                    let datum = make_byte(i);
                    assert_eq!(datum, page[i as usize]);
                }
            }
            e => panic!("Did not expect: {:?}", e),
        }
    }
}
