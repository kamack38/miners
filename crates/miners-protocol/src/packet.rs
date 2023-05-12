use std::io::Read;

/// Represents a raw packet (id + data)
/// 
/// This is the packet that is sent over the network.
/// Data can be extracted from it using `read_*` methods
/// as well as written to it using `write_*` methods.
/// 
/// # Example
/// ```rs
/// let mut packet = RawPacket::empty(0);
/// packet.write_string("Hello world!");
/// packet.write_bool(true);
/// packet.write_varint(123456);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPacket {
    pub id: i32,
    pub data: Vec<u8>,
}

impl RawPacket {
    /// Creates a new raw packet with given id and data
    pub fn new(id: i32, data: Vec<u8>) -> RawPacket {
        RawPacket {
            id,
            data,
        }
    }

    /// Creates a new empty raw packet with given id and no data
    pub fn empty(id: i32) -> RawPacket {
        RawPacket {
            id,
            data: Vec::new(),
        }
    }

    /// This is mainly used internally to read packets from the socket
    pub fn read_from_socket(socket: &mut std::net::TcpStream, threshold: i32, non_blocking: bool) -> Result<RawPacket, crate::PacketError> {
        // TODO: Implement encryption

        // If non_blocking is true, this will return an error if there is no data to read
        if non_blocking {
            let mut buf = [0];
            if socket.peek(&mut buf).is_err() {
                return Err(crate::PacketError::text("No data to read".to_string()));
            }
        }

        // Read packet length (varint)
        let mut length = 0;
        let mut buf = [0];
        for i in 0..4 {
            socket.read_exact(&mut buf).ok();
            length |= ((buf[0] & 0b0111_1111) as i32) << (7 * i);
            if (buf[0] & 0b1000_0000) == 0 {
                break;
            }
        }
        
        // Read packet data
        let mut data = vec![0; length as usize];
        socket.read_exact(&mut data).unwrap();

        let mut p = RawPacket::new(0, data);

        // If threshold for compression is set (compression is enabled) read uncompressed length
        let uncompressed_length = if threshold > 0 {
            p.read_varint()
        } else {
            -1
        };

        // Get data from packet
        let data = p.data;

        // Create new packet with our data (currently without id)
        let mut packet = RawPacket::empty(0);
        if uncompressed_length >= threshold && threshold > 0 {
            // Decompress
            let mut d = flate2::read::ZlibDecoder::new(&data[..]);
            let mut decompressed = Vec::new();
            d.read_to_end(&mut decompressed)
                .map_err(|e| crate::PacketError::text(format!("Error decompressing packet: {:?}", e)))?;
            // Set packet data to decompressed data
            packet.data = decompressed;
        } else {
            // Set packet data to original data
            packet.data = data;
        }
        
        // Set packet id to first varint in packet data
        packet.id = packet.read_varint();
        
        // Return
        Ok(packet)
    }

    // ====< Writers >====
    /// Writes single byte to the packet
    pub fn write_byte(&mut self, byte: u8) {
        self.data.push(byte);
    }

    /// Writes multiple bytes to the packet
    pub fn write_bytes(&mut self, bytes: Vec<u8>) {
        self.data.extend(bytes);
    }

    /// Writes a VarInt to the packet
    pub fn write_varint(&mut self, mut value: i32) {
        let mut buf = [0];
        loop {
            buf[0] = (value & 0b0111_1111) as u8;
            value = (value >> 7) & (i32::max_value() >> 6);
            if value != 0 {
                buf[0] |= 0b1000_0000;
            }

            self.data.extend_from_slice(&buf[..]);

            if value == 0 {
                break;
            }
        }
    }

    /// Writes a String to the packet
    pub fn write_string(&mut self, string: &str) {
        self.write_varint(string.len() as i32);
        for byte in string.bytes() {
            self.write_byte(byte);
        }
    }

    /// Writes an unsigned short to the packet
    pub fn write_ushort(&mut self, short: u16) {
        self.write_byte((short >> 8) as u8);
        self.write_byte(short as u8);
    }

    /// Writes signed long to the packet
    pub fn write_long(&mut self, long: i64) {
        // Write long as 8 bytes (from MSB to LSB)
        self.write_byte((long >> 56) as u8);
        self.write_byte((long >> 48) as u8);
        self.write_byte((long >> 40) as u8);
        self.write_byte((long >> 32) as u8);
        self.write_byte((long >> 24) as u8);
        self.write_byte((long >> 16) as u8);
        self.write_byte((long >> 8) as u8);
        self.write_byte(long as u8);
    }

    /// Writes unsigned long to the packet
    pub fn write_ulong(&mut self, long: u64) {
        // Write long as 8 bytes (from MSB to LSB)
        self.write_byte((long >> 56) as u8);
        self.write_byte((long >> 48) as u8);
        self.write_byte((long >> 40) as u8);
        self.write_byte((long >> 32) as u8);
        self.write_byte((long >> 24) as u8);
        self.write_byte((long >> 16) as u8);
        self.write_byte((long >> 8) as u8);
        self.write_byte(long as u8);
    }

    /// Writes bool to the packet
    pub fn write_bool(&mut self, boolean: bool) {
        self.write_byte(boolean as u8);
    }

