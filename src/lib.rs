#[derive(Debug, Default)]
pub struct Request {
    pub headers: Vec<String>,
    pub headers_end: usize,
    pub content_length: usize,
    pub raw: Vec<u8>,
    pub header_errors: Vec<std::string::FromUtf8Error>,
}

const LINE_END: &[u8; 2] = b"\r\n";
const HEADER_END: &[u8; 4] = b"\r\n\r\n";

impl Request {
    pub fn body(&self) -> Vec<u8> {
        self.raw[self.headers_end..].to_vec()
    }

    pub fn body_complete(&self) -> bool {
        self.raw[self.headers_end..].len() == self.content_length
    }

    pub fn update(&mut self, data: &mut Vec<u8>) {
        self.raw.append(data);

        if self.headers.is_empty() {
            let mut at = self.headers_end;

            let mut found_header_end = false;
            while at < self.raw.len() {
                if self.raw[at..].starts_with(HEADER_END) {
                    found_header_end = true;
                    self.headers_end = at + HEADER_END.len();

                    // 'header_newlines' contains the "\r\n" indices where newlines occur
                    let header_chunk = self.raw[0..at].to_vec();
                    let mut header_newlines = header_chunk
                        .windows(2)
                        .enumerate()
                        .filter(|(_, w)| w == b"\r\n")
                        .map(|(i, _)| i)
                        .collect::<Vec<_>>();
                    header_newlines.push(header_chunk.len());

                    // print!("{:#?}", values);

                    // This first entry in header_newlines skips the HTTP version line
                    let mut header_start = header_newlines[0] + LINE_END.len();
                    for header_end in header_newlines[1..].iter() {
                        match String::from_utf8(
                            header_chunk[header_start..header_end.clone()].to_owned(),
                        ) {
                            Ok(s) => self.headers.push(s),
                            Err(e) => self.header_errors.push(e),
                        }

                        if self.raw[header_start..header_end.clone()]
                            .starts_with(b"Content-Length:")
                        {
                            let mut v = vec![];
                            let mut collect = false;
                            for val in self.raw[header_start..header_end.clone()].to_vec().iter() {
                                if collect {
                                    v.push(*val);
                                }
                                if val == &b':' {
                                    collect = true;
                                }
                            }
                            let num = match String::from_utf8(v) {
                                Ok(s) => s,
                                Err(e) => {
                                    self.header_errors.push(e);
                                    "0".to_owned()
                                }
                            };

                            self.content_length = match num.trim().parse::<usize>() {
                                Ok(i) => i,
                                Err(_) => 0,
                            };
                        }
                        header_start = header_end + LINE_END.len();
                    }
                }
                at += 1;
            }
            if !found_header_end {
                self.headers_end = at;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_length() {
        let mut r = Request::default();
        r.update(
            &mut "GET / HTTP/1.1\r\nContent-Length: 4\r\nHere: here\r\n\r\nBODY"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(r.content_length, 4);
        assert_eq!(r.header_errors.len(), 0);
        assert_eq!(r.body(), vec![b'B', b'O', b'D', b'Y']);
        assert_eq!(r.body_complete(), true);
    }

    #[test]
    fn test_body_incomplete() {
        let mut r = Request::default();
        r.update(
            &mut "GET / HTTP/1.1\r\nContent-Length: 5\r\nHere: here\r\n\r\nBODY"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(r.content_length, 5);
        assert_eq!(r.body_complete(), false);

        r.update(&mut "S".as_bytes().to_vec());
        assert_eq!(r.body_complete(), true);
    }

    #[test]
    fn test_content_length_zero() {
        let mut r = Request::default();
        r.update(&mut "GET / HTTP/1.1\r\nHere: here\r\n".as_bytes().to_vec());
        r.update(&mut "More: more\r\nFinal: final\r\n\r\n".as_bytes().to_vec());
        assert_eq!(r.content_length, 0);
        assert_eq!(r.header_errors.len(), 0);
        assert_eq!(r.body_complete(), true);
    }
}
