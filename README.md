<div align="center">

[![CI status](https://github.com/cakevm/huff-neo/actions/workflows/ci.yaml/badge.svg?branch=main)][gh-huff-neo]
[![Telegram Chat][tg-badge]][tg-url]


[gh-huff-neo]: https://github.com/cakevm/huff-neo/actions/workflows/ci.yaml
[tg-badge]: https://img.shields.io/badge/telegram-huff_neo-2CA5E0?style=plastic&logo=telegram
[tg-url]: https://t.me/huff_neo

</div>

# huff-neo

`huff-neo` marks a new dawn for the once-abandoned [huff-rs](https://github.com/huff-language/huff-rs), breathing fresh life into its legacy. This is a hard-fork with update dependencies and brings the project up-to-date with the latest Rust version and dependencies. 

## What is a Huff?

Huff is a low-level programming language designed for developing highly optimized smart contracts. For a more detailed explanation, see the original repository [huff-rs](https://github.com/huff-language/huff-rs).

## How about huff2?

We are very happy that someone picked up the work. In the meantime we still need some compiler to work with. We are trying to keep the original compiler up-to-date with the latest dependencies. Many, many thanks in advance to the [huff2](https://github.com/huff-language/huff2) team!

## Installation

You can use the installer `huff-neo-up` to install the latest version of `huff-neo`:
```bash
curl -L https://raw.githubusercontent.com/cakevm/huff-neo/refs/heads/main/huff-neo-up/install | bash
huff-neo-up
```

As alternative, you can build it yourself by cloning the repository and running the following command:
```bash
make release
```

## Modules

- [core](crates/core): The core module to huff-neo-rs. Resolves source file paths, executes compilation, and exports artifacts.
- [cli](bin/huff-neo): The command line interface for the Huff compiler.
- [js](crates/js): A wasm compatible interface to the Huff compiler for JavaScript bindings.
- [lexer](crates/lexer): Takes in the source of a `.huff` file and generates a vector of `Token`s.
- [parser](crates/parser): Crafts a `Contract` AST from the vector of `Token`s generated by [huff-lexer](crates/lexer).
- [codegen](crates/codegen): EVM Bytecode generation module that accepts an AST generated by [huff-parser](crates/parser).
- [utils](crates/utils): Various utilities and types used by all modules.
- [huff-neo-up](./huff-neo-up): Update or revert to a specific huff-neo-rs branch with ease. (Forked from [foundry](https://github.com/foundry-rs/foundry))

## Contributing

Feel free to create any issue or PR. We are always looking for contributors to help us improve the project.

Before submitting a PR, please make sure to run the following commands:
```bash
cargo check --all
cargo test --all --all-features
cargo fmt -- --check
cargo clippy --all --all-features -- -D warnings
```

## Safety

Please be aware that the resulting bytecode can be unsafe. It is your responsibility to ensure that the contracts are safe and secure. The authors of this project are not responsible for any misuse or loss of funds.

## Acknowledgements

Many thanks to all [huff-rs](https://github.com/huff-language/huff-rs) contributors and to the authors wo maintained it for such a long period! Again thanks to the original [Huff Language](https://github.com/huff-language) compiler: [`huffc`](https://github.com/huff-language/huffc). Thanks to [ripc](https://github.com/ibraheemdev/ripc), and big shoutout to [Paradigm](https://github.com/paradigmxyz). Without [Foundry](https://github.com/foundry-rs/foundry) the original implementation would not be possible.

## License
This project is as the original huff-rs dual licensed under [Apache 2.0](./LICENSE-APACHE) or [MIT](./LICENSE-MIT) licence.