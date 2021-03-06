#![cfg_attr(not(feature = "std"),  no_std)]
#![cfg_attr(test, deny(warnings))]
#![deny(missing_docs)]
//! # icaparse
//!
//! A push library for parsing ICAP/1.0 requests and responses.
//!
//! The focus is on speed and safety. Unsafe code is used to keep parsing fast,
//! but unsafety is contained in a submodule, with invariants enforced. The
//! parsing internals use an `Iterator` instead of direct indexing, while
//! skipping bounds checks.
//!
//! This parser is based heavily on the httparse HTTP parsing library.
#[cfg(feature = "std")] extern crate std as core;

use core::{fmt, result, str, slice};
use std::collections::HashMap;

use iter::Bytes;

mod iter;
mod test;

macro_rules! next {
    ($bytes:ident) => ({
        match $bytes.next() {
            Some(b) => b,
            None => return Ok(Status::Partial)
        }
    })
}

macro_rules! expect {
    ($bytes:ident.next() == $pat:pat => $ret:expr) => {
        expect!(next!($bytes) => $pat |? $ret)
    };
    ($e:expr => $pat:pat |? $ret:expr) => {
        match $e {
            v@$pat => v,
            _ => return $ret
        }
    };
}

macro_rules! complete {
    ($e:expr) => {
        match try!($e) {
            Status::Complete(v) => v,
            Status::Partial => return Ok(Status::Partial)
        }
    }
}

#[inline]
fn shrink<T>(slice: &mut &mut [T], len: usize) {
    debug_assert!(slice.len() >= len);
    let ptr = slice.as_mut_ptr();
    *slice = unsafe { slice::from_raw_parts_mut(ptr, len) };
}

/// Determines if byte is a token char.
///
/// > ```notrust
/// > token          = 1*tchar
/// >
/// > tchar          = "!" / "#" / "$" / "%" / "&" / "'" / "*"
/// >                / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~"
/// >                / DIGIT / ALPHA
/// >                ; any VCHAR, except delimiters
/// > ```
#[inline]
fn is_token(b: u8) -> bool {
    b > 0x1F && b < 0x7F
}
macro_rules! byte_map {
    ($($flag:expr,)*) => ([
        $($flag != 0,)*
    ])
}

static HEADER_NAME_MAP: [bool; 256] = byte_map![
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 1, 0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0,
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 0, 1, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[inline]
fn is_header_name_token(b: u8) -> bool {
    HEADER_NAME_MAP[b as usize]
}

static HEADER_VALUE_MAP: [bool; 256] = byte_map![
    0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
];


#[inline]
fn is_header_value_token(b: u8) -> bool {
    HEADER_VALUE_MAP[b as usize]
}


macro_rules! space {
    ($bytes:ident or $err:expr) => ({
        expect!($bytes.next() == b' ' => Err($err));
        $bytes.slice();
    })
}

macro_rules! newline {
    ($bytes:ident) => ({
        match next!($bytes) {
            b'\r' => {
                expect!($bytes.next() == b'\n' => Err(Error::NewLine));
                $bytes.slice();
            },
            b'\n' => {
                $bytes.slice();
            },
            _ => return Err(Error::NewLine)
        }
    })
}

/// An error in parsing.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Error {
    /// Invalid byte in header name.
    HeaderName,
    /// Invalid byte in header value.
    HeaderValue,
    /// Invalid byte in new line.
    NewLine,
    /// Invalid byte in Response status.
    Status,
    /// Invalid byte where token is required.
    Token,
    /// Parsed more headers than provided buffer can contain.
    TooManyHeaders,
    /// Invalid byte in HTTP version.
    Version,
    /// Missing encapsulated header
    MissingEncapsulated
}

impl Error {
    #[inline]
    fn description_str(&self) -> &'static str {
        match *self {
            Error::HeaderName => "invalid header name",
            Error::HeaderValue => "invalid header value",
            Error::NewLine => "invalid new line",
            Error::Status => "invalid response status",
            Error::Token => "invalid token",
            Error::TooManyHeaders => "too many headers",
            Error::Version => "invalid ICAP version",
            Error::MissingEncapsulated => "missing encapsulated ICAP header",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description_str())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn description(&self) -> &str {
        self.description_str()
    }
}

