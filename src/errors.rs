#[derive(Debug, Clone, PartialEq)]
pub enum Errors<'a> {
    HeadersEnd,
    Header(&'a str),
    Parse(std::string::FromUtf8Error),
    ContentLength(std::num::ParseIntError),
}
