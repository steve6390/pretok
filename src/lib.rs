// Copyright 2020, Steve King
// See LICENSE.txt.
//! # pretok
//!
//! Pretok is a pre-tokenizer (or pre-lexer) for C-like syntaxes.  Pretok
//! simplifies subsequent lexers by handling line and block comments,
//! whitespace and strings. Pretok operates as an iterator over an input string
//! of UTF-8 code points.
//!
//! Given an input string, pretok does the following.
//! * Implements the iterator trait where ``next()`` returns a sequence of
//!   ``Option<Pretoken>`` structures.
//! * Filters ``// line comments`` from the input string.
//! * Filters ``/* block comments */`` from the input string
//! * Returns ``"quoted strings with \"escapes\""`` as a single ``Pretoken``.
//! * Skips whitespace characters.
//! * After above filters, returns ``Pretokens`` usually delineated by whitespace.
//! * Returns the line number and byte offset of each pretoken
//!
//! ## Motivation
//! Common computer language features such comments, line number tracking,
//! whitespace tolerance, etc. introduce corner cases that can make lexing
//! awkward.  By imposing a few opinions, the Pretokenizer solves these problems
//! at the earliest stage of processing. This preprocessing normalizes the input
//! stream and simplifies subsequent processing.
//!
//! # Basic Use
//! The [Pretokenizer] is an iterator returning a sequence of [Pretoken] objects
//! from an input string reference.  Normally each returned [Pretoken]
//! represents at least one actual language token.  A subsequent lexing step
//! would split [Pretoken]s into language tokens as needed.
//!
//! ## Examples
//!
//! Whitespace typically separates [Pretoken]s and is stripped outside of quoted strings.
//! ```
//!     use pretok::{Pretokenizer, Pretoken};
//!     let mut pt = Pretokenizer::new("Hello World!");
//!     assert!(pt.next() == Some(Pretoken{s:"Hello", line:1, offset:0}));
//!     assert!(pt.next() == Some(Pretoken{s:"World!", line:1, offset:6}));
//!     assert!(pt.next() == None);
//! ```
//! Comments are stripped and may also delineate [Pretoken]s.
//! ```
//!     use pretok::{Pretokenizer, Pretoken};
//!     let mut pt = Pretokenizer::new("x/*y*/z");
//!     assert!(pt.next() == Some(Pretoken{s:"x", line:1, offset:0}));
//!     assert!(pt.next() == Some(Pretoken{s:"z", line:1, offset:6}));
//!     assert!(pt.next() == None);
//!
//!     let mut pt = Pretokenizer::new("x\ny//z");
//!     assert!(pt.next() == Some(Pretoken{s:"x", line:1, offset:0}));
//!     assert!(pt.next() == Some(Pretoken{s:"y", line:2, offset:2}));
//!     assert!(pt.next() == None);
//! ```
//! Quoted strings are a single [Pretoken].
//! ```
//!     use pretok::{Pretokenizer, Pretoken};
//!     let mut pt = Pretokenizer::new("Hello \"W o r l d!\"");
//!     assert!(pt.next() == Some(Pretoken{s:"Hello", line:1, offset:0}));
//!     assert!(pt.next() == Some(Pretoken{s:"\"W o r l d!\"", line:1, offset:6}));
//!     assert!(pt.next() == None);
//! ```
//! Quoted strings create a single [Pretoken] separate from the surrounding pretoken(s).
//! ```
//!     use pretok::{Pretokenizer, Pretoken};
//!     let mut pt = Pretokenizer::new("x+\"h e l l o\"+z");
//!     assert!(pt.next() == Some(Pretoken{s:"x+", line:1, offset:0}));
//!     assert!(pt.next() == Some(Pretoken{s:"\"h e l l o\"", line:1, offset:2}));
//!     assert!(pt.next() == Some(Pretoken{s:"+z", line:1, offset:13}));
//!     assert!(pt.next() == None);
//! ```
//!
//! ## Unit Testing
//! Pretok supports unit tests.
//! <pre>
//! cargo test
//! </pre>
//! ## Fuzz Testing
//! Pretok supports fuzz tests.  Fuzz testing starts from a corpus of random
//! inputs and then further randomizes those inputs to try to cause crashes and
//! hangs.  At the time of writing (Rust 1.46.0), fuzz testing required the
//! nightly build.
//!
//! To run fuzz tests:
//! <pre>
//! rustup default nightly
//! cargo fuzz run fuzz_target_1
//! </pre>
//! You can leave the compiler on the nightly build or switch back to stable
//! with:
//! <pre>
//! rustup default stable
//! </pre>
//! Fuzz tests run until stopped with Ctrl-C.  In my experience, fuzz tests will
//! catch a problem almost immediately or not at all.
//!
//! Cargo fuzz use LLVM's libFuzzer internally, which provides a vast array of
//! runtime options.  To see thh options using the nightly compiler build:
//! <pre>
//! cargo fuzz run fuzz_target_1 -- -help=1
//! </pre>
//! For example, setting a smaller 5 second timeout for hangs:
//! <pre>
//! cargo fuzz run fuzz_target_1 -- -timeout=5
//! </pre>
//!
#![warn(clippy::all)]
#![warn(missing_docs)]
#![warn(missing_doc_code_examples)]
use strcursor::StrCursor;

