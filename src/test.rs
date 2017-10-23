use super::*;

#[cfg(test)]
mod test {
    extern crate httparse;
    use super::{Request, Response, Status, EMPTY_HEADER, shrink, parse_chunk_size, SectionType};

    const NUM_OF_HEADERS: usize = 4;

    #[test]
    fn test_shrink() {
        let mut arr = [EMPTY_HEADER; 16];
        {
            let slice = &mut &mut arr[..];
            assert_eq!(slice.len(), 16);
            shrink(slice, 4);
            assert_eq!(slice.len(), 4);
        }
        assert_eq!(arr.len(), 16);
    }

    macro_rules! req {
        ($name:ident, $buf:expr, |$arg:ident| $body:expr) => (
            req! {$name, $buf, Ok(Status::Complete($buf.len())), |$arg| $body }
        );
        ($name:ident, $buf:expr, $len:expr, |$arg:ident| $body:expr) => (
        #[test]
        fn $name() {
            let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
            let mut req = Request::new(&mut headers[..]);
            let status = req.parse($buf.as_ref());
            assert_eq!(status, $len);
            closure(req);

            fn closure($arg: Request) {
                $body
            }
        }
        )
    }

    req! {
        test_request_simple,
        b"OPTIONS / ICAP/1.0\r\nEncapsulated:null-body=0\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "OPTIONS");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 0);
            assert_eq!(req.headers.len(), 1);
        }
    }

    req! {
        test_icap_options,
        b"OPTIONS icap://example.local/service ICAP/1.0\r\nHost: example.local\r\nUser-Agent: Example-ICAP-Client-Library/2.0\r\nEncapsulated:null-body=0\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "OPTIONS");
            assert_eq!(req.path.unwrap(), "icap://example.local/service");
            assert_eq!(req.headers.len(), 3);
            assert_eq!(req.headers[0].name, "Host");
            assert_eq!(req.headers[0].value, b"example.local");
            assert_eq!(req.headers[1].name, "User-Agent");
            assert_eq!(req.headers[1].value, b"Example-ICAP-Client-Library/2.0");
        }
    }

    req! {
        test_reqmod_basic,
        b"REQMOD icap://icap-server.net/server?arg=87 ICAP/1.0\r
Host: icap-server.net\r
Encapsulated: req-hdr=0, null-body=170\r
\r
GET / HTTP/1.1\r
Host: www.origin-server.com\r
Accept: text/html, text/plain\r
Accept-Encoding: compress\r
Cookie: ff39fk3jur@4ii0e02i\r
If-None-Match: \"xyzzy\", \"r2d2xxxx\"\r
\r
",
       |req| {
           assert_eq!(req.method.unwrap(), "REQMOD");
           let encapsulated = req.encapsulated_sections.unwrap();
           assert_eq!(encapsulated.len(), 2);
           let mut headers = [httparse::EMPTY_HEADER; 16];
           let mut req = httparse::Request::new(&mut headers);
           let http_request = encapsulated.get(&SectionType::RequestHeader).unwrap();
           assert_eq!(req.parse(http_request).unwrap().is_complete(), true);
       }
    }

    req! {
        test_basic_respmod,
        b"RESPMOD / ICAP/1.0\r\nEncapsulated: null-body=0\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "RESPMOD");
        }
    }

    req! {
        test_full_respmod,
        b"RESPMOD icap://icap.example.org/satisf ICAP/1.0\r
Host: icap.example.org\r
Encapsulated: req-hdr=0, res-hdr=137, res-body=296\r
\r
GET /origin-resource HTTP/1.1\r
Host: www.origin-server.com\r
Accept: text/html, text/plain, image/gif\r
Accept-Encoding: gzip, compress\r
\r
HTTP/1.1 200 OK\r
Date: Mon, 10 Jan 2000 09:52:22 GMT\r
Server: Apache/1.3.6 (Unix)\r
ETag: \"63840-1ab7-378d415b\"\r
Content-Type: text/html\r
Content-Length: 51\r
\r
33\r
This is data that was returned by an origin server.\r
0\r
\r
",
        |req| {
            use SectionType::RequestHeader;
            assert_eq!(req.method.unwrap(), "RESPMOD");
            let sections = req.encapsulated_sections.unwrap();
            assert_eq!(sections[&RequestHeader], b"GET /origin-resource HTTP/1.1\r
Host: www.origin-server.com\r
Accept: text/html, text/plain, image/gif\r
Accept-Encoding: gzip, compress\r
\r
".to_vec());
        }
    }



    req! {
        test_request_headers_max,
        b"RESPMOD / ICAP/1.0\r\nA: A\r\nB: B\r\nC: C\r\nEncapsulated:null-body=0\r\n\r\n",
        |req| {
            assert_eq!(req.headers.len(), NUM_OF_HEADERS);
        }
    }

    req! {
        test_request_multibyte,
        b"RESPMOD / ICAP/1.0\r\nHost: foo.com\r\nUser-Agent: \xe3\x81\xb2\xe3/1.0\r\nEncapsulated:null-body=0\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "RESPMOD");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 0);
            assert_eq!(req.headers[0].name, "Host");
            assert_eq!(req.headers[0].value, b"foo.com");
            assert_eq!(req.headers[1].name, "User-Agent");
            assert_eq!(req.headers[1].value, b"\xe3\x81\xb2\xe3/1.0");
        }
    }


    req! {
        test_request_partial,
        b"RESPMOD / ICAP/1.0\r\n\r", Ok(Status::Partial),
        |_req| {}
    }

    req! {
        test_request_newlines,
        b"RESPMOD / ICAP/1.0\nHost: foo.bar\nEncapsulated:null-body=0\n\n",
        |_r| {}
    }

    req! {
        test_request_empty_lines_prefix,
        b"\r\n\r\nRESPMOD / ICAP/1.0\r\nEncapsulated:null-body=0\r\n\r\n",
        |req| {
            assert_eq!(req.method.unwrap(), "RESPMOD");
            assert_eq!(req.path.unwrap(), "/");
            assert_eq!(req.version.unwrap(), 0);
            assert_eq!(req.headers.len(), 1);
        }
    }

    req! {
        test_request_with_invalid_token_delimiter,
        b"RESPMOD\n/ ICAP/1.0\r\nHost: foo.bar\r\n\r\n",
        Err(::Error::Token),
        |_r| {}
    }

    macro_rules! res {
        ($name:ident, $buf:expr, |$arg:ident| $body:expr) => (
            res! {$name, $buf, Ok(Status::Complete($buf.len())), |$arg| $body }
        );
        ($name:ident, $buf:expr, $len:expr, |$arg:ident| $body:expr) => (
        #[test]
        fn $name() {
            let mut headers = [EMPTY_HEADER; NUM_OF_HEADERS];
            let mut res = Response::new(&mut headers[..]);
            let status = res.parse($buf.as_ref());
            assert_eq!(status, $len);
            closure(res);

            fn closure($arg: Response) {
                $body
            }
        }
        )
    }

    res! {
        test_response_simple,
        b"ICAP/1.0 200 OK\r\n\r\n",
        |res| {
            assert_eq!(res.version.unwrap(), 0);
            assert_eq!(res.code.unwrap(), 200);
            assert_eq!(res.reason.unwrap(), "OK");
        }
    }

    res! {
        test_response_newlines,
        b"ICAP/1.0 403 Forbidden\nServer: foo.bar\n\n",
        |_r| {}
    }

    res! {
        test_response_reason_missing,
        b"ICAP/1.0 200 \r\n\r\n",
        |res| {
            assert_eq!(res.version.unwrap(), 0);
            assert_eq!(res.code.unwrap(), 200);
            assert_eq!(res.reason.unwrap(), "");
        }
    }

    res! {
        test_response_reason_missing_no_space,
        b"ICAP/1.0 200\r\n\r\n",
        |res| {
            assert_eq!(res.version.unwrap(), 0);
            assert_eq!(res.code.unwrap(), 200);
            assert_eq!(res.reason.unwrap(), "");
        }
    }

    res! {
        test_response_reason_with_space_and_tab,
        b"ICAP/1.0 101 Switching Protocols\t\r\n\r\n",
        |res| {
            assert_eq!(res.version.unwrap(), 0);
            assert_eq!(res.code.unwrap(), 101);
            assert_eq!(res.reason.unwrap(), "Switching Protocols\t");
        }
    }

    static RESPONSE_REASON_WITH_OBS_TEXT_BYTE: &'static [u8] = b"ICAP/1.0 200 X\xFFZ\r\n\r\n";
    res! {
        test_response_reason_with_obsolete_text_byte,
        RESPONSE_REASON_WITH_OBS_TEXT_BYTE,
        Err(::Error::Status),
        |_res| {}
    }

    res! {
        test_response_reason_with_nul_byte,
        b"ICAP/1.0 200 \x00\r\n\r\n",
        Err(::Error::Status),
        |_res| {}
    }

    res! {
        test_response_version_missing_space,
        b"ICAP/1.0",
        Ok(Status::Partial),
        |_res| {}
    }

    res! {
        test_response_code_missing_space,
        b"ICAP/1.0 200",
        Ok(Status::Partial),
        |_res| {}
    }

    res! {
        test_response_empty_lines_prefix_lf_only,
        b"\n\nICAP/1.0 200 OK\n\n",
        |_res| {}
    }

    #[test]
    fn test_chunk_size() {
        assert_eq!(parse_chunk_size(b"0\r\n"), Ok(Status::Complete((3, 0))));
        assert_eq!(parse_chunk_size(b"12\r\nchunk"), Ok(Status::Complete((4, 18))));
        assert_eq!(parse_chunk_size(b"3086d\r\n"), Ok(Status::Complete((7, 198765))));
        assert_eq!(parse_chunk_size(b"3735AB1;foo bar*\r\n"), Ok(Status::Complete((18, 57891505))));
        assert_eq!(parse_chunk_size(b"3735ab1 ; baz \r\n"), Ok(Status::Complete((16, 57891505))));
        assert_eq!(parse_chunk_size(b"77a65\r"), Ok(Status::Partial));
        assert_eq!(parse_chunk_size(b"ab"), Ok(Status::Partial));
        assert_eq!(parse_chunk_size(b"567f8a\rfoo"), Err(::InvalidChunkSize));
        assert_eq!(parse_chunk_size(b"567f8a\rfoo"), Err(::InvalidChunkSize));
        assert_eq!(parse_chunk_size(b"567xf8a\r\n"), Err(::InvalidChunkSize));
        assert_eq!(parse_chunk_size(b"ffffffffffffffff\r\n"), Ok(Status::Complete((18, ::core::u64::MAX))));
        assert_eq!(parse_chunk_size(b"1ffffffffffffffff\r\n"), Err(::InvalidChunkSize));
        assert_eq!(parse_chunk_size(b"Affffffffffffffff\r\n"), Err(::InvalidChunkSize));
        assert_eq!(parse_chunk_size(b"fffffffffffffffff\r\n"), Err(::InvalidChunkSize));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_std_error() {
        use super::Error;
        use std::error::Error as StdError;
        let err = Error::HeaderName;
        assert_eq!(err.to_string(), err.description());
    }
}
