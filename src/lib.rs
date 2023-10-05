mod errors;
mod headers;

#[derive(Debug, Clone, Default, PartialEq, PartialOrd)]
pub enum HeadersEnd {
    #[default]
    Unset,
    Scanning(usize),
    FoundAt(usize),
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd)]
pub enum ContentLength {
    #[default]
    Unset,
    Value(usize),
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd)]
pub enum Chunked {
    #[default]
    Unset,
    Processing,
    Complete,
}

#[derive(Debug, Clone, Default)]
pub struct Request {
    pub request_line: String,
    pub headers: headers::Headers,
    pub headers_end: HeadersEnd,
    pub raw: Vec<u8>,
    pub content_length: ContentLength,
    pub is_chunked: Chunked,
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
        dump.append(
            &mut self
                .headers
                .values
                .iter()
                .map(|h| format!("{}: {}", h.key, h.value))
                .collect::<Vec<String>>()
                .join("\r\n")
                .as_bytes()
                .to_vec(),
        );
        dump.append(&mut HEADER_END.to_vec());
        if self.body_complete() {
            dump.append(&mut self.body());
        }
        dump
    }

    pub fn body(&self) -> Vec<u8> {
        match self.headers_end {
            HeadersEnd::FoundAt(at) => self.raw[at + HEADER_END.len()..].to_vec(),
            _ => vec![],
        }
    }

    pub fn body_complete(&self) -> bool {
        match self.headers_end {
            HeadersEnd::Unset => false,
            HeadersEnd::Scanning(_) => false,
            HeadersEnd::FoundAt(at) => {
                match self.is_chunked {
                    Chunked::Unset => false,
                    Chunked::Processing => return false,
                    Chunked::Complete => true,
                };
                match self.content_length {
                    ContentLength::Unset => true,
                    ContentLength::Value(content_length) => {
                        self.raw[at + HEADER_END.len()..].len() == content_length
                    }
                }
            }
        }
    }

    pub fn update_raw(&mut self, data: &mut Vec<u8>) -> Result<(), errors::Errors> {
        self.raw.append(data);

        match self.headers_end {
            HeadersEnd::Unset => self.attempt_header_parsing(0),
            HeadersEnd::Scanning(index) => self.attempt_header_parsing(index),
            HeadersEnd::FoundAt(_) => Ok(()),
        }
    }

    fn attempt_header_parsing(&mut self, mut at: usize) -> Result<(), errors::Errors> {
        while at < self.raw.len() {
            if self.raw[at..].starts_with(HEADER_END) {
                self.headers_end = HeadersEnd::FoundAt(at);
                break;
            }
            at += 1;
        }

        if let HeadersEnd::FoundAt(_) = self.headers_end {
            self.parse_and_fill_headers()?;
        } else {
            // raw data might come in that splits the HEADER_END in two:
            // EG:
            //  previous append to raw: "\r"
            //  next append to raw: "\n\r\n"
            //
            // as a result, backup enough to find a complete HEADER_END
            self.headers_end = HeadersEnd::Scanning(at - HEADER_END.len());
        }
        Ok(())
    }

    fn parse_and_fill_headers(&mut self) -> Result<(), errors::Errors> {
        if let HeadersEnd::FoundAt(end) = self.headers_end {
            let header_chunk = self.raw[0..end].to_vec();

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
                // TODO: check that the first line of the HTTP request is valid
                Ok(s) => self.request_line = s,
                Err(e) => return Err(errors::Errors::Parse(e)),
            };

            loop {
                let sindex = at + LINE_END.len();
                let mut eindex = match newline.next() {
                    Some(eindex) => eindex,
                    None => break,
                };

                let mut skip_fold_spaces: Vec<usize> = vec![sindex, *eindex];

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
                    let mut is_line_fold = false;

                    let mut next_non_empty_char = header_chunk[eindex + LINE_END.len() + offset];
                    while next_non_empty_char == b'\t' || next_non_empty_char == b' ' {
                        offset += 1;
                        next_non_empty_char = header_chunk[eindex + LINE_END.len() + offset];
                        is_line_fold = true;
                    }

                    if is_line_fold {
                        let sindex = eindex + LINE_END.len() + offset;
                        eindex = match newline.next() {
                            Some(eindex) => eindex,
                            None => break,
                        };
                        skip_fold_spaces.push(sindex);
                        skip_fold_spaces.push(*eindex);
                    } else {
                        break;
                    }
                }
                at = eindex;

                // reduce spaces and tabs in "line folded" headers to a single space
                let mut header: Vec<u8> = vec![];
                for i in 0..skip_fold_spaces.len() {
                    if i % 2 == 1 {
                        continue;
                    }
                    let mut chunk =
                        header_chunk[skip_fold_spaces[i]..skip_fold_spaces[i + 1]].to_owned();
                    header.append(&mut chunk);
                }

                let header = headers::Header::new(header)?;
                let key = header.key.to_lowercase();

                if key == "content-length" {
                    match self.content_length {
                        ContentLength::Value(_) => {
                            return Err(errors::Errors::Header(
                                "Content-Length header must appear only once",
                            ))
                        }
                        ContentLength::Unset => {
                            self.content_length = match header.value.trim().parse::<usize>() {
                                Ok(i) => ContentLength::Value(i),
                                Err(e) => return Err(errors::Errors::ContentLength(e)),
                            };
                        }
                    }
                }

                // check for chunked state: Transfer-Encoding: gzip, chunked
                if key == "transfer-encoding" {
                    if header.value.contains("chunked") && !header.value.ends_with("chunked") {
                        return Err(errors::Errors::Header(
                            "chunked must appear at the very end of the Transfer-Encoding header value",
                        ));
                    }
                    if header.value.ends_with("chunked") {
                        match self.is_chunked {
                            Chunked::Processing => {
                                return Err(errors::Errors::Header(
                                    "Transfer-Encoding must appear only once",
                                ))
                            }
                            Chunked::Complete => {
                                return Err(errors::Errors::Header(
                                    "Unexpected chunked status: Complete",
                                ))
                            }
                            Chunked::Unset => {
                                self.is_chunked = Chunked::Processing;
                            }
                        }
                    }
                }

                let content_length_set = match self.content_length {
                    ContentLength::Unset => false,
                    _ => true,
                };
                let is_chunked_set = match self.is_chunked {
                    Chunked::Unset => false,
                    _ => true,
                };
                if content_length_set && is_chunked_set {
                    return Err(errors::Errors::Header(
                        "Transfer-Encoding and Content-Length headers are mutually exclusive",
                    ));
                }

                self.headers.values.push(header.clone());
            }
        } else {
            return Err(errors::Errors::CannotFillHeaders);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /*
    #[test]
    fn test_chunked() {
        // TODO: https://stackoverflow.com/questions/5590791/http-chunked-encoding-need-an-example-of-trailer-mentioned-in-spec
        let mut r = Request::default();
        let res = r.update_raw(
            &mut "POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nWiki\r\n7\r\npedia i\r\nB\r\nn \r\nchunks.\r\n0\r\n\r\n"

                .as_bytes()
                .to_vec(),
        );
        assert_eq!(res, Ok(()));
        assert_eq!(r.body_complete(), true);
    }
    */

    #[test]
    fn test_content_length() {
        let mut r = Request::default();
        let res = r.update_raw(
            &mut "POST / HTTP/1.1\r\nContent-Length: 4\r\nHere: here\r\n\r\nBODY"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(res, Ok(()));
        assert_eq!(r.headers.values[0].to_string(), "Content-Length: 4");
        assert_eq!(r.headers.values[1].to_string(), "Here: here");
        assert_eq!(r.headers.values.len(), 2);
        assert_eq!(r.content_length, ContentLength::Value(4));
        assert_eq!(r.body(), vec![b'B', b'O', b'D', b'Y']);
        assert_eq!(r.body_complete(), true);
    }

    #[test]
    fn test_body_incomplete() {
        let mut r = Request::default();
        let res = r.update_raw(
            &mut "POST / HTTP/1.1\r\nContent-Length: 5\r\nHere: here\r\n\r\nBODY"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(res, Ok(()));
        assert_eq!(r.headers.values[0].to_string(), "Content-Length: 5");
        assert_eq!(r.headers.values[1].to_string(), "Here: here");
        assert_eq!(r.headers.values.len(), 2);
        assert_eq!(r.content_length, ContentLength::Value(5));
        assert_eq!(r.body_complete(), false);

        let res = r.update_raw(&mut "S".as_bytes().to_vec());
        assert_eq!(res, Ok(()));
        assert_eq!(r.body_complete(), true);
    }

    #[test]
    fn test_content_length_zero() {
        let mut r = Request::default();
        let res = r.update_raw(&mut "GET / HTTP/1.1\r\nHere: here\r\n".as_bytes().to_vec());
        assert_eq!(res, Ok(()));
        assert_eq!(r.body_complete(), false);

        let res = r.update_raw(&mut "More: more\r\nFinal: final\r\n\r\n".as_bytes().to_vec());
        assert_eq!(res, Ok(()));
        assert_eq!(r.headers.values[0].to_string(), "Here: here");
        assert_eq!(r.headers.values[1].to_string(), "More: more");
        assert_eq!(r.headers.values[2].to_string(), "Final: final");
        assert_eq!(r.headers.values.len(), 3);
        assert_eq!(r.content_length, ContentLength::Unset);
        assert_eq!(r.body_complete(), true);
    }

    #[test]
    fn test_multi_line_header() {
        let mut r = Request::default();
        let res = r.update_raw(
            &mut "GET / HTTP/1.1\r\nFirst: wrapp\r\n   ing\r\n\ttest\r\nSecond: wrapp\r\n    ing\r\n\ttest\r\n\r\n"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(res, Ok(()));
        assert_eq!(r.headers.values[0].to_string(), "First: wrappingtest");
        assert_eq!(r.headers.values[1].to_string(), "Second: wrappingtest");
        assert_eq!(r.body_complete(), true);
    }

    #[test]
    fn test_bad_chunked_header() {
        let mut r = Request::default();
        let res = r.update_raw(
            &mut "GET / HTTP/1.1\r\nTransfer-Encoding: chunked, gzip\r\n\r\n"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(
            res,
            Err(errors::Errors::Header(
                "chunked must appear at the very end of the Transfer-Encoding header value",
            ))
        );
    }

    #[test]
    fn test_mutually_exclusive() {
        let mut r = Request::default();
        let res = r.update_raw(
            &mut "GET / HTTP/1.1\r\nContent-Length: 10\r\nTransfer-Encoding: chunked\r\n\r\n"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(
            res,
            Err(errors::Errors::Header(
                "Transfer-Encoding and Content-Length headers are mutually exclusive",
            ))
        );
    }

    #[test]
    fn test_body() {
        let mut r = Request::default();

        let res = r.update_raw(&mut "POST TEST\r".as_bytes().to_vec());
        assert_eq!(res, Ok(()));
        let res = r.update_raw(&mut "\nContent-L".as_bytes().to_vec());
        assert_eq!(res, Ok(()));
        let res = r.update_raw(&mut "ength: 4\r\n".as_bytes().to_vec());
        assert_eq!(res, Ok(()));
        let res = r.update_raw(&mut "\r\nBODY".as_bytes().to_vec());
        assert_eq!(res, Ok(()));

        assert_eq!(r.headers.values[0].to_string(), "Content-Length: 4");
        assert_eq!(r.body_complete(), true);
    }

    #[test]
    fn test_post_edit_dump() {
        let mut r = Request::default();
        let res = r.update_raw(
            &mut "GET / HTTP/1.1\r\nWrapping: pre\r\n -\r\n\tupdate\r\nAnother: header\r\nContent-Length: 7\r\n\r\nTHE END"
                .as_bytes()
                .to_vec(),
        );
        assert_eq!(res, Ok(()));
        assert_eq!(r.headers.values[0].to_string(), "Wrapping: pre-update");
        assert_eq!(r.headers.values[1].to_string(), "Another: header");
        assert_eq!(r.headers.values[2].to_string(), "Content-Length: 7");
        assert_eq!(r.body_complete(), true);

        let res = match String::from_utf8(r.dump()) {
            Ok(s) => s,
            Err(e) => panic!("{}", e),
        };
        assert_eq!(
            res,
            "GET / HTTP/1.1\r\nWrapping: pre-update\r\nAnother: header\r\nContent-Length: 7\r\n\r\nTHE END"
        );

        r.headers
            .set(0, "Wrap".to_string(), "post-update".to_string())
            .unwrap();
        assert_eq!(r.headers.values[0].to_string(), "Wrap: post-update");

        let res = match String::from_utf8(r.dump()) {
            Ok(s) => s,
            Err(e) => panic!("{}", e),
        };
        assert_eq!(
            res,
            "GET / HTTP/1.1\r\nWrap: post-update\r\nAnother: header\r\nContent-Length: 7\r\n\r\nTHE END"
        );
    }
}
