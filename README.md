# httparse

[![crates.io](http://meritbadge.herokuapp.com/icaparse)](https://crates.io/crates/icaparse)

A push parser for the ICAP 1.0 protocol. Avoids allocations. Fast.

The goal of the library is to support the ICAP specification as defined in [RFC 3507](https://tools.ietf.org/html/rfc3507).

[Documentation](https://docs.rs/icaparse)

## Usage

```rust
let mut headers = [icaparse::EMPTY_HEADER; 16];
let mut req = icaparse::Request::new(&mut headers);

let buf = b"RESPMOD /index.html ICAP/1.0\r\nHost";
assert!(try!(req.parse(buf)).is_partial());

// a partial request, so we try again once we have more data

let buf = b"RESPMOD /index.html ICAP/1.0\r\nHost: example.domain\r\nEncapsulated:null-body=0\r\n\r\n";
assert!(try!(req.parse(buf)).is_complete());
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
