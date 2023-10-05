use crate::errors::Errors;
use std::fmt;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Header {
    pub key: String,
    pub value: String,
    pub bytes: Vec<u8>,
}

impl Header {
    pub fn new(raw: Vec<u8>) -> Result<Self, Errors<'static>> {
        let v = match String::from_utf8(raw.to_owned()) {
            Ok(s) => Ok(s),
            Err(e) => Err(Errors::HeaderFromUtf8(e)),
        }?;

        let mut key: &[u8] = &[];
        let mut value: &[u8] = &[];

        for i in 0..raw.len() {
            let byte = raw[i];

            if byte > 127 {
                return Err(Errors::HeaderNonAsciiByteAt(i));
            }
            if key.len() > 0 {
                // trim value's leading whitespace
                if value.len() == 0 && byte != b' ' {
                    value = &raw[i..];
                }
            } else {
                if byte == b':' {
                    key = &raw[0..i];
                    if key.len() == 0 {
                        return Err(Errors::HeaderIsEmpty);
                    }
                } else if byte == b' ' || byte == b'\t' {
                    /*
                        https://datatracker.ietf.org/doc/html/rfc7230#section-3.2.4

                        No whitespace is allowed between the header field-name and colon.  In
                        the past, differences in the handling of such whitespace have led to
                        security vulnerabilities in request routing and response handling.  A
                        server MUST reject any received request message that contains
                        whitespace between a header field-name and colon with a response code
                        of 400 (Bad Request).
                    */
                    return Err(Errors::HeaderKeyWhitespace);
                }
            }
        }

        let key = match String::from_utf8(key.to_owned()) {
            Ok(s) => Ok(s),
            Err(e) => Err(Errors::HeaderFromUtf8(e)),
        }?;

        let value = match String::from_utf8(value.to_owned()) {
            Ok(s) => Ok(s),
            Err(e) => Err(Errors::HeaderFromUtf8(e)),
        }?;
        println!("     '{}'", v);
        println!("key: '{}'\nval: '{}'", key, value);

        Ok(Header {
            key: key.to_owned(),
            value: value.to_owned(),
            bytes: raw.to_vec(),
        })
    }
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.key, self.value)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Headers {
    pub values: Vec<Header>,
}

impl Headers {
    pub fn add(&mut self, key: String, value: String) -> Result<(), Errors<'static>> {
        let h = Header::new(format!("{}: {}", key, value).as_bytes().to_vec())?;
        self.values.push(h);
        Ok(())
    }

    pub fn set(&mut self, index: usize, key: String, value: String) -> Result<(), Errors<'static>> {
        if index >= self.len() {
            return Err(Errors::HeaderIndexOutOfBounds);
        }
        let h = Header::new(format!("{}: {}", key, value).as_bytes().to_vec())?;
        self.values[index] = h;
        Ok(())
    }

    pub fn at(&self, index: usize) -> Result<Header, Errors> {
        if index >= self.len() {
            return Err(Errors::HeaderIndexOutOfBounds);
        }
        return Ok(self.values[index].clone());
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_ascii() {
        let h = Header::new("foo: bär".as_bytes().to_vec());
        assert_eq!(Err(Errors::HeaderNonAsciiByteAt(6)), h);
    }

    #[test]
    fn test_whitespace_header_key() {
        let h = Header::new("fo o: bar".as_bytes().to_vec());
        assert_eq!(Err(Errors::HeaderKeyWhitespace), h);
    }

    #[test]
    fn test_empty_header_key() {
        let mut h = Headers { values: vec![] };
        let r = h.add("".to_owned(), "B".to_owned());
        assert_eq!(Err(Errors::HeaderIsEmpty), r);
    }

    #[test]
    fn test_index_out_of_bounds() {
        let mut h = Headers { values: vec![] };
        assert_eq!(Err(Errors::HeaderIndexOutOfBounds), h.at(0));
        h.add("A".to_owned(), "B".to_owned()).unwrap();
        assert_eq!(Err(Errors::HeaderIndexOutOfBounds), h.at(1));
    }
}
