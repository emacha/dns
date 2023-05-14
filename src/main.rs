use rand::Rng;
use std::collections::{HashMap, VecDeque};
use std::net::UdpSocket;
use std::{str, println};

// When the first 2 bits are 1, the domain name is compressed.
// Check by ANDing with 0b11000000
fn is_compressed(byte: usize) -> bool {
    return byte & 0b11000000 == 0b11000000
}


#[derive(Debug)]
struct DNSHeader {
    id: u16,
    flags: u16,
    num_questions: u16,
    num_answers: u16,
    num_authorities: u16,
    num_additionals: u16
}

impl DNSHeader {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.to_be_bytes());
        bytes.extend_from_slice(&self.flags.to_be_bytes());
        bytes.extend_from_slice(&self.num_questions.to_be_bytes());
        bytes.extend_from_slice(&self.num_answers.to_be_bytes());
        bytes.extend_from_slice(&self.num_authorities.to_be_bytes());
        bytes.extend_from_slice(&self.num_additionals.to_be_bytes());
        return bytes
    }

    fn from_buffer(buffer: &mut VecDeque<u8>, idx: &mut u16) -> DNSHeader {
        let header_vals: Vec<u16> = buffer.drain(0..12).collect::<Vec<u8>>().chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        *idx += 12;

        DNSHeader {
            id: header_vals[0],
            flags: header_vals[1],
            num_questions: header_vals[2],
            num_answers: header_vals[3],
            num_authorities: header_vals[4],
            num_additionals: header_vals[5]
        }
    }
}

#[derive(Debug)]
struct DNSQuestion {
    name: String,
    type_: u16,
    class: u16
}

impl DNSQuestion {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&encode_dns_name(&self.name));
        bytes.extend_from_slice(&self.type_.to_be_bytes());
        bytes.extend_from_slice(&self.class.to_be_bytes());
        return bytes
    }

    fn from_buffer(buffer: &mut VecDeque<u8>, idx: &mut u16, decoded_names: &mut HashMap<u16, String>) -> DNSQuestion {
        let name = decode_name(buffer, idx, decoded_names);

        let question_vals: Vec<u16> = buffer.drain(0..4).collect::<Vec<u8>>().chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();
        *idx += 4;

        DNSQuestion {
            name,
            type_: question_vals[0],
            class: question_vals[1]
        }
    }

}

#[derive(Debug)]
struct DNSRecord {
    name: String,
    type_: u16,
    class: u16,
    ttl: u32,
    data: Vec<u8>
}

impl DNSRecord {
    fn from_buffer(buffer: &mut VecDeque<u8>, idx: &mut u16, decoded_names: &mut HashMap<u16, String>) -> DNSRecord {
        let name = decode_name(buffer, idx, decoded_names);

        let type_ = u16::from_be_bytes([buffer.pop_front().unwrap(), buffer.pop_front().unwrap()]);
        let class = u16::from_be_bytes([buffer.pop_front().unwrap(), buffer.pop_front().unwrap()]);
        let ttl = u32::from_be_bytes([buffer.pop_front().unwrap(), buffer.pop_front().unwrap(), buffer.pop_front().unwrap(), buffer.pop_front().unwrap()]);
        let data_len = u16::from_be_bytes([buffer.pop_front().unwrap(), buffer.pop_front().unwrap()]);
        *idx += 10;

        let data = buffer.drain(0..data_len as usize).collect();
        *idx += data_len;

        DNSRecord {
            name,
            type_,
            class,
            ttl,
            data
        }
    }
}

#[derive(Debug)]
struct DNSPacket {
    header: DNSHeader,
    questions: Vec<DNSQuestion>,
    answers: Vec<DNSRecord>,
    authorities: Vec<DNSRecord>,
    additionals: Vec<DNSRecord>
}