/// An error in parsing a chunk size.
// Note: Move this into the error enum once v2.0 is released.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidChunkSize;

impl fmt::Display for InvalidChunkSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("invalid chunk size")
    }
}

/// A Result of any parsing action.
///
/// If the input is invalid, an `Error` will be returned. Note that incomplete
/// data is not considered invalid, and so will not return an error, but rather
/// a `Ok(Status::Partial)`.
pub type Result<T> = result::Result<Status<T>, Error>;

/// The result of a successful parse pass.
///
/// `Complete` is used when the buffer contained the complete value.
/// `Partial` is used when parsing did not reach the end of the expected value,
/// but no invalid data was found.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Status<T> {
    /// The completed result.
    Complete(T),
    /// A partial result.
    Partial
}

impl<T> Status<T> {
    /// Convenience method to check if status is complete.
    #[inline]
    pub fn is_complete(&self) -> bool {
        match *self {
            Status::Complete(..) => true,
            Status::Partial => false
        }
    }

    /// Convenience method to check if status is partial.
    #[inline]
    pub fn is_partial(&self) -> bool {
        match *self {
            Status::Complete(..) => false,
            Status::Partial => true
        }
    }

    /// Convenience method to unwrap a Complete value. Panics if the status is
    /// `Partial`.
    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            Status::Complete(t) => t,
            Status::Partial => panic!("Tried to unwrap Status::Partial")
        }
    }
}

/// A parsed Request.
///
/// The optional values will be `None` if a parse was not complete, and did not
/// parse the associated property. This allows you to inspect the parts that
/// could be parsed, before reading more, in case you wish to exit early.
///
/// # Example
///
/// ```no_run
/// let buf = b"RESPMOD /404 ICAP/1.0\r\nHost:";
/// let mut headers = [icaparse::EMPTY_HEADER; 16];
/// let mut req = icaparse::Request::new(&mut headers);
/// let res = req.parse(buf).unwrap();
/// if res.is_partial() {
///     match req.path {
///         Some(ref path) => {
///             // check router for path.
///             // /404 doesn't exist? we could stop parsing
///         },
///         None => {
///             // must read more and parse again
///         }
///     }
/// }
/// ```
#[derive(Debug, PartialEq)]
pub struct Request<'headers, 'buf: 'headers> {
    /// The request method, such as `RESPMOD`.
    pub method: Option<&'buf str>,
    /// The request path, such as `/about-us`.
    pub path: Option<&'buf str>,
    /// The request version, such as `ICAP/1.0`.
    pub version: Option<u8>,
    /// The request headers.
    pub headers: &'headers mut [Header<'buf>],
    /// The sections of the encapsulated body listed in the Encapsulated header
    pub encapsulated_sections: Option<HashMap<SectionType, Vec<u8>>>
}

impl<'h, 'b> Request<'h, 'b> {
    /// Creates a new Request, using a slice of headers you allocate.
    #[inline]
    pub fn new(headers: &'h mut [Header<'b>]) -> Request<'h, 'b> {
        Request {
            method: None,
            path: None,
            version: None,
            headers: headers,
            encapsulated_sections: None,
        }
    }

    /// Try to parse a buffer of bytes into the Request.
    pub fn parse(&mut self, buf: &'b [u8]) -> Result<usize> {
        let orig_len = buf.len();
        let mut bytes = Bytes::new(buf);
        complete!(skip_empty_lines(&mut bytes));
        self.method = Some(complete!(parse_token(&mut bytes)));
        self.path = Some(complete!(parse_token(&mut bytes)));
        self.version = Some(complete!(parse_version(&mut bytes)));
        newline!(bytes);

        let len = orig_len - bytes.len();
        let headers_len = complete!(parse_headers_iter(&mut self.headers, &mut bytes));
        match self.headers.iter().find(|&h| h.name == "Encapsulated") {
            Some(h) => {
                self.encapsulated_sections = Some(parse_encapsulated(h.value, &buf[len+headers_len..buf.len()]));
                Ok(Status::Complete(orig_len + len - len + headers_len - headers_len))
            },
            None => {
                match self.method {
                  Some("OPTIONS") => Ok(Status::Complete(orig_len)),
                  _ =>  Err(Error::MissingEncapsulated)
                }
            }
        }

    }
}

