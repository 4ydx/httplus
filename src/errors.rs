#[derive(Debug, Clone, PartialEq)]
pub enum Errors<'a> {
    HeaderIndexOutOfBounds,
    HeaderKeyWhitespace,
    HeaderNonAsciiByteAt(usize),
    HeaderIsEmpty,
    HeaderFromUtf8(std::string::FromUtf8Error),
    CannotFillHeaders,
    Header(&'a str),
    Parse(std::string::FromUtf8Error),
    ContentLength(std::num::ParseIntError),
}
