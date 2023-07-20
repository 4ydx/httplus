pub struct Request {
    pub headers: Vec<String>,
    pub headers_end: usize,
    pub body_length: usize,
    pub raw: Vec<u8>,
}

impl Request {
    pub fn new() -> Self {
        Self {
            headers: vec![],
            headers_end: 0,
            body_length: 0,
            raw: vec![],
        }
    }

    pub fn update(&mut self, data: &mut Vec<u8>) {
        self.raw.append(data);

        if self.headers.is_empty() {
            let mut at = self.headers_end;

            while at < self.raw.len() {
                if self.raw[at..].starts_with(b"\r\n\r\n") {
                    let headers = self.raw[0..at].to_vec();

                    // https://stackoverflow.com/questions/64849149/how-to-split-a-vecu8-by-a-sequence-of-chars
                    let mut values = headers
                        .windows(2)
                        .enumerate()
                        .filter(|(_, w)| w == b"\r\n")
                        .map(|(i, _)| i)
                        .collect::<Vec<_>>();
                    values.push(self.raw.len());

                    // print!("{:#?}", values);

                    let mut split = 0;
                    for index in values {
                        if self.raw[split..index].starts_with(b"Content-Length:") {
                            let mut v = vec![];
                            let mut collect = false;
                            for val in self.raw[split..index].to_vec().iter() {
                                if collect {
                                    v.push(*val);
                                }
                                if val == &b':' {
                                    collect = true;
                                }
                            }
                            let num = String::from_utf8(v).unwrap(); // TODO...

                            self.body_length = match num.trim().parse::<usize>() {
                                Ok(i) => i,
                                Err(_) => 0,
                            };
                        }
                        split = index + "\r\n".len();
                    }
                }
                at += 1;
            }
            self.headers_end = at;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut r = Request::new();
        r.update(&mut "GET / HTTP/1.1\r\nContent-Length: 33\r\nHere: here\r\n\r\n".as_bytes().to_vec());

        assert_eq!(r.body_length, 33);
    }
}