#[inline]
fn skip_empty_lines(bytes: &mut Bytes) -> Result<()> {
    loop {
        let b = bytes.peek();
        match b {
            Some(b'\r') => {
                bytes.bump();
                expect!(bytes.next() == b'\n' => Err(Error::NewLine));
            },
            Some(b'\n') => {
                bytes.bump();
            },
            Some(..) => {
                bytes.slice();
                return Ok(Status::Complete(()));
            },
            None => return Ok(Status::Partial)
        }
    }
}

/// A parsed Response.
///
/// See `Request` docs for explanation of optional values.
#[derive(Debug, PartialEq)]
pub struct Response<'headers, 'buf: 'headers> {
    /// The response version, such as `ICAP/1.0`.
    pub version: Option<u8>,
    /// The response code, such as `200`.
    pub code: Option<u16>,
    /// The response reason-phrase, such as `OK`.
    pub reason: Option<&'buf str>,
    /// The response headers.
    pub headers: &'headers mut [Header<'buf>]
}

impl<'h, 'b> Response<'h, 'b> {
    /// Creates a new `Response` using a slice of `Header`s you have allocated.
    #[inline]
    pub fn new(headers: &'h mut [Header<'b>]) -> Response<'h, 'b> {
        Response {
            version: None,
            code: None,
            reason: None,
            headers: headers,
        }
    }

    /// Try to parse a buffer of bytes into this `Response`.
    pub fn parse(&mut self, buf: &'b [u8]) -> Result<usize> {
        let orig_len = buf.len();
        let mut bytes = Bytes::new(buf);

        complete!(skip_empty_lines(&mut bytes));
        self.version = Some(complete!(parse_version(&mut bytes)));
        space!(bytes or Error::Version);
        self.code = Some(complete!(parse_code(&mut bytes)));

        // RFC7230 says there must be 'SP' and then reason-phrase, but admits
        // its only for legacy reasons. With the reason-phrase completely
        // optional (and preferred to be omitted) in HTTP2, we'll just
        // handle any response that doesn't include a reason-phrase, because
        // it's more lenient, and we don't care anyways.
        //
        // So, a SP means parse a reason-phrase.
        // A newline means go to headers.
        // Anything else we'll say is a malformed status.
        match next!(bytes) {
            b' ' => {
                bytes.slice();
                self.reason = Some(complete!(parse_reason(&mut bytes)));
            },
            b'\r' => {
                expect!(bytes.next() == b'\n' => Err(Error::Status));
                self.reason = Some("");
            },
            b'\n' => self.reason = Some(""),
            _ => return Err(Error::Status),
        }


        let len = orig_len - bytes.len();
        let headers_len = complete!(parse_headers_iter(&mut self.headers, &mut bytes));
        Ok(Status::Complete(len + headers_len))
    }
}

/// Represents a parsed header.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Header<'a> {
    /// The name portion of a header.
    ///
    /// A header name must be valid ASCII-US, so it's safe to store as a `&str`.
    pub name: &'a str,
    /// The value portion of a header.
    ///
    /// While headers **should** be ASCII-US, the specification allows for
    /// values that may not be, and so the value is stored as bytes.
    pub value: &'a [u8],
}

/// An empty header, useful for constructing a `Header` array to pass in for
/// parsing.
///
/// # Example
///
/// ```
/// let headers = [icaparse::EMPTY_HEADER; 64];
/// ```
pub const EMPTY_HEADER: Header<'static> = Header { name: "", value: b"" };

#[inline]
fn parse_version(bytes: &mut Bytes) -> Result<u8> {
    if let Some(mut eight) = bytes.next_8() {
        expect!(eight._0() => b'I' |? Err(Error::Version));
        expect!(eight._1() => b'C' |? Err(Error::Version));
        expect!(eight._2() => b'A' |? Err(Error::Version));
        expect!(eight._3() => b'P' |? Err(Error::Version));
        expect!(eight._4() => b'/' |? Err(Error::Version));
        expect!(eight._5() => b'1' |? Err(Error::Version));
        expect!(eight._6() => b'.' |? Err(Error::Version));
        let v = match eight._7() {
            b'0' => 0,
            b'1' => 1,
            _ => return Err(Error::Version)
        };
        Ok(Status::Complete(v))
    } else {
        Ok(Status::Partial)
    }
}