    // ====< Readers >====
    /// Reads a byte from the packet
    pub fn read_byte(&mut self) -> u8 {
        let byte = self.data[0];
        self.data.remove(0);
        byte
    }

    /// Reads a UUID from the packet (as u128)
    pub fn read_uuid(&mut self) -> u128 {
        let l1 = self.read_ulong();
        let l2 = self.read_ulong();
        (l1 as u128) | ((l2 as u128) << 64)
    }

    /// Reads a VarInt from the packet
    pub fn read_varint(&mut self) -> i32 {
        let mut buf = [0];
        let mut result = 0;
        for i in 0..4 {
            buf[0] = self.read_byte();
            result |= ((buf[0] & 0b0111_1111) as i32) << (7 * i);
            if (buf[0] & 0b1000_0000) == 0 {
                break;
            }
        }
        result
    }

    /// Tries to read a VarInt from the packet
    /// Returns None if there is no VarInt to read
    pub fn try_read_varint(&mut self) -> Option<i32> {
        let mut buf = [0];
        let mut result = 0;
        for i in 0..4 {
            if self.data.len() == 0 {
                return None;
            }
            buf[0] = self.read_byte();
            result |= ((buf[0] & 0b0111_1111) as i32) << (7 * i);
            if (buf[0] & 0b1000_0000) == 0 {
                break;
            }
        }
        Some(result)
    }

    /// Reads a String from the packet
    pub fn read_string(&mut self) -> String {
        let length = self.read_varint();
        let mut result = String::new();
        for _ in 0..length {
            result.push(self.read_byte() as char);
        }
        result
    }

    /// Reads a String from the packet (prefixed with unsigned short instead of default VarInt)
    pub fn read_string_ushort(&mut self) -> String {
        let length = self.read_ushort();
        let mut result = String::new();
        for _ in 0..length {
            result.push(self.read_byte() as char);
        }
        result
    }

    /// Tries to read a String from the packet
    /// Returns None if there is no String to read
    pub fn try_read_string(&mut self) -> Option<String> {
        let length = match self.try_read_varint() {
            Some(length) => length,
            None => return None,
        };
        let mut result = String::new();
        for _ in 0..length {
            if self.data.len() == 0 {
                return None;
            }
            result.push(self.read_byte() as char);
        }
        Some(result)
    }

    /// Reads an unsigned short from the packet
    pub fn read_ushort(&mut self) -> u16 {
        let mut result = self.read_byte() as u16;
        result <<= 8;
        result |= self.read_byte() as u16;
        result
    }

    /// Reads u64 from the packet
    pub fn read_ulong(&mut self) -> u64 {
        // TODO: Make this prettier XD
        let mut result = self
            .read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result
    }

    /// Reads i64 from the packet
    pub fn read_long(&mut self) -> i64 {
        let mut result = self.read_byte() as i64;
        result <<= 8;
        result |= self.read_byte() as i64;
        result <<= 8;
        result |= self.read_byte() as i64;
        result <<= 8;
        result |= self.read_byte() as i64;
        result <<= 8;
        result |= self.read_byte() as i64;
        result <<= 8;
        result |= self.read_byte() as i64;
        result <<= 8;
        result |= self.read_byte() as i64;
        result <<= 8;
        result |= self.read_byte() as i64;
        result
    }

    /// Reads i32 from the packet
    pub fn read_int(&mut self) -> i32 {
        let mut result = self.read_byte() as i32;
        result <<= 8;
        result |= self.read_byte() as i32;
        result <<= 8;
        result |= self.read_byte() as i32;
        result <<= 8;
        result |= self.read_byte() as i32;
        result
    }

    /// Reads i16 from the packet
    pub fn read_short(&mut self) -> i16 {
        let mut result = self.read_byte() as i16;
        result <<= 8;
        result |= self.read_byte() as i16;
        result
    }

    /// Reads f32 from the packet
    pub fn read_float(&mut self) -> f32 {
        let mut result = self.read_byte() as u32;
        result <<= 8;
        result |= self.read_byte() as u32;
        result <<= 8;
        result |= self.read_byte() as u32;
        result <<= 8;
        result |= self.read_byte() as u32;
        f32::from_bits(result)
    }

    /// Reads f64 from the packet
    pub fn read_double(&mut self) -> f64 {
        let mut result = self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        result <<= 8;
        result |= self.read_byte() as u64;
        f64::from_bits(result)
    }

    /// Reads a bool from the packet
    pub fn read_bool(&mut self) -> bool {
        self.read_byte() == 1
    }

    /// Reads `n` bytes from the packet
    pub fn read_bytes(&mut self, n: usize) -> Vec<u8> {
        let mut result = Vec::new();
        for _ in 0..n {
            result.push(self.read_byte());
        }
        result
    }
}

/// Trait for converting a type into a packet (Should be implemented for all packet types that can be sent)
pub trait IntoPacket {
    fn into_packet(self, protocol_version: i32) -> RawPacket;
}

impl IntoPacket for RawPacket {
    fn into_packet(self, _protocol_version: i32) -> RawPacket {
        self
    }
}