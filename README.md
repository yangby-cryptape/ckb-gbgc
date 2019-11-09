# [CKB Genesis Block Generator (GBG)] Candidate

[![License]](#license)
[![Travis CI]](https://travis-ci.com/yangby-cryptape/ckb-gbgc)

[Unofficial & Experimental] [CKB Genesis Block Generator (GBG)] Candidate.

[License]: https://img.shields.io/badge/License-Apache--2.0%20OR%20MIT-blue.svg
[Travis CI]: https://img.shields.io/travis/com/yangby-cryptape/ckb-gbgc.svg

## Usage

```bash
# Set the level of logger, default is "warn,ckb_gbgc=info"
# #export GBGC_LOG=warn,ckb_gbgc=trace
cargo run --release -- \
    --url "http://YOUR-CKB-JSONRPC-SERVER-ADDRESS:PORT" \
    --epoch DEFAULT-IS-89 \
    --output "THE-OUTPUT-SPEC-TOML"
```

## License

Licensed under either of [Apache License, Version 2.0] or [MIT License], at
your option.

[Apache License, Version 2.0]: LICENSE-APACHE
[MIT License]: LICENSE-MIT

[CKB Genesis Block Generator (GBG)]: https://medium.com/nervosnetwork/a-decentralized-mainnet-launch-for-nervos-ckb-9cb119d15540