fn find_section_start(encapsulated: &str, section: &str) -> Option<usize> {
    if let Some(x) = encapsulated.find(section) {
        let start = x + section.len();
        let end = match encapsulated[start..encapsulated.len()].find(',') {
            Some(i) => start + i,
            None => encapsulated.len()
        };
        encapsulated[start..end].parse::<usize>().ok()
    }
    else {
        None
    }
}

/// Describes a section of the encapsulated data
/// as listed in the Encapsulated header
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct EncapsulationSection {
    /// The type of data in the section
    name: SectionType,
    /// The start of the section
    start: usize
}

impl EncapsulationSection {
    /// provides a constructor for method for the type
  pub fn new(name: SectionType, start: usize) -> EncapsulationSection {
      EncapsulationSection {
          name: name,
          start: start
      }
  }
}

impl fmt::Display for EncapsulationSection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "found {:?} starting at {}", self.name, self.start)
    }
}

/// Possible sections of the encapsulated icap data
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum SectionType {
    /// Null Body Section
    NullBody,
    /// Resquest Header Section
    RequestHeader,
    /// Request Body Section
    RequestBody,
    /// Response Header Section
    ResponseHeader,
    /// Response Body Section
    ResponseBody,
    /// Options Body Section
    OptionsBody
}

fn parse_encapsulated(header: &[u8], encapsulated: &[u8]) -> HashMap<SectionType, Vec<u8>> {
    let mut sections = Vec::new();
    {
        let mut add_section = |section, start| sections.push(EncapsulationSection::new(section, start));

        if let Ok(encapsulated) = str::from_utf8(header) {
            if let Some(section_start) = find_section_start(encapsulated, "req-hdr=") {
                add_section(SectionType::RequestHeader, section_start);
            }
            if let Some(section_start) = find_section_start(encapsulated, "req-body=") {
                add_section(SectionType::ResponseBody, section_start);
            }
            if let Some(section_start) = find_section_start(encapsulated, "null-body=") {
                add_section(SectionType::NullBody, section_start);
            }
            if let Some(section_start) = find_section_start(encapsulated, "res-hdr=") {
                add_section(SectionType::ResponseHeader, section_start);
            }
            if let Some(section_start) = find_section_start(encapsulated, "res-body=") {
                add_section(SectionType::ResponseBody, section_start);
            }
            if let Some(section_start) = find_section_start(encapsulated, "opt-body=") {
                add_section(SectionType::OptionsBody, section_start);
            }
        }
    }
    sections.sort_by(|a, b| a.start.cmp(&b.start));
    let mut hm = HashMap::new();
    let mut iter = sections.iter().peekable();
    loop {
        let x = iter.next();
        let y = iter.peek();
        if x.is_some() {
            let x = x.unwrap();
            if y.is_some() {
                let y = y.unwrap();
                hm.insert(x.name.clone(), encapsulated[x.start..y.start].to_vec());
            }
            else {
                hm.insert(x.name.clone(), encapsulated[x.start..encapsulated.len()].to_vec());
            }
        }
        else {
            break;
        }
    }
    hm
}

/// From [RFC 7230](https://tools.ietf.org/html/rfc7230):
///
/// > ```notrust
/// > reason-phrase  = *( HTAB / SP / VCHAR / obs-text )
/// > HTAB           = %x09        ; horizontal tab
/// > VCHAR          = %x21-7E     ; visible (printing) characters
/// > obs-text       = %x80-FF
/// > ```
///
/// > A.2.  Changes from RFC 2616
/// >
/// > Non-US-ASCII content in header fields and the reason phrase
/// > has been obsoleted and made opaque (the TEXT rule was removed).
///
/// Note that the following implementation deliberately rejects the obsoleted (non-US-ASCII) text range.
///
/// The fully compliant parser should probably just return the reason-phrase as an opaque &[u8] data
/// and leave interpretation to user or specialized helpers (akin to .display() in `std::path::Path`)
#[inline]
fn parse_reason<'a>(bytes: &mut Bytes<'a>) -> Result<&'a str> {
    loop {
        let b = next!(bytes);
        if b == b'\r' {
            expect!(bytes.next() == b'\n' => Err(Error::Status));
            return Ok(Status::Complete(unsafe {
                // all bytes up till `i` must have been HTAB / SP / VCHAR
                str::from_utf8_unchecked(bytes.slice_skip(2))
            }));
        } else if b == b'\n' {
            return Ok(Status::Complete(unsafe {
                // all bytes up till `i` must have been HTAB / SP / VCHAR
                str::from_utf8_unchecked(bytes.slice_skip(1))
            }));
        } else if !((b >= 0x20 && b <= 0x7E) || b == b'\t') {
            return Err(Error::Status);
        }
    }
}

