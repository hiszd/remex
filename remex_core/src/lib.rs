use heapless::Vec as StackVec;

pub type RawPacket = [u8; 128];

// [packet number, total packets, ...information]
#[derive(Debug, Clone)]
pub struct Packet {
    pub number: u8,
    pub total: u8,
    pub information: StackVec<u8, 126>,
}

impl Packet {
    pub fn new(number: u8, total: u8, information: StackVec<u8, 126>) -> Self {
        Packet {
            number,
            total,
            information,
        }
    }

    pub fn to_vec(self) -> StackVec<u8, 128> {
        let mut vc: StackVec<u8, 128> = StackVec::from_slice(&[self.number, self.total]).unwrap();
        vc.extend(self.information);
        vc
    }
}

impl Into<RawPacket> for Packet {
    fn into(self) -> RawPacket {
        let mut vc: StackVec<u8, 128> = StackVec::from_slice(&[self.number, self.total]).unwrap();
        vc.extend(self.information);
        let mut rtrn: [u8; 128] = [0; 128];
        for i in 0..vc.len() {
            rtrn[i] = vc[i];
        }
        rtrn
    }
}

impl Into<Packet> for RawPacket {
    fn into(self) -> Packet {
        Packet {
            number: self[0],
            total: self[1],
            information: StackVec::from_slice(&self[2..]).unwrap(),
        }
    }
}

/// Message is a wrapper around a String and a Vec of Packets
/// the Vec of Packets is created from the String when the message is created, or updated.
#[derive(Debug, Clone)]
pub struct Message {
    msg: String,
    pub packets: Vec<Packet>,
}

impl Message {
    pub fn new(msg: String) -> Self {
        let packets = Message::packets_from_string(msg.clone());
        Message { msg, packets }
    }

    pub fn update(&mut self, msg: String) {
        self.msg = msg.clone();
        self.packets = Message::packets_from_string(msg);
    }

    fn packets_from_string(msg: String) -> Vec<Packet> {
        let len = match msg.len() {
            0 => 0,
            _ => (msg.len() / 126) + 1,
        };

        let mut packets = Vec::new();
        for i in 0..len {
            let byts = msg.as_bytes();
            let start = i * 126;
            let marker = (i + 1) * 126;
            let end = if marker > msg.len() {
                msg.len()
            } else {
                marker
            };
            println!("start: {}, marker: {}, end: {}", start, marker, end);
            let slc: &[u8] = &byts[start..end];
            let info: StackVec<u8, 126> = StackVec::from_slice(slc).unwrap();
            let packet = Packet::new((i + 1) as u8, len as u8, info);
            packets.push(packet);
        }
        println!("packets: {:?}", packets);

        packets
    }

    fn string_from_packets(packets: &Vec<Packet>) -> String {
        let mut rtrn = String::from("");
        for packet in packets {
            rtrn.push_str(&String::from_utf8_lossy(&packet.information).to_string());
        }
        rtrn
    }
}

impl From<Packet> for Message {
    fn from(value: Packet) -> Self {
        let mut packets = Vec::new();
        packets.push(value);
        Message {
            msg: Message::string_from_packets(&packets),
            packets,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_from_rawpacket() {
        let rawpacket: RawPacket = [1; 128];
        let packet_expect = Packet {
            number: 1u8,
            total: 1u8,
            information: StackVec::from_slice(&[1u8; 126]).unwrap(),
        };
        let rawpacket_into: Packet = rawpacket.into();
        assert_eq!(packet_expect.number, rawpacket_into.number);
        assert_eq!(packet_expect.total, rawpacket_into.total);
        assert_eq!(packet_expect.information, rawpacket_into.information);
    }

    #[test]
    fn test_rawpacket_from_packet() {
        let packet = Packet {
            number: 1u8,
            total: 1u8,
            information: StackVec::from_slice(&[1u8; 126]).unwrap(),
        };
        let rawpacket_expect: RawPacket = [1; 128];

        let packet_into: RawPacket = packet.into();
        assert_eq!(rawpacket_expect, packet_into);
    }

    #[test]
    fn test_message_from_string() {
        let msg = String::from("Hello, World hope you're listening. I think that they can see, a better side of me. Let's keep talking about this to help with the frustration of not remembering the lyrics.");
        let message = Message::new(msg);
        println!("Message: {:?}", message);
        assert_eq!(message.packets.len(), 2);
        assert_eq!(message.packets[0].number, 1);
        assert_eq!(message.packets[0].total, 2);
        assert_eq!(
            message.packets[0].information,
            StackVec::<u8, 126>::from_slice(&[
                72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 32, 104, 111, 112, 101, 32,
                121, 111, 117, 39, 114, 101, 32, 108, 105, 115, 116, 101, 110, 105, 110, 103, 46,
                32, 73, 32, 116, 104, 105, 110, 107, 32, 116, 104, 97, 116, 32, 116, 104, 101, 121,
                32, 99, 97, 110, 32, 115, 101, 101, 44, 32, 97, 32, 98, 101, 116, 116, 101, 114,
                32, 115, 105, 100, 101, 32, 111, 102, 32, 109, 101, 46, 32, 76, 101, 116, 39, 115,
                32, 107, 101, 101, 112, 32, 116, 97, 108, 107, 105, 110, 103, 32, 97, 98, 111, 117,
                116, 32, 116, 104, 105, 115, 32, 116, 111, 32, 104, 101, 108, 112, 32, 119, 105,
                116, 104
            ])
            .unwrap()
        );
        assert_eq!(message.packets[1].number, 2);
        assert_eq!(message.packets[1].total, 2);
        assert_eq!(
            message.packets[1].information,
            StackVec::<u8, 126>::from_slice(&[
                32, 116, 104, 101, 32, 102, 114, 117, 115, 116, 114, 97, 116, 105, 111, 110, 32,
                111, 102, 32, 110, 111, 116, 32, 114, 101, 109, 101, 109, 98, 101, 114, 105, 110,
                103, 32, 116, 104, 101, 32, 108, 121, 114, 105, 99, 115, 46
            ])
            .unwrap()
        );
    }
}
