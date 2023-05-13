use rand::Rng;
use std::net::UdpSocket;
use std::str;

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
}


fn to_bytestring(bytes: &Vec<u8>) -> String {
    let byte_string: String = bytes.iter().map(|b| format!("\\x{:02x}", b)).collect();
    return byte_string
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
    let (amt, src) = socket.recv_from(&mut buffer)?;

    println!("Received {} bytes from {}: {:?}", amt, src, &buffer[..amt]);

    Ok(())
}


