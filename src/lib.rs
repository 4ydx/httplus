const HEADERS_SPLIT: &str = "\r\n\r\n";
const HEADERS_SPLIT_TOP: usize = HEADERS_SPLIT.len() + 1;

pub struct Request {
    pub headers: Vec<String>,
    pub body_length: usize,
    pub raw: String,
}

impl Request {
    pub fn new() -> Self {
        Self {
            headers: vec![],
            body_length: 0,
            raw: String::from(""),
        }
    }

    pub fn update(&mut self, data: &String) {
        self.raw += data;
        if self.raw.len() < HEADERS_SPLIT_TOP {
            return;
        }

        if self.headers.is_empty() {
            let mut as_chars = self.raw.chars().enumerate();
            let count = as_chars.clone().count();
            print!("COUNT {}\n", count);

            loop {
                let (index, c) = match as_chars.next() {
                    Some(v) => v,
                    None => break,
                };

                if c == '\r'
                    && as_chars.next().unwrap() == '\n'
                    && as_chars.next().unwrap() == '\r'
                    && as_chars.next().unwrap() == '\n'
                {
                    self.headers = self.raw[0..at]
                        .split("\r\n")
                        .map(|s| s.to_owned())
                        .collect();

                    for header in &self.headers {
                        if header.starts_with("Content-Length:") {
                            let parts: Vec<&str> = header.split(":").collect();
                            let num = parts.last().unwrap();
                            self.body_length = match num.trim().parse::<usize>() {
                                Ok(i) => i,
                                Err(_e) => 0,
                            };
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut r = Request::new();
        r.update(&String::from("GET / HTTP/1.1\r\nTest: test\r\n\r\n"));

        assert_eq!(r.raw, String::from("GET"));
        assert_eq!(r.body_length, 0);
    }
}
