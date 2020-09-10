# pretok

Pretok is a pre-tokenizer for C-like syntaxes.  Pretok simplifies subsequent tokenizers by handling line and block comments, whitespace and strings.  Pretok operates as an iterator over an input string of UTF-8 code points.

Given an input string, pretok does the following.
* Implements the iterator trait where ``next()`` returns a sequence of ``Option<Pretoken>`` structures.
* Filters ``// line comments`` from the input string.
* Filters ``/* block comments */`` from the input string
* Returns ``"quoted strings with \"escapes\""`` as a single ``Pretoken``.
* Skips whitespace characters.
* After above filters, returns ``Pretokens`` usually delineated by whitespace.
* Returns the line number and byte offset of each pretoken


## Examples

Whitespace typically separates tokens and is stripped outside of quoted strings.

    let mut pretok = PreTokenizer::new("Hello World!");
    assert!(pretok.next() == Some(Pretoken{s:"Hello", line:1, offset:0}));
    assert!(pretok.next() == Some(Pretoken{s:"World!", line:1, offset:6}));
    assert!(pretok.next() == None);

Comments are stripped and may also delineate tokens.

    let mut pretok = PreTokenizer::new("x/*y*/z");
    assert!(pretok.next() == Some(Pretoken{s:"x", line:1, offset:0}));
    assert!(pretok.next() == Some(Pretoken{s:"z", line:1, offset:6}));
    assert!(pretok.next() == None);

    let mut pretok = PreTokenizer::new("x\ny//z");
    assert!(pretok.next() == Some(Pretoken{s:"x", line:1, offset:0}));
    assert!(pretok.next() == Some(Pretoken{s:"y", line:2, offset:2}));
    assert!(pretok.next() == None);

Quoted strings are a single token.

    let mut pretok = PreTokenizer::new("Hello \"W o r l d!\"");
    assert!(pretok.next() == Some(Pretoken{s:"Hello", line:1, offset:0}));
    assert!(pretok.next() == Some(Pretoken{s:"\"W o r l d!\"", line:1, offset:6}));
    assert!(pretok.next() == None);

Quoted strings create a single token separate from the surrounding token(s).

    let mut pretok = PreTokenizer::new("x+\"h e l l o\"+z");
    assert!(pretok.next() == Some(Pretoken{s:"x+", line:1, offset:0}));
    assert!(pretok.next() == Some(Pretoken{s:"\"h e l l o\"", line:1, offset:2}));
    assert!(pretok.next() == Some(Pretoken{s:"+z", line:1, offset:13}));
    assert!(pretok.next() == None);


## Unit Testing
Pretok support unit tests.

    cargo test

## Fuzz Testing
Pretok supports fuzz tests.  Fuzz testing starts from a corpus of random inputs and then further randomizes those inputs to try to cause crashes and hangs.  At the time of writing (Rust 1.46.0), fuzz testing required the nightly build.

To run fuzz tests:

    rustup default nightly
    cargo fuzz run fuzz_target_1

You can leave the compiler on the nightly build or switch back to stable with:

    rustup default stable

Fuzz tests run until stopped with Ctrl-C.  In my experience, fuzz tests will catch a problem almost immediately or not at all.

Cargo fuzz use LLVM's libFuzzer internally, which provides a vast array of runtime options.  To see thh options using the nightly compiler build:

    cargo fuzz run fuzz_target_1 -- -help=1

For example, setting a smaller 5 second timeout for hangs:

    cargo fuzz run fuzz_target_1 -- -timeout=5

