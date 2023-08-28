use base64::{engine::general_purpose, Engine as _};
use encoding::label::encoding_from_whatwg_label;
use encoding::DecoderTrap;

#[derive(Debug, PartialEq)]
struct Point {
    s: usize, // start
    e: usize, // end
}

#[derive(Debug, PartialEq)]
pub struct Raw {
    pub charset: String,
    pub encoding: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub struct EncodedWord {
    pub raw: Raw,
    pub value: String,
    pub error: String,
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

        let parts: Vec<&str> = bytes.split('?').collect();
        let charset = parts[1];
        let encoded_text = parts[3];

        let raw = Raw {
            charset: charset.to_owned(),
            encoding: parts[2].to_owned(),
            bytes: bytes.into(),
        };
        let mut word = EncodedWord {
            raw,
            value: "".to_owned(),
            error: "".to_owned(),
        };
        if word.raw.encoding == "B" {
            let mut bytes: Vec<u8> = vec![];
            match &general_purpose::STANDARD_NO_PAD.decode(encoded_text) {
                Ok(b) => bytes = b.to_vec(),
                Err(e) => word.error = e.to_string(),
            };
            match encoding_from_whatwg_label(charset) {
                Some(enc) => {
                    match enc.decode(&bytes, DecoderTrap::Replace) {
                        Ok(b) => word.value = b,
                        Err(e) => word.error = e.to_string(),
                    };
                }
                None => word.error = format!("unsupported charset {}", charset).to_owned(),
            };
        }
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
    fn test_parse_encoded_words_1() {
        let value = "=?US-ASCII?Q?Keith_Moore?= <moore@cs.utk.edu>";
        let words_at = find_encoded_words(value);
        let word1 = EncodedWord {
            raw: Raw {
                charset: "US-ASCII".to_owned(),
                encoding: "Q".to_owned(),
                bytes: value[words_at[0].s..words_at[0].e].as_bytes().to_vec(),
            },
            value: "".to_owned(),
            error: "".to_owned(),
        };
        let expect = vec![word1];

        assert_eq!(expect, parse_encoded_words(value, words_at));
    }
}