#[inline]
fn parse_token<'a>(bytes: &mut Bytes<'a>) -> Result<&'a str> {
    loop {
        let b = next!(bytes);
        if b == b' ' {
            return Ok(Status::Complete(unsafe {
                // all bytes up till `i` must have been `is_token`.
                str::from_utf8_unchecked(bytes.slice_skip(1))
            }));
        } else if !is_token(b) {
            return Err(Error::Token);
        }
    }
}

#[inline]
fn parse_code(bytes: &mut Bytes) -> Result<u16> {
    let hundreds = expect!(bytes.next() == b'0'...b'9' => Err(Error::Status));
    let tens = expect!(bytes.next() == b'0'...b'9' => Err(Error::Status));
    let ones = expect!(bytes.next() == b'0'...b'9' => Err(Error::Status));

    Ok(Status::Complete(u16::from(hundreds - b'0') * 100 +
                        u16::from(tens - b'0') * 10 +
                        u16::from(ones - b'0')))
}

/// Parse a buffer of bytes as headers.
///
/// The return value, if complete and successful, includes the index of the
/// buffer that parsing stopped at, and a sliced reference to the parsed
/// headers. The length of the slice will be equal to the number of properly
/// parsed headers.
///
/// # Example
///
/// ```
/// let buf = b"Host: foo.bar\nAccept: */*\n\nblah blah";
/// let mut headers = [icaparse::EMPTY_HEADER; 4];
/// assert_eq!(icaparse::parse_headers(buf, &mut headers),
///            Ok(icaparse::Status::Complete((27, &[
///                icaparse::Header { name: "Host", value: b"foo.bar" },
///                icaparse::Header { name: "Accept", value: b"*/*" }
///            ][..]))));
/// ```
pub fn parse_headers<'b: 'h, 'h>(src: &'b [u8], mut dst: &'h mut [Header<'b>])
    -> Result<(usize, &'h [Header<'b>])> {
    let mut iter = Bytes::new(src);
    let pos = complete!(parse_headers_iter(&mut dst, &mut iter));
    Ok(Status::Complete((pos, dst)))
}

