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
    pub fn set(&mut self, index: usize, key: String, value: String) {
        if index >= self.len() {
            return;
        }
        self.raw[index] = format!("{}: {}", key, value);
    }

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

        return Header {
            charset: "utf8".to_owned(),
            key: key.to_owned(),
            value: value.to_owned(),
            bytes: raw_header.as_bytes().to_vec(),
            error: "".to_owned(),
        };
    }
}
