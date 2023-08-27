#[derive(Debug, PartialEq)]
struct Point {
    s: usize, // start
    e: usize, // end
}

#[derive(Debug, PartialEq)]
pub struct Raw {
    pub charset: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub struct EncodedWord {
    pub raw: Raw,
    value: String,
}

impl EncodedWord {
    pub fn as_utf8(&self) -> &String {
        &self.value
    }
}

fn parse_encoded_words(value: &str, words_at: Vec<Point>) -> Vec<EncodedWord> {
    let mut words: Vec<EncodedWord> = vec![];
    for word_at in words_at {
        let bytes = &value[word_at.s..word_at.e];
        let raw = Raw {
            charset: "".to_owned(),
            bytes: bytes.into(),
        };
        let word = EncodedWord {
            raw,
            value: "".to_owned(),
        };
        words.push(word);
    }
    words
}

fn find_encoded_words(value: &str) -> Vec<Point> {
    let mut words_at: Vec<Point> = vec![];
    let value_bytes = value.as_bytes();
    for i in 0..value_bytes.len() - 1 {
        if value_bytes[i] == b'=' && value_bytes[i + 1] == b'?' {
            words_at.push(Point {
                s: i,
                e: usize::MAX,
            })
        }
        if value_bytes[i] == b'?' && value_bytes[i + 1] == b'=' {
            match words_at.pop() {
                Some(mut v) => {
                    v.e = i + "?=".len();
                    words_at.push(v);
                }
                None => (),
            }
        }
    }
    words_at
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_encoded_words() {
        let expect = vec![Point { s: 0, e: 5 }, Point { s: 6, e: 11 }];
        assert_eq!(expect, find_encoded_words("=?a?= =?b?="));
    }

    #[test]
    fn test_parse_encoded_words() {
        let value = "=?a?= nothing =?b?=";
        let words_at = find_encoded_words(value);

        let word1 = EncodedWord {
            raw: Raw {
                charset: "".to_owned(),
                bytes: vec![b'=', b'?', b'a', b'?', b'='],
            },
            value: "".to_owned(),
        };
        let word2 = EncodedWord {
            raw: Raw {
                charset: "".to_owned(),
                bytes: vec![b'=', b'?', b'b', b'?', b'='],
            },
            value: "".to_owned(),
        };
        let expect = vec![word1, word2];

        assert_eq!(expect, parse_encoded_words(value, words_at));
    }
}
