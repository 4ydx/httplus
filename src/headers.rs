#[derive(Debug, Default)]
pub struct Headers {
    pub raw: Vec<String>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Header {
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
                key: "".to_owned(),
                value: "".to_owned(),
                bytes: vec![],
                error: "index out of bounds".to_owned(),
            };
        }

        let raw_header: &str = &self.raw[index];
        let length = raw_header.len();

        let mut key: &str = "";
        let mut value: &str = "";

        for i in 0..length {
            let byte = raw_header.as_bytes()[i];

            if byte > 127 {
                return Header {
                    key: "".to_owned(),
                    value: "".to_owned(),
                    bytes: raw_header.as_bytes().to_vec(),
                    error: format!("non-ascii byte found at index {}", i).to_owned(),
                };
            }

            if key.len() > 0 {
                // trim leading whitespace
                if value.len() == 0 && byte != b' ' {
                    value = &raw_header[i..];
                }
            }

            if byte == b':' {
                key = &raw_header[0..i];
            }
        }

        // an empty key implies a "bad" header value
        if key.len() == 0 {
            return Header {
                key: key.to_owned(),
                value: value.to_owned(),
                bytes: raw_header.as_bytes().to_vec(),
                error: "header key/value pair not found".to_owned(),
            };
        }

        return Header {
            key: key.to_owned(),
            value: value.to_owned(),
            bytes: raw_header.as_bytes().to_vec(),
            error: "".to_owned(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_ascii() {
        let h = Headers {
            raw: vec!["foo: bar".to_owned(), "foo: bär".to_owned()],
        };
        assert_eq!(
            Header {
                bytes: "foo: bar".as_bytes().to_vec(),
                key: "foo".to_owned(),
                value: "bar".to_owned(),
                error: "".to_owned(),
            },
            h.at(0)
        );
        assert_eq!(
            Header {
                bytes: "foo: bär".as_bytes().to_vec(),
                key: "".to_owned(),
                value: "".to_owned(),
                error: "non-ascii byte found at index 6".to_owned(),
            },
            h.at(1)
        );
    }
}
