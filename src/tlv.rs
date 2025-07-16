#[derive(Debug)]
pub struct Tlv {
    pub t: u16,
    pub v: u32,
}

pub fn parse_tlv(buf: &[u8]) -> Vec<Tlv> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < buf.len() {
        // Check if header (2 bytes) is available
        if i + 2 > buf.len() {
            break;
        }

        let b0 = buf[i];
        let b1 = buf[i + 1];

        // Extract tag (10 bits: 8 from b0 and 2 from b1)
        let tag = (u16::from(b0) << 2) | (u16::from(b1) >> 6);
        // Extract length indicator (2 bits)
        let length_field = (b1 >> 4) & 0x03;
        let value_bytes = length_field as usize;

        // Check if value bytes are available
        if i + 2 + value_bytes > buf.len() {
            break;
        }

        // Extract value (4 bits from header or additional bytes)
        let value = if value_bytes == 0 {
            u32::from(b1 & 0x0F)
        } else {
            let mut v = 0;
            for j in 0..value_bytes {
                v = (v << 8) | u32::from(buf[i + 2 + j]);
            }
            v
        };

        result.push(Tlv { t: tag, v: value });
        i += 2 + value_bytes;
    }

    result
}

pub fn build_tlv(elements: &[Tlv]) -> Vec<u8> {
    let mut out = Vec::new();

    for el in elements {
        let t0 = ((el.t >> 2) & 0xFF) as u8;
        out.push(t0);

        let tl = ((el.t & 3) << 6) as u8;

        if el.v < 0x10 {
            out.push(tl | el.v as u8);
        } else if el.v < 0x100 {
            out.push(tl | 0x10);
            out.push(el.v as u8);
        } else if el.v < 0x10000 {
            out.push(tl | 0x20);
            out.push((el.v >> 8) as u8);
            out.push((el.v & 0xFF) as u8);
        } else {
            out.push(tl | 0x30);
            out.push((el.v >> 16) as u8);
            out.push((el.v >> 8) as u8);
            out.push((el.v & 0xFF) as u8);
        }
    }

    out
}