impl DNSPacket {
    fn from_buffer(buffer: Vec<u8>) -> DNSPacket {
        let mut response = VecDeque::from(buffer);
        let mut idx = 0;
        let mut decoded_names: HashMap<u16, String> = HashMap::new();

        let header = DNSHeader::from_buffer(&mut response, &mut idx);

        let mut questions: Vec<DNSQuestion> = Vec::new();
        for _ in 0..header.num_questions {
            questions.push(DNSQuestion::from_buffer(&mut response, &mut idx, &mut decoded_names));
        }

        let mut answers: Vec<DNSRecord> = Vec::new();
        for _ in 0..header.num_answers {
            answers.push(DNSRecord::from_buffer(&mut response, &mut idx, &mut decoded_names));
        }

        let mut authorities: Vec<DNSRecord> = Vec::new();
        for _ in 0..header.num_authorities {
            authorities.push(DNSRecord::from_buffer(&mut response, &mut idx, &mut decoded_names));
        }

        let mut additionals: Vec<DNSRecord> = Vec::new();
        for _ in 0..header.num_additionals {
            additionals.push(DNSRecord::from_buffer(&mut response, &mut idx, &mut decoded_names));
        }

        DNSPacket { header, questions, answers, authorities, additionals }
    }
}

fn decode_name(buffer: &mut VecDeque<u8>, idx: &mut u16, decoded_names: &mut HashMap<u16, String>) -> String {
    let name_idx = *idx as u16;

    let mut len = buffer.pop_front().unwrap() as usize;
    let name: String;
    *idx += 1;
    if is_compressed(len) {
        let pointer = u16::from_be_bytes([(len & 0b0011_1111) as u8, buffer.pop_front().unwrap()]);
        *idx += 1;
        name = decoded_names.get(&pointer).unwrap().clone()
    } else {
        let mut name_parts = Vec::new();
        loop {
            let part = buffer.drain(0..len).collect::<Vec<u8>>();
            let part = String::from_utf8(part).expect("Found invalid UTF-8");
            name_parts.push(part);
            *idx += len as u16;

            len = buffer.pop_front().unwrap() as usize;
            *idx += 1;
            if len == 0 {
                name = name_parts.join(".");
                decoded_names.insert(name_idx, name.clone());
                break;
            }
        }
    }
    name
}

fn encode_dns_name(name: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let parts: Vec<&str> = name.split('.').collect();
    for part in parts {
        bytes.push(part.len() as u8);
        bytes.extend_from_slice(part.as_bytes());
    }
    bytes.push(0);
    return bytes
}

// Hardcode record type to A and u16. Should be a enum
// Also hardcode class to IN.
fn build_query(domain_name: String, record_type: u16) -> Vec<u8> {
    let id = rand::thread_rng().gen_range(0..65535);
    let recursion_desired = 1 << 8;
    let header = DNSHeader {
        id: id,
        flags: recursion_desired,
        num_questions: 1,
        num_answers: 0,
        num_authorities: 0,
        num_additionals: 0
    };
    let question = DNSQuestion {
        name: domain_name,
        type_: record_type,
        class: 1
    };

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&header.to_bytes());
    bytes.extend_from_slice(&question.to_bytes());
    return bytes
}


fn main() -> std::io::Result<()>  {
    let query = build_query("www.example.com".to_string(), 1);
    let socket = UdpSocket::bind("0.0.0.0:0")?; // bind to a random available port

    socket.send_to(&query, "8.8.8.8:53")?; // replace with the actual target IP address

    let mut buffer = [0u8; 1024]; // create a buffer to hold the response
    let (amt, _) = socket.recv_from(&mut buffer)?;

    // buffer bytes up to the amount received
    let response = buffer[..amt].to_vec();
    let packet = DNSPacket::from_buffer(response);
    println!("packet: {:?}", packet);

    let raw_ip = &packet.answers[0].data;
    let ip: String = raw_ip[0..4]
    .iter()
    .map(|byte| byte.to_string())
    .collect::<Vec<String>>()
    .join(".");

    println!("ip: {}", ip);

    Ok(())
}


