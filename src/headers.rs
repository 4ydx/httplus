use base64::{engine::general_purpose, Engine as _};
use encoding::label::encoding_from_whatwg_label;
use encoding::DecoderTrap;

#[derive(Debug, Default)]
pub struct Headers {
    pub raw: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Header {
    pub charset: String,
    pub key: String,
    pub value: String,
    pub bytes: Vec<u8>,
    pub error: String,
}

impl Headers {
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    pub fn at(&self, index: usize) -> Header {
        if index >= self.len() {
            return Header {
                charset: "utf8".to_owned(),
                key: "".to_owned(),
                value: "".to_owned(),
                bytes: vec![],
                error: "index out of bounds".to_owned(),
            };
        }

        let raw_header: &str = &self.raw[index].to_owned();
        let length = raw_header.len();

        let mut key: &str = "";
        let mut value: &str = "";

        for i in 0..length {
            if key.len() > 0 {
                if value.len() == 0 && raw_header.as_bytes()[i] != b' ' {
                    value = &raw_header[i..]; // trim leading whitespace
                }
            }
            if raw_header.as_bytes()[i] == b':' {
                key = &raw_header[0..i];
            }
        }

        // an empty key implies a "bad" header value
        if key.len() == 0 {
            return Header {
                charset: "utf8".to_owned(),
                key: key.to_owned(),
                value: value.to_owned(),
                bytes: raw_header.as_bytes().to_vec(),
                error: "header key/value pair not found".to_owned(),
            };
        }

        // strictly speaking an encoded-word requires a longer string
        // but this is simply to prevent a panic later on in the code
        if value.len() < 2 {
            return Header {
                charset: "utf8".to_owned(),
                key: key.to_owned(),
                value: value.to_owned(),
                bytes: raw_header.as_bytes().to_vec(),
                error: "".to_owned(),
            };
        }

        // encoded-word
        //
        // https://www.rfc-editor.org/rfc/rfc2047#section-2
        // encoded-word = "=?" charset "?" encoding "?" encoded-text "?="
        let value_bytes = value.as_bytes();
        if value_bytes[0] == b'='
            && value_bytes[1] == b'?'
            && value_bytes[value_bytes.len() - 2] == b'?'
            && value_bytes[value_bytes.len() - 1] == b'='
        {
            let mut parts = raw_header.split('?');
            let charset = parts.nth(1).unwrap();
            let encoding = parts.nth(2);
            let encoded_text = parts.nth(3).unwrap();

            let mut header = Header::default();
            header.charset = charset.to_owned();

            let header: Header = match encoding {
                Some("B") => {
                    match &general_purpose::STANDARD_NO_PAD.decode(encoded_text) {
                        Ok(bytes) => header.bytes = bytes.to_vec(),
                        Err(e) => header.error = e.to_string(),
                    };
                    header.key = key.to_owned();

                    match encoding_from_whatwg_label(charset) {
                        Some(enc) => {
                            match enc.decode(&header.bytes, DecoderTrap::Strict) {
                                Ok(s) => header.value = s,
                                Err(e) => header.error = e.to_string(),
                            };
                        }
                        None => {
                            header.error = format!("unsupported charset {}", charset).to_owned()
                        }
                    };

                    header
                }
                // Some("Q") => quoted_printable::decode(encoded_text).unwrap(),
                _ => {
                    header.error = "unsupported encoding".to_owned();
                    header
                }
            };
            return header;
        }

        return Header {
            charset: "utf8".to_owned(),
            key: key.to_owned(),
            value: value.to_owned(),
            bytes: raw_header.as_bytes().to_vec(),
            error: "".to_owned(),
        };
    }
}
