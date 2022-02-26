# rivia

***Rust utilities to reduce code verbosity***

***rivia*** provides low level functionality to facilitate system level interaction. The intent is
to reduce the need for boiler plate code and deliver a simplified api consummable by higher level
applications. As such the crate is broken into top level modules grouped by logical categories.
***rivia*** is a rewrite of ***fungus*** with a focus on reducing dependencies and/or shifting
them into optional separate crates.

***rivia*** comes with absolutely no guarantees or support of any kind. It is to be used at
your own risk.  Any damages, issues, losses or problems caused by the use of ***rivia*** are
strictly the responsiblity of the user and not the developer/creator of ***rivia***.

### Features by category <a name="features-by-category"/></a>

* **User Management** - ***XDG Support***, ***User ID mangement***
* **Virtual FileSystem** - ***Path extensions***, ***VFS Support***

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

# Usage <a name="usage"/></a>

#### Requires rustc >= 1.30 <a name="rustc-requirements"/></a>
This minimum rustc requirement is driven by the enhancements made to [Rust's `std::error::Error`
handling improvements](https://doc.rust-lang.org/std/error/trait.Error.html#method.source)

# Contribute <a name="Contribute"/></a>
Pull requests are always welcome. However understand that they will be evaluated purely on whether
or not the change fits with my goals/ideals for the project.

**Project guidelines**:
* ***Chaining*** - ensure Rust's functional chaining style isn't impeded by additions
* ***Brevity*** - keep the naming as concise as possible while not infringing on clarity
* ***Clarity*** - keep the naming as unambiguous as possible while not infringing on brevity
* ***Performance*** - keep convenience functions as performant as possible while calling out significant costs
* ***Speed*** - provide ergonomic functions similar to rapid development languages
* ***Comfort*** - use naming and concepts in similar ways to popular languages

## Dev Environment <a name="dev-environment"/></a>

### Automatic version <a name="automatic-version"/></a>
Enable the git hooks to have the version automatically incremented on commits

```bash
cd ~/Projects/rivia
git config core.hooksPath .githooks
```

## Testing <a name="testing"/></a>

### Host dependencies <a name="host-dependencies"/></a>
Due to the low level nature of some of the funtionality that `rivia` provides testing requires a few
dependencies in the host system where the tests are being run.

* `sudo`
* `touch`

### Test in container <a name="test-in-container"/></a>
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

# License <a name="license"/></a>
This project is licensed under either of:
 * MIT license [LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT
 * Apache License, Version 2.0 [LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0

## Contribution <a name="contribution"/></a>
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this project by you, shall be dual licensed as above, without any additional terms or conditions.

---

# Backlog <a name="backlog"/></a>

# Changelog <a name="changelog"/></a>
* VFS Memfs
* VFS Stdfs