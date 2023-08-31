mod headers;

#[derive(Debug, Clone, Default)]
pub struct Request {
    pub request_line: String,
    pub headers: headers::Headers,
    pub headers_end: usize,
    pub content_length: usize,
    pub raw: Vec<u8>,
    pub parsing_errors: Vec<std::string::FromUtf8Error>,
}

const LINE_END: &[u8; 2] = b"\r\n";
const HEADER_END: &[u8; 4] = b"\r\n\r\n";

/*
    https://www.rfc-editor.org/rfc/rfc7230#section-3
    HTTP-message = start-line
                   *( header-field CRLF )
                   CRLF
                   [ message-body ]
*/

impl Request {
    pub fn dump(&self) -> Vec<u8> {
        if !self.body_complete() {
            return vec![];
        }
        let mut dump = vec![];
        dump.append(&mut self.request_line.as_bytes().to_vec());
        dump.append(&mut LINE_END.to_vec());
        dump.append(&mut self.headers.raw.join("\r\n").as_bytes().to_vec());
        dump.append(&mut HEADER_END.to_vec());
        if self.body_complete() {
            dump.append(&mut self.body());
        }
        dump
    }

    pub fn body(&self) -> Vec<u8> {
        if !self.body_complete() {
            return vec![];
        }
        self.raw[self.headers_end + HEADER_END.len()..].to_vec()
    }

    pub fn body_complete(&self) -> bool {
        if self.headers_end + HEADER_END.len() > self.raw.len() {
            return false;
        }
        self.raw[self.headers_end + HEADER_END.len()..].len() == self.content_length
    }

    pub fn update_raw(&mut self, data: &mut Vec<u8>) {
        self.raw.append(data);

        if self.headers.raw.is_empty() {
            let mut at = self.headers_end;

            while at < self.raw.len() {
                if self.raw[at..].starts_with(HEADER_END) {
                    self.headers_end = at;
                    self.parse_and_fill_headers();
                    break;
                }
                self.headers_end = at;
                at += 1;
            }
        }
    }

    // The initial HTTP line (example: GET / HTTP/1.1) is skipped since the first
    // newline_indices entry is the first instance of \r\n in the self.raw data field.
    // The first instance is, of course, at the end of the 'GET / HTTP/1.1' line.
    fn parse_and_fill_headers(&mut self) {
        let header_chunk = self.raw[0..self.headers_end].to_vec();

        let mut newline_indices = header_chunk
            .windows(2)
            .enumerate()
            .filter(|(_, w)| w == LINE_END)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        newline_indices.push(header_chunk.len());

        let mut newline = newline_indices.iter();
        let mut at = newline.next().unwrap();

        match String::from_utf8(header_chunk[0..*at].to_owned()) {
            Ok(s) => self.request_line = s,
            Err(e) => self.parsing_errors.push(e),
        };

        loop {
            let sindex = at + LINE_END.len();
            let mut eindex = match newline.next() {
                Some(eindex) => eindex,
                None => break,
            };

            let mut skip: Vec<usize> = vec![sindex];
            loop {
                if eindex == &header_chunk.len() {
                    break;
                }

                // evaluate the first byte in the next line
                // to determine if we are dealing with a multi-line header
                let next_byte = header_chunk[eindex + LINE_END.len()];
                if next_byte == b'\t' || next_byte == b' ' {
                    skip.push(*eindex);
                    skip.push(eindex + LINE_END.len() + 1);

                    eindex = match newline.next() {
                        Some(eindex) => eindex,
                        None => break,
                    };
                } else {
                    break;
                }
            }
            skip.push(*eindex);

            // remove spaces in multi-line headers
            let mut header: Vec<u8> = vec![];
            for i in 0..skip.len() - 1 {
                if i % 2 == 1 {
                    continue;
                }
                let mut chunk = header_chunk[skip[i]..skip[i + 1]].to_owned();
                header.append(&mut chunk);
            }

            match String::from_utf8(header.to_owned()) {
                Ok(s) => self.headers.raw.push(s),
                Err(e) => self.parsing_errors.push(e),
            }
            at = eindex;

            // check most recent header to see if it contains content-length
            let header = self.headers.at(self.headers.raw.len() - 1);
            if header.key == "Content-Length" && header.error.len() == 0 {
                self.content_length = match header.value.trim().parse::<usize>() {
                    Ok(i) => i,
                    Err(_) => 0,
                };
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
        r.update_raw(
            &mut "POST / HTTP/1.1\r\nContent-Length: 4\r\nHere: here\r\n\r\nBODY"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(r.headers.raw[0], "Content-Length: 4");
        assert_eq!(r.headers.raw[1], "Here: here");
        assert_eq!(r.headers.raw.len(), 2);
        assert_eq!(r.content_length, 4);
        assert_eq!(r.parsing_errors.len(), 0);
        assert_eq!(r.body(), vec![b'B', b'O', b'D', b'Y']);
        assert_eq!(r.body_complete(), true);
    }

    #[test]
    fn test_body_incomplete() {
        let mut r = Request::default();
        r.update_raw(
            &mut "POST / HTTP/1.1\r\nContent-Length: 5\r\nHere: here\r\n\r\nBODY"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(r.headers.raw[0], "Content-Length: 5");
        assert_eq!(r.headers.raw[1], "Here: here");
        assert_eq!(r.headers.raw.len(), 2);
        assert_eq!(r.content_length, 5);
        assert_eq!(r.body_complete(), false);

        r.update_raw(&mut "S".as_bytes().to_vec());
        assert_eq!(r.body_complete(), true);
    }

    #[test]
    fn test_content_length_zero() {
        let mut r = Request::default();
        r.update_raw(&mut "GET / HTTP/1.1\r\nHere: here\r\n".as_bytes().to_vec());
        assert_eq!(r.body_complete(), false);

        r.update_raw(&mut "More: more\r\nFinal: final\r\n\r\n".as_bytes().to_vec());
        assert_eq!(r.headers.raw[0], "Here: here");
        assert_eq!(r.headers.raw[1], "More: more");
        assert_eq!(r.headers.raw[2], "Final: final");
        assert_eq!(r.headers.raw.len(), 3);
        assert_eq!(r.content_length, 0);
        assert_eq!(r.parsing_errors.len(), 0);
        assert_eq!(r.body_complete(), true);
    }

    #[test]
    fn test_multi_line_header() {
        let mut r = Request::default();
        r.update_raw(
            &mut "GET / HTTP/1.1\r\nWrapping: wrapp\r\n ing\r\n\ttest\r\nAnother: a\r\n\r\n"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(r.headers.raw[0], "Wrapping: wrappingtest");
        assert_eq!(r.headers.raw[1], "Another: a");
        assert_eq!(r.body_complete(), true);
    }

    #[test]
    fn test_post_edit_dump() {
        let mut r = Request::default();
        r.update_raw(
            &mut "GET / HTTP/1.1\r\nWrapping: wrapp\r\n ing\r\n\ttest\r\nAnother: a\r\nContent-Length: 7\r\n\r\nTHE END"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(r.headers.raw[0], "Wrapping: wrappingtest");
        assert_eq!(r.headers.raw[1], "Another: a");
        assert_eq!(r.body_complete(), true);

        let h = r.headers.at(0);
        r.headers.set(0, h.key, "updated-wrap".to_string());
        match String::from_utf8(r.dump()) {
            Ok(s) => println!("{}", s),
            Err(e) => panic!("{}", e),
        }
    }
}
