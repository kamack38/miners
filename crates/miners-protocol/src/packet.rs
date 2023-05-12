use std::io::Read;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPacket {
    pub id: i32,
    pub data: Vec<u8>,
}

impl RawPacket {
    pub fn new(id: i32, data: Vec<u8>) -> RawPacket {
        RawPacket {
            id,
            data,
        }
    }

    pub fn empty(id: i32) -> RawPacket {
        RawPacket {
            id,
            data: Vec::new(),
        }
    }

    pub fn read_from_socket(socket: &mut std::net::TcpStream, threshold: i32) -> Result<RawPacket, crate::PacketError> {
        // TODO: Implement compression and encryption
        // Read length (varint)
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

        let uncompressed_length = if threshold > 0 {
            p.read_varint()
        } else {
            -1
        };

        let data = p.data;

        // Read id
        let mut packet = RawPacket::empty(0);
        if uncompressed_length >= threshold && threshold > 0 {
            // Decompress
            let mut d = flate2::read::ZlibDecoder::new(&data[..]);
            let mut decompressed = Vec::new();
            d.read_to_end(&mut decompressed)
                .map_err(|e| crate::PacketError::text(format!("Error decompressing packet: {:?}", e)))?;
            packet.data = decompressed;
        } else {
            packet.data = data;
        }
        packet.id = packet.read_varint();
        
        // Return
        Ok(packet)
    }

    // ====< Writers >====
    pub fn write_byte(&mut self, byte: u8) {
        self.data.push(byte);
    }

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

    /// Writes bool to the packet
    pub fn write_bool(&mut self, boolean: bool) {
        self.write_byte(boolean as u8);
    }

    // ====< Readers >====
    pub fn read_byte(&mut self) -> u8 {
        let byte = self.data[0];
        self.data.remove(0);
        byte
    }

    pub fn read_uuid(&mut self) -> u128 {
        let l1 = self.read_ulong();
        let l2 = self.read_ulong();
        ((l1 as u128) << 64) | (l2 as u128)
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

    pub fn read_short(&mut self) -> i16 {
        let mut result = self.read_byte() as i16;
        result <<= 8;
        result |= self.read_byte() as i16;
        result
    }

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

pub trait IntoPacket {
    fn into_packet(self, protocol_version: i32) -> RawPacket;
}

impl IntoPacket for RawPacket {
    fn into_packet(self, _protocol_version: i32) -> RawPacket {
        self
    }
}