/// A pretoken object contains a slice of the `Pretokenizer` input string
/// with lifetime a.
#[derive(Clone, Debug, PartialEq)]
pub struct Pretoken<'a> {
    /// The UTF-8 string slice.
    pub s: &'a str,
    /// Number > 0 of the _last_ line in this pretoken.
    pub line: usize,
    /// The byte offset of the first character in the pretoken.
    pub offset: usize,
}

impl<'a> Pretoken<'a> {
    /// The string for this pretoken is the slice between the specified cursors.
    /// * `start`: The starting code point (inclusive).
    /// * `end`: The end code point (exclusive).
    /// * `offset`: The byte offset of `start` from the front
    ///             of the string used to initialize the Pretokenizer.
    pub fn new(
        start: StrCursor<'a>,
        end: StrCursor<'a>, line: usize,
        offset: usize) -> Pretoken<'a> {
        Pretoken{ s:start.slice_between(end).unwrap(), line, offset}
    }
}


/// The Pretokenizer is an iterator that produces Option<[Pretoken]> objects over
/// an input string.
///
/// The Pretokenizer has a simple interface with only new() and next() functions.
/// ```
/// use pretok::{Pretokenizer, Pretoken};
/// let pt = Pretokenizer::new("a+b c// stuff\nd");
/// for tok in pt {
///     println!("{} found on line {}, offset {}",
///             tok.s, tok.line, tok.offset);
/// }
/// ```
/// <pre>
/// Produces the following output:
/// a+b found on line 1, offset 0
/// c found on line 1, offset 4
/// d found on line 2, offset 14
/// </pre>

#[derive(Clone, Debug)]
pub struct Pretokenizer<'a> {
    /// Cursor to the current code point in the input string
    pos: StrCursor<'a>,

    /// The current number of newlines encountered
    line: usize,
}

impl<'a> Pretokenizer<'a> {
    /// Create a new tokenizer
    pub fn new(s: &'a str) -> Pretokenizer {
        Pretokenizer{
            pos: StrCursor::new_at_start(s),
            line: 1,  // Line number are not zero-based
        }
    }

