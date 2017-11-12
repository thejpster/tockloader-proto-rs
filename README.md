# Tockloader Protocol

[![Build Status](https://travis-ci.org/thejpster/tockloader-proto-rs.svg?branch=master)](https://travis-ci.org/thejpster/tockloader-proto-rs)

Implements the Tockloader protocol.

TockOS applications are loaded with `tockloader`. This speaks to the TockOS
bootloader using a specific protocol. This crate implements that protocol so
that you can write future `tockloader` compatible bootloaders in Rust!

Usage
-----

In your embedded bootloader, you need a loop that looks something like:

```rust
use tockloader_proto::{ResponseEncoder, CommandDecoder};

#[no_mangle]
pub extern "C" fn main() {
    let mut uart = uart::Uart::new(uart::UartId::Uart0, 115200, uart::NewlineMode::Binary);
    let mut decoder = CommandDecoder::new();
    loop {
        if let Ok(Some(ch)) = uart.getc_try() {
            let mut need_reset = false;
            let response = match decoder.receive(ch) {
                Ok(None) => None,
                Ok(Some(tockloader_proto::Command::Ping)) => Some(tockloader_proto::Response::Pong),
                Ok(Some(tockloader_proto::Command::Reset)) => {
                    need_reset = true;
                    None
                },
                Ok(Some(_)) => Some(tockloader_proto::Response::Unknown),
                Err(_) => Some(tockloader_proto::Response::InternalError),
            };
            if need_reset {
                decoder.reset();
            }
            if let Some(response) = response {
                let mut encoder = ResponseEncoder::new(&response).unwrap();
                while let Some(byte) = encoder.next() {
                    uart.putc(byte);
                }
            }
        }
    }
}
```

Using this library in a CLI flash tool (like tockloader) is left as an excercise for the read (hint: you want `ResponseDecoder` and `CommandEncoder`).

Over the Wire Protocol
----------------------

This is all cribbed from the TockOS documentation.

All messages are sent over UART and are initiated by the client and responded
to by the bootloader.

### Framing

#### Commands

```
                             0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
Message (arbitrary length)  | Escape Char   | Command       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

- `Message`: The command packet as specified by the individual commands.
             Escaped by replacing all `0xFC` with two consecutive `0xFC`.
- `Escape Character`: `0xFC`.
- `Command`: The command byte.


#### Response

```
 0                   1
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Escape Char   | Response      | Message (arbitrary length)
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```
- `Escape Character`: `0xFC`.
- `Response`: The response byte.
- `Message`: The response packet as specified by the individual commands.
             Escaped by replacing all `0xFC` with two consecutive `0xFC`.



### Commands

#### `PING`

Send a ping to the bootloader. If everything is working it will respond with a
pong.

##### Command
- `Command`: `0x01`.
- `Message`: `None`.

##### Response
- `Response`: `0x11`.
- `Message`: `None`.


#### `INFO`

Retrieve an information string from the bootloader.

##### Command
- `Command`: `0x03`.
- `Message`: `None`.

##### Response

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Length        | String...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
                     192 bytes                                  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```
- `Response`: `0x25`
- `Length`: Length of the information string.
- `String`: `Length` bytes of information string and 192-length zeros.


#### `RESET`

Reset the internal buffer pointers in the bootloader. This is typically
called before each command.

##### Command
- `Command`: `0x05`.
- `Message`: `None`.

##### Response
None.


#### `ERASE_PAGE`

Erase a page of internal flash.

##### Command
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Address                                                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```
- `Command`: `0x06`.
- `Address`: The address of the page to erase. Little endian.

##### Response
- `Response`: `0x15`.
- `Message`: `None`.



#### `WRITE_PAGE`

Write a page of internal flash.

##### Command
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Address                                                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Data...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
             (512 bytes)                                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```
- `Command`: `0x07`.
- `Address`: The address of the page to write. Little endian.
- `Data`: 512 data bytes to write to the page.

##### Response
- `Response`: `0x15`.
- `Message`: `None`.


#### `READ_RANGE`

Read an arbitrary rage of internal flash.

##### Command
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Address                                                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Length                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```
- `Command`: `0x06`.
- `Address`: The address of the page to erase. Little endian.
- `Length`: The number of bytes to read.

##### Response
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Data...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
             (arbitrary length)                                 |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```
- `Response`: `0x20`.
- `Data`: Bytes read back from flash.



#### `SET_ATTRIBUTE`

Set an attribute at a given index in the internal flash.

##### Command
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Index         | Key
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
                | Length        | Value
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
             (arbitrary length)                                 |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```
- `Command`: `0x13`.
- `Index`: The attribute index to set. 0-15.
- `Key`: Eight byte key, zero padded.
- `Length`: Length of the value. 1-55.
- `Value`: `Length` bytes of value to be stored in the attribute.

##### Response
- `Response`: `0x15`.
- `Message`: `None`.


#### `GET_ATTRIBUTE`

Get an attribute at a given index from the internal flash.

##### Command
```
 0
 0 1 2 3 4 5 6 7
+-+-+-+-+-+-+-+-+
| Index         |
+-+-+-+-+-+-+-+-+
```
- `Command`: `0x13`.
- `Index`: The attribute index to get. 0-15.

##### Response
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Key
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
                                                                |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Length        | Value
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
             (55 bytes)                                         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```
- `Response`: `0x22`.
- `Key`: Eight byte key, zero padded.
- `Length`: Length of the value. 1-55.
- `Value`: 55 bytes of potential value.



#### `CRC_INTERNAL_FLASH`

Get the CRC of a range of internal flash.

##### Command
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Address                                                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Length                                                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```
- `Command`: `0x13`.
- `Address`: The address to begin the CRC at. Little endian.
- `Length`: The length of the range to calculate the CRC over.

##### Response
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| CRC                                                           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```
- `Response`: `0x23`.
- `CRC`: The calculated CRC.



#### `CHANGE_BAUD_RATE`

Set a new baud rate for the bootloader.

##### Command
```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| SubCmd        | Baud Rate
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
                |
+-+-+-+-+-+-+-+-+
```
- `Command`: `0x21`.
- `SubCmd`: The subcommand. `0x01` is used to set the new baud rate.
  When subcommand `0x01` is sent, the response will be sent at the old
  baud rate, but the bootloader will switch to the new baud rate after sending
  the response. To confirm that everything is working, the bootloader expects
  to see the `CHANGE_BAUD_RATE` command sent again, this time with subcommand
  `0x02`. Do not send a `RESET` command between the two `CHANGE_BAUD_RATE`
  commands. Ensure that the same baud rate is sent in both messages.
- `Baud Rate`: The new baud rate to use. Little endian.

##### Response
- `Response`: `0x15`.
- `Message`: `None`.

