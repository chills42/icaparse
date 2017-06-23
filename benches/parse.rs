#![feature(test)]

extern crate icaparse;

extern crate test;

const REQ: &'static [u8] = b"\
RESPMOD /wp-content/uploads/2010/03/hello-kitty-darth-vader-pink.jpg ICAP/1.0\r\n\
Host: www.kittyhell.com\r\n\
User-Agent: Mozilla/5.0 (Macintosh; U; Intel Mac OS X 10.6; ja-JP-mac; rv:1.9.2.3) Gecko/20100401 Firefox/3.6.3 Pathtraq/0.9\r\n\
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8\r\n\
Accept-Language: ja,en-us;q=0.7,en;q=0.3\r\n\
Accept-Encoding: gzip,deflate\r\n\
Accept-Charset: Shift_JIS,utf-8;q=0.7,*;q=0.7\r\n\
Keep-Alive: 115\r\n\
Connection: keep-alive\r\n\
Cookie: wp_ozh_wsa_visits=2; wp_ozh_wsa_visit_lasttime=xxxxxxxxxx; __utma=xxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.x; __utmz=xxxxxxxxx.xxxxxxxxxx.x.x.utmccn=(referral)|utmcsr=reader.livedoor.com|utmcct=/reader/|utmcmd=referral\r\n\
Encapsulated:null-body=0\r\n\r\n";

#[bench]
fn bench_icaparse(b: &mut test::Bencher) {
    let mut headers = [icaparse::Header{ name: "", value: &[] }; 16];
    let mut req = icaparse::Request::new(&mut headers);
    b.iter(|| {
        assert_eq!(req.parse(REQ).unwrap(), icaparse::Status::Complete(REQ.len()));
    });
    b.bytes = REQ.len() as u64;
}

const OPTIONS_REQ: &'static [u8] = b"\
OPTIONS icap://icap.server.net/sample-service ICAP/1.0\r\n\
Host: icap.server.net\r\n\
User-Agent: BazookaDotCom-ICAP-Client-Library/2.3\r\n\r\n";

#[bench]
fn bench_icaparse_options(b: &mut test::Bencher) {
    let mut headers = [icaparse::Header{ name: "", value: &[] }; 16];
    let mut req = icaparse::Request::new(&mut headers);
    b.iter(|| {
        assert_eq!(req.parse(OPTIONS_REQ).unwrap(), icaparse::Status::Complete(OPTIONS_REQ.len()));
    });
    b.bytes = OPTIONS_REQ.len() as u64;
}

const REQMOD_GET_REQ: &'static [u8] = b"\
REQMOD icap://icap-server.net/server?arg=87 ICAP/1.0\r\n\
Host: icap-server.net\r\n\
Encapsulated: req-hdr=0, null-body=170\r\n\
\r\n\
GET / HTTP/1.1\r\n\
Host: www.origin-server.com\r\n\
Accept: text/html, text/plain\r\n\
Accept-Encoding: compress\r\n\
Cookie: ff39fk3jur@4ii0e02i\r\n\
If-None-Match: \"xyzzy\", \"r2d2xxxx\"\r\n\r\n";

#[bench]
fn bench_icaparse_reqmod_get(b: &mut test::Bencher) {
    let mut headers = [icaparse::Header{ name: "", value: &[] }; 16];
    let mut req = icaparse::Request::new(&mut headers);
    b.iter(|| {
        assert_eq!(req.parse(REQMOD_GET_REQ).unwrap(), icaparse::Status::Complete(REQMOD_GET_REQ.len()));
    });
    b.bytes = REQMOD_GET_REQ.len() as u64;
}

const REQMOD_POST_REQ: &'static [u8] = b"\
REQMOD icap://icap-server.net/server?arg=87 ICAP/1.0\r\n\
Host: icap-server.net\r\n\
Encapsulated: req-hdr=0, req-body=147\r\n\
\r\n\
POST /origin-resource/form.pl HTTP/1.1\r\n\
Host: www.origin-server.com\r\n\
Accept: text/html, text/plain\r\n\
Accept-Encoding: compress\r\n\
Pragma: no-cache\r\n\
\r\n\
1e\r\n\
I am posting this information.\r\n\
0\r\n\r\n";

#[bench]
fn bench_icaparse_reqmod_post(b: &mut test::Bencher) {
    let mut headers = [icaparse::Header{ name: "", value: &[] }; 16];
    let mut req = icaparse::Request::new(&mut headers);
    b.iter(|| {
        assert_eq!(req.parse(REQMOD_POST_REQ).unwrap(), icaparse::Status::Complete(REQMOD_POST_REQ.len()));
    });
    b.bytes = REQMOD_POST_REQ.len() as u64;
}

const RESPMOD_REQ: &'static [u8] = b"\
RESPMOD icap://icap.example.org/satisf ICAP/1.0\r\n\
Host: icap.example.org\r\n\
Encapsulated: req-hdr=0, res-hdr=137, res-body=296\r\n\
\r\n\
GET /origin-resource HTTP/1.1\r\n\
Host: www.origin-server.com\r\n\
Accept: text/html, text/plain, image/gif\r\n\
Accept-Encoding: gzip, compress\r\n\
\r\n\
HTTP/1.1 200 OK\r\n\
Date: Mon, 10 Jan 2000 09:52:22 GMT\r\n\
Server: Apache/1.3.6 (Unix)\r\n\
ETag: \"63840-1ab7-378d415b\"\r\n\
Content-Type: text/html\r\n\
Content-Length: 51\r\n\
\r\n\
33\r\n\
This is data that was returned by an origin server.\r\n\
0\r\n\r\n";

#[bench]
fn bench_icaparse_respmod(b: &mut test::Bencher) {
    let mut headers = [icaparse::Header{ name: "", value: &[] }; 16];
    let mut req = icaparse::Request::new(&mut headers);
    b.iter(|| {
        assert_eq!(req.parse(RESPMOD_REQ).unwrap(), icaparse::Status::Complete(RESPMOD_REQ.len()));
    });
    b.bytes = RESPMOD_REQ.len() as u64;
}