    fn make_pretok(&mut self, end: StrCursor<'a>) -> Option<Pretoken<'a>> {
        // If the current position hasn't moved, then return None.
        // This check simplifies corner cases like end-of-input.
        if end == self.pos {
            return None;
        }

        // Update the state of the Pretokenizer to the end of this pretoken.
        let start = self.pos;
        self.pos = end;
        Some(Pretoken::new(start, end, self.line, start.byte_pos()))
    }
}

/// Advances the internal iterator to the next pretoken. Skips whitespace
/// and comments. If the result is OK(None), then we successfully reached
/// end of the input string.
impl <'a> std::iter::Iterator for Pretokenizer<'a> {
    type Item = Pretoken<'a>;
    fn next(&mut self) -> Option<Self::Item> {

        #[derive(Debug)]
        enum STATE {
            WS,
            MaybeComment,
            LineComment,
            BlockComment,
            MaybeBlockCommentDone,
            StartTok,
            NormalTok,
            QuotedTok,
            EscapeChar,
        };

        // Start by skipping any whitespace
        let mut state = STATE::WS;

        // Get a local cursor starting at our current position.
        let mut curs = self.pos;

        loop {

            // Note that we're dealing with unicode code points rather
            // than grapheme clusters
            let copt = curs.cp_after();

            if copt.is_none() {
                // End of input!
                match state {
                    STATE::NormalTok => {
                        return self.make_pretok(curs);
                    }
                    STATE::BlockComment => {
                        // Unterminated block comment at end of input
                        // Caller may want to detect this and warn.
                    }
                    STATE::QuotedTok | STATE::EscapeChar => {
                        // Unterminated quoted string at end of input
                        // Caller may want to detect this and warn.
                        return self.make_pretok(curs);
                    }

                    _ => {}
                }

                self.pos = curs; // sync cursor position
                return None;
            }

            // Get the byte offset and character respectively
            let c = copt.unwrap();

            match state {
                STATE::WS => {
                    match c {
                        // need braces so each arm returns ()
                        '\n' => {
                            self.line += 1;
                            curs.seek_next_cp();
                        }
                        ' ' | '\t' => {
                            curs.seek_next_cp();
                        }
                        '/' => {
                            state = STATE::MaybeComment;
                            curs.seek_next_cp();
                        }
                        _ => state = STATE::StartTok,
                    }
                }

                // We enter the this state after peeking a '/' character.
                // We're looking for another '/' or '*'
                STATE::MaybeComment => {
                    match c {
                        '/' => {
                            // We're in a line comment.
                            state = STATE::LineComment;
                            curs.seek_next_cp();
                        }
                        '*' => {
                            // We're in a block comment.
                            state = STATE::BlockComment;
                            curs.seek_next_cp();
                        }
                        _ => state = STATE::StartTok,
                    }
                }

                STATE::LineComment => {
                    if c == '\n' {
                        // handle the new line WS state.
                        state = STATE::WS;
                    } else {
                        curs.seek_next_cp();
                    }
                }

                STATE::BlockComment => {
                    match c {
                        '*' => {
                            state = STATE::MaybeBlockCommentDone;
                        }
                        '\n' => {
                            self.line += 1;
                        }
                        _ => {}
                    }
                    curs.seek_next_cp();
                }

                STATE::MaybeBlockCommentDone => {
                    match c {
                        '/' => {
                            // Done with the block
                            state = STATE::WS;
                        }
                        '\n' => {
                            self.line += 1;
                            // false alarm, not done with block
                            state = STATE::BlockComment;
                        }
                        // False alarm, not done with the block
                        _ => { state = STATE::BlockComment; }
                    }
                    curs.seek_next_cp();
                }

                STATE::StartTok => {
                    // sync the real iterator with our temporary
                    // If this is a quoted string, the returned token
                    // will include the quote character.
                    self.pos = curs;

                    if c == '"' {
                        state = STATE::QuotedTok;
                        curs.seek_next_cp();
                    } else {
                        state = STATE::NormalTok;
                        curs.seek_next_cp();
                    }
                }

                STATE::NormalTok => {
                    match c {
                        ' ' | '\t' => {
                            // we'll process this ws on the next next()
                            return self.make_pretok(curs);
                        }
                        '\n' => {
                            // we'll process this newline on the next next()
                            return self.make_pretok(curs);
                        }
                        '"' => {
                            // We found quote without whitespace separation.
                            // Return whatever we've captured before the quote as the token.
                            // We'll process the quote on the next next()
                            return self.make_pretok(curs);
                        }
                        '/' => {
                            // We maybe found a comment without whitespace separation.
                            // Peek ahead one more character to know for sure.
                            let mut temp = curs;
                            temp.seek_next_cp();  // skip the / we're peeking at
                            let temp_copt = temp.cp_after();
                            if temp_copt.is_none() {
                                // There's nothing past the /.  Return current token
                                // including the / we're peeking at.
                                return self.make_pretok(temp);
                            } else {
                                match temp_copt.unwrap() {
                                    '/' | '*' => {
                                        // Found a comment, so return the preceding token
                                        return self.make_pretok(curs);
                                    }
                                    _ => {
                                        // False alarm, It was just a lonely / so keep going.
                                        curs.seek_next_cp();
                                    }
                                }
                            }
                        }
                        _ => { curs.seek_next_cp(); }
                    }
                }
                STATE::QuotedTok => {
                    match c {
                        '\n' => {
                            self.line +=1;
                        }
                        '"' => {
                            // We found the closing quote.  Advance the cursor so the
                            // closing quote is included in the returned token.
                            curs.seek_next_cp();
                            return self.make_pretok(curs);
                        }
                        '\\' => {
                            // We found an escape sequence.  Next character is always inside the string,
                            // if if it's another quote.
                            state = STATE::EscapeChar;
                        }
                        _ => { }
                    }
                    curs.seek_next_cp();
                }
                STATE::EscapeChar => {
                    if c == '\n' {
                        self.line +=1;
                    }
                    state = STATE::QuotedTok;
                    curs.seek_next_cp();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretokenizer_test_0() {
        let mut pt = Pretokenizer::new("");
        assert!(pt.next().is_none());
    }
    #[test]
    fn pretokenizer_test_1() {
        let mut pt = Pretokenizer::new("foo");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_2() {
        let mut pt = Pretokenizer::new("foo\n");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_3() {
        let mut pt = Pretokenizer::new("\nfoo");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 1);
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_4() {
        let mut pt = Pretokenizer::new("\nfoo\n");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 1);
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_5() {
        let mut pt = Pretokenizer::new("/* */foo");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 5);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_6() {
        let mut pt = Pretokenizer::new("\n/* */foo");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 6);
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_7() {
        let mut pt = Pretokenizer::new("\n/* */\nfoo");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 7);
        assert_eq!(t.line, 3);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_8() {
        let mut pt = Pretokenizer::new("// bar");
        let t = pt.next();
        assert!(t.is_none());
    }

    #[test]
    fn pretokenizer_test_9() {
        let mut pt = Pretokenizer::new("\n// bar");
        let t = pt.next();
        assert!(t.is_none());
    }

    #[test]
    fn pretokenizer_test_10() {
        let mut pt = Pretokenizer::new("// bar\n");
        let t = pt.next();
        assert!(t.is_none());
    }

    #[test]
    fn pretokenizer_test_11() {
        let mut pt = Pretokenizer::new("// bar\nfoo");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 7);
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_12() {
        let mut pt = Pretokenizer::new("// bar\n\nfoo");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 8);
        assert_eq!(t.line, 3);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_13() {
        let mut pt = Pretokenizer::new("\"");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\"");
    }

    #[test]
    fn pretokenizer_test_14() {
        let mut pt = Pretokenizer::new("\"\"");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\"\"");
    }

    #[test]
    fn pretokenizer_test_15() {
        let mut pt = Pretokenizer::new("\"x\"");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\"x\"");
    }

    #[test]
    fn pretokenizer_test_16() {
        let mut pt = Pretokenizer::new("\" x\"");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\" x\"");
    }

    #[test]
    fn pretokenizer_test_17() {
        let mut pt = Pretokenizer::new("\" x x \"");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\" x x \"");
    }

    #[test]
    fn pretokenizer_test_18() {
        let mut pt = Pretokenizer::new("//\" x x \"");
        let t = pt.next();
        assert!(t.is_none());
    }

    #[test]
    fn pretokenizer_test_19() {
        let mut pt = Pretokenizer::new("\"// x x \"");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\"// x x \"");
    }

