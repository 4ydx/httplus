mod errors;
mod headers;

#[derive(Debug, Clone, Default)]
pub struct Request<'a> {
    pub request_line: String,
    pub headers: headers::Headers,
    pub headers_end: usize,
    pub raw: Vec<u8>,
    pub parsing_errors: Vec<errors::Errors<'a>>,
    pub content_length: usize,
    pub is_chunked: bool,
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

impl Request<'_> {
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
                if at > HEADER_END.len() {
                    // raw data might come in that splits the HEADER_END in two:
                    //
                    // EG:
                    //  previous append to raw: "\r"
                    //  next append to raw: "\n\r\n"
                    //
                    // as a result, backup enough to find a complete HEADER_END
                    self.headers_end = at - HEADER_END.len();
                }
                at += 1;
            }
        }
    }

    // Below newline_indices starts with the first instance of a newline in the raw data.
    // As a result the initial HTTP line (example: GET / HTTP/1.1) is automatically skipped.
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
        let mut at = newline.next().unwrap(); // starting at the very first newline past
                                              // the, for example, 'Get / HTTP/x.x' line

        match String::from_utf8(header_chunk[0..*at].to_owned()) {
            // TODO: check that the first line of the HTTP request is valid
            Ok(s) => self.request_line = s,
            Err(e) => self.parsing_errors.push(errors::Errors::Parse(e)),
        };

        loop {
            let mut eindex = match newline.next() {
                Some(eindex) => eindex,
                None => break,
            };

            let sindex = at + LINE_END.len();
            let mut skip: Vec<usize> = vec![sindex, *eindex]; // the first line of a header should
                                                              // never be a "line folded" header
            loop {
                if eindex == &header_chunk.len() {
                    break;
                }

                /*
                  https://www.rfc-editor.org/rfc/rfc7230

                  A proxy or gateway that receives an obs-fold in a response message
                  that is not within a message/http container MUST either discard the
                  message and replace it with a 502 (Bad Gateway) response, preferably
                  with a representation explaining that unacceptable line folding was
                  received, or replace each received obs-fold with one or more SP
                  octets prior to interpreting the field value or forwarding the
                  message downstream.

                  https://www.ietf.org/rfc/rfc2616.txt

                  All linear white space, including folding, has the same semantics as SP. A
                  recipient MAY replace any linear white space with a single SP before
                  interpreting the field value or forwarding the message downstream.

                  LWS            = [CRLF] 1*( SP | HT )

                  In other words, one or more spaces or tabs must be replaced with a single space.
                */

                // evaluate the first byte(s) in the next line
                // to determine if we are dealing with a "line folded" header
                let mut offset = 0;
                let mut skipping = false;

                let mut next_non_empty_char = header_chunk[eindex + LINE_END.len() + offset];
                while next_non_empty_char == b'\t' || next_non_empty_char == b' ' {
                    offset += 1;
                    next_non_empty_char = header_chunk[eindex + LINE_END.len() + offset];
                    skipping = true;
                }

                if skipping {
                    let sindex = eindex + LINE_END.len() + offset;
                    eindex = match newline.next() {
                        Some(eindex) => eindex,
                        None => break,
                    };
                    skip.push(sindex);
                    skip.push(*eindex);
                } else {
                    break;
                }
            }

            // reduce spaces and tabs in "line folded" headers to a single space
            let mut header: Vec<u8> = vec![];
            for i in 0..skip.len() {
                if i % 2 == 1 {
                    continue;
                }
                let mut chunk = header_chunk[skip[i]..skip[i + 1]].to_owned();
                header.append(&mut chunk);
            }

            match String::from_utf8(header.to_owned()) {
                Ok(s) => self.headers.raw.push(s),
                Err(e) => self.parsing_errors.push(errors::Errors::Parse(e)),
            }
            at = eindex;

            let header = self.headers.at(self.headers.raw.len() - 1);

            // check most recent header to see if it contains content-length
            if header.key.to_lowercase() == "content-length" && header.error.len() == 0 {
                if self.is_chunked {
                    self.parsing_errors.push(errors::Errors::Header(
                        "Transfer-Encoding and Content-Length headers mutually exclusive",
                    ));
                } else {
                    self.content_length = match header.value.trim().parse::<usize>() {
                        Ok(i) => i,
                        Err(e) => {
                            self.parsing_errors.push(errors::Errors::ContentLength(e));
                            0
                        }
                    };
                }
            }

            // check for chunked state: Transfer-Encoding: gzip, chunked
            if header.key.to_lowercase() == "transfer-encoding" && header.error.len() == 0 {
                if self.content_length > 0 {
                    self.parsing_errors.push(errors::Errors::Header(
                        "Transfer-Encoding and Content-Length headers are mutually exclusive",
                    ));
                } else {
                    self.is_chunked = header.value.ends_with("chunked");
                }
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
            &mut "GET / HTTP/1.1\r\nFirst: wrapp\r\n   ing\r\n\ttest\r\nSecond: wrapp\r\n    ing\r\n\ttest\r\n\r\n"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(r.headers.raw[0], "First: wrappingtest");
        assert_eq!(r.headers.raw[1], "Second: wrappingtest");
        assert_eq!(r.body_complete(), true);
    }

    #[test]
    fn test_mutually_exclusive() {
        let mut r = Request::default();
        r.update_raw(
            &mut "GET / HTTP/1.1\r\nContent-Length: 10\r\nTransfer-Encoding: chunked\r\n\r\n"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(r.parsing_errors.len(), 1);
        assert_eq!(
            r.parsing_errors.pop(),
            Some(errors::Errors::Header(
                "Transfer-Encoding and Content-Length headers are mutually exclusive",
            ))
        );
        assert_eq!(r.body_complete(), false);
    }

    #[test]
    fn test_body() {
        let mut r = Request::default();
        r.update_raw(&mut "POST TEST\r".as_bytes().to_vec());
        r.update_raw(&mut "\nContent-L".as_bytes().to_vec());
        r.update_raw(&mut "ength: 4\r\n".as_bytes().to_vec());
        r.update_raw(&mut "\r\nBODY".as_bytes().to_vec());
        assert_eq!(r.headers.raw[0], "Content-Length: 4");
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
        assert_eq!(r.headers.at(0).value, "updated-wrap");

        /*
        match String::from_utf8(r.dump()) {
            Ok(s) => println!("{}", s),
            Err(e) => panic!("{}", e),
        }
        */
    }
}
