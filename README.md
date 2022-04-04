# rivia
[![license-badge](https://img.shields.io/crates/l/rivia.svg)](https://opensource.org/licenses/MIT)
[![build](https://github.com/phR0ze/rivia/workflows/build/badge.svg?branch=main)](https://github.com/phR0ze/rivia/actions)
[![codecov](https://codecov.io/gh/phR0ze/rivia/branch/main/graph/badge.svg?token=DDWE2PFXNZ)](https://codecov.io/gh/phR0ze/rivia)
[![crates.io](https://img.shields.io/crates/v/rivia.svg)](https://crates.io/crates/rivia)
[![Minimum rustc](https://img.shields.io/badge/rustc-1.30+-lightgray.svg)](https://github.com/phR0ze/rivia#rustc-requirements)

***Rust utilities to reduce code verbosity***

***rivia*** provides low level functionality to facilitate system level interaction. The intent is
to reduce the need for boiler plate code and deliver a simplified api consummable by higher level
applications. As such the crate is broken into top level modules grouped by logical categories.
***rivia*** is a rewrite of ***fungus*** with a focus on reducing dependencies and/or shifting
them into optional separate crates.

### Disclaimer
***rivia*** comes with absolutely no guarantees or support of any kind. It is to be used at
your own risk.  Any damages, issues, losses or problems caused by the use of rivia are
strictly the responsiblity of the user and not the developer/creator of rivia.

### Features by category
* **User Management** - *XDG Support*, *User ID mangement*
* **FileSystem** - *Path extensions*, *Virtual FileSystem*

### Quick links
* [Usage](#usage)
  * [Rustc requirments](#rustc-requirements)
* [Contribute](#contribute)
  * [Dev Environment](#dev-environment)
    * [Automatic version](#automatic-version)
  * [Testing](#testing)
    * [Test in container](#test-in-container)
* [License](#license)
  * [Contribution](#contribution)
* [Backlog](#backlog)
* [Changelog](#changelog)

# Usage

#### Rustc requirements
This minimum rustc requirement is driven by the enhancements made to [Rust's `std::error::Error`
handling improvements](https://doc.rust-lang.org/std/error/trait.Error.html#method.source)

# Contribute
Pull requests are always welcome. However understand that they will be evaluated purely on whether
or not the change fits with my goals/ideals for the project.

**Project guidelines**:
* ***Chaining*** - ensure Rust's functional chaining style isn't impeded by additions
* ***Brevity*** - keep the naming as concise as possible while not infringing on clarity
* ***Clarity*** - keep the naming as unambiguous as possible while not infringing on brevity
* ***Performance*** - keep convenience functions as performant as possible while calling out significant costs
* ***Speed*** - provide ergonomic functions similar to rapid development languages
* ***Comfort*** - use naming and concepts in similar ways to popular languages

## Dev Environment

### Automatic version
Enable the git hooks to have the version automatically incremented on commits

```bash
cd ~/Projects/rivia
git config core.hooksPath .githooks
```

## Testing

### Host dependencies
Due to the low level nature of some of the funtionality that `rivia` provides testing requires a few
dependencies in the host system where the tests are being run.

* `sudo`
* `touch`

### Test in container
Build the test container using the code in `examples/cli.rs`

**Build container**:
```bash
$ docker build -f Dockerfile.test -t rivia-test .
```

**Run container**:
```bash
$ docker run --rm rivia-test:latest
```

**Debug in container**:
```bash
$ docker run --rm -it rivia-test:latest bash
```

# License
This project is licensed under either of:
 * MIT license [LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT
 * Apache License, Version 2.0 [LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0

## Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this project by you, shall be dual licensed as above, without any additional terms or conditions.

---

# Backlog

# Changelog
* VFS Memfs
* VFS Stdfs
