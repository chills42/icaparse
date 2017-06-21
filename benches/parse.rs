#![feature(test)]

extern crate icaparse;

extern crate test;

const REQ: &'static [u8] = b"\
RESPMOD /wp-content/uploads/2010/03/hello-kitty-darth-vader-pink.jpg ICAP/1.1\r\n\
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