#[inline]
fn parse_headers_iter<'a, 'b>(headers: &mut &mut [Header<'a>], bytes: &'b mut Bytes<'a>)
    -> Result<usize> {
    let mut num_headers: usize = 0;
    let mut count: usize = 0;
    let mut result = Err(Error::TooManyHeaders);

    {
        let mut iter = headers.iter_mut();

        'headers: loop {
            // a newline here means the head is over!
            let b = next!(bytes);
            if b == b'\r' {
                expect!(bytes.next() == b'\n' => Err(Error::NewLine));
                result = Ok(Status::Complete(count + bytes.pos()));
                break;
            } else if b == b'\n' {
                result = Ok(Status::Complete(count + bytes.pos()));
                break;
            } else if !is_header_name_token(b) {
                return Err(Error::HeaderName);
            }

            let header = match iter.next() {
                Some(header) => header,
                None => break 'headers
            };

            num_headers += 1;
            // parse header name until colon
            'name: loop {
                let b = next!(bytes);
                if b == b':' {
                    count += bytes.pos();
                    header.name = unsafe {
                        str::from_utf8_unchecked(bytes.slice_skip(1))
                    };
                    break 'name;
                } else if !is_header_name_token(b) {
                    return Err(Error::HeaderName);
                }
            }

            let mut b;

            'value: loop {

                // eat white space between colon and value
                'whitespace: loop {
                    b = next!(bytes);
                    if b == b' ' || b == b'\t' {
                        count += bytes.pos();
                        bytes.slice();
                        continue 'whitespace;
                    } else {
                        if !is_header_value_token(b) {
                            break 'value;
                        }
                        break 'whitespace;
                    }
                }

                // parse value till EOL



                macro_rules! check {
                    ($bytes:ident, $i:ident) => ({
                        b = $bytes.$i();
                        if !is_header_value_token(b) {
                            break 'value;
                        }
                    });
                    ($bytes:ident) => ({
                        check!($bytes, _0);
                        check!($bytes, _1);
                        check!($bytes, _2);
                        check!($bytes, _3);
                        check!($bytes, _4);
                        check!($bytes, _5);
                        check!($bytes, _6);
                        check!($bytes, _7);
                    })
                }
                while let Some(mut bytes8) = bytes.next_8() {
                    check!(bytes8);
                }
                loop {
                    b = next!(bytes);
                    if !is_header_value_token(b) {
                        break 'value;
                    }
                }
            }

            //found_ctl
            if b == b'\r' {
                expect!(bytes.next() == b'\n' => Err(Error::HeaderValue));
                count += bytes.pos();
                header.value = bytes.slice_skip(2);
            } else if b == b'\n' {
                count += bytes.pos();
                header.value = bytes.slice_skip(1);
            } else {
                return Err(Error::HeaderValue);
            }

        }
    } // drop iter

    shrink(headers, num_headers);
    result
}

/// Parse a buffer of bytes as a chunk size.
///
/// The return value, if complete and successful, includes the index of the
/// buffer that parsing stopped at, and the size of the following chunk.
///
/// # Example
///
/// ```
/// let buf = b"4\r\nRust\r\n0\r\n\r\n";
/// assert_eq!(icaparse::parse_chunk_size(buf),
///            Ok(icaparse::Status::Complete((3, 4))));
/// ```
pub fn parse_chunk_size(buf: &[u8])
        -> result::Result<Status<(usize, u64)>, InvalidChunkSize> {
    const RADIX: u64 = 16;
    let mut bytes = Bytes::new(buf);
    let mut size = 0;
    let mut in_chunk_size = true;
    let mut in_ext = false;
    let mut count = 0;
    loop {
        let b = next!(bytes);
        match b {
            b'0'...b'9' if in_chunk_size => {
                if count > 15 {
                    return Err(InvalidChunkSize);
                }
                count += 1;
                size *= RADIX;
                size += u64::from(b - b'0');
            },
            b'a'...b'f' if in_chunk_size => {
                if count > 15 {
                    return Err(InvalidChunkSize);
                }
                count += 1;
                size *= RADIX;
                size += u64::from(b + 10 - b'a');
            }
            b'A'...b'F' if in_chunk_size => {
                if count > 15 {
                    return Err(InvalidChunkSize);
                }
                count += 1;
                size *= RADIX;
                size += u64::from(b + 10 - b'A');
            }
            b'\r' => {
                match next!(bytes) {
                    b'\n' => break,
                    _ => return Err(InvalidChunkSize),
                }
            }
            // If we weren't in the extension yet, the ";" signals its start
            b';' if !in_ext => {
                in_ext = true;
                in_chunk_size = false;
            }
            // "Linear white space" is ignored between the chunk size and the
            // extension separator token (";") due to the "implied *LWS rule".
            b'\t' | b' ' if !in_ext & !in_chunk_size => {}
            // LWS can follow the chunk size, but no more digits can come
            b'\t' | b' ' if in_chunk_size => in_chunk_size = false,
            // We allow any arbitrary octet once we are in the extension, since
            // they all get ignored anyway. According to the HTTP spec, valid
            // extensions would have a more strict syntax:
            //     (token ["=" (token | quoted-string)])
            // but we gain nothing by rejecting an otherwise valid chunk size.
            _ if in_ext => {}
            // Finally, if we aren't in the extension and we're reading any
            // other octet, the chunk size line is invalid!
            _ => return Err(InvalidChunkSize),
        }
    }
    Ok(Status::Complete((bytes.pos(), size)))
}