    #[test]
    fn pretokenizer_test_20() {
        let mut pt = Pretokenizer::new("\" /* x x */ \"");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\" /* x x */ \"");
    }

    #[test]
    fn pretokenizer_test_21() {
        let mut pt = Pretokenizer::new("\" \\\" x \"");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\" \\\" x \"");
    }

    #[test]
    fn pretokenizer_test_22() {
        let mut pt = Pretokenizer::new("\" \\");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\" \\");
    }

    #[test]
    fn pretokenizer_test_23() {
        let mut pt = Pretokenizer::new("\" \\\"");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\" \\\"");
    }

    #[test]
    fn pretokenizer_test_24() {
        // Found by fuzz testing
        let mut pt = Pretokenizer::new("\" x\nx\"");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        // tokens that span lines are reported on the
        // last line of the token.
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "\" x\nx\"");
    }

    #[test]
    fn pretokenizer_test_25() {
        let mut pt = Pretokenizer::new("x//x");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "x");
    }

    #[test]
    fn pretokenizer_test_26() {
        let mut pt = Pretokenizer::new("x/*x*/");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "x");
    }

    #[test]
    fn pretokenizer_test_27() {
        let mut pt = Pretokenizer::new("x/*y*/z");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "x");
        // Now get the z.
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 6);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "z");
    }

    #[test]
    fn pretokenizer_test_28() {
        let mut pt = Pretokenizer::new("x y z");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "x");
        // Now get the 7.
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 2);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "y");
        // Now get the z.
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 4);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "z");
    }

    #[test]
    fn pretokenizer_test_29() {
        let mut pt = Pretokenizer::new("  x\n y\n   z");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 2);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "x");
        // Now get the 7.
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 5);
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "y");
        // Now get the z.
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 10);
        assert_eq!(t.line, 3);
        assert_eq!(t.s, "z");
    }

    #[test]
    fn pretokenizer_test_30() {
        let mut pt = Pretokenizer::new("  x // foo\ny\n   z");
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 2);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "x");
        // Now get the 7.
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 11);
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "y");
        // Now get the z.
        let t = pt.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 16);
        assert_eq!(t.line, 3);
        assert_eq!(t.s, "z");
    }

    #[test]
    fn pretokenizer_test_31() {
        let pt = Pretokenizer::new("a+b c// stuff\nd");
        for tok in pt {
            println!("{} found on line {}, offset {}",
                    tok.s, tok.line, tok.offset);
        }
    }
}


