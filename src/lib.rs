// Pretok returns pretokens, which are filtered strings in the
// input file.  The pre-tokenizer does the following
// * Skips whitespace and comments
// * Returns pretoken strings from next()
// * Records line and byte offset for each pretoken

#![warn(clippy::all)]
use strcursor::StrCursor;

/// Track information about pretokens
#[derive(Clone, Debug)]
pub struct Pretoken<'a> {
    /// The UTF-8 string for this pretoken
    pub s: &'a str,

    /// Line number > 0 start the start of this pretoken
    pub line: usize,

    /// The byte offset of the first character in the pretoken
    pub offset: usize,
}

impl<'a> Pretoken<'a> {
    // The string for this pretoken is the slice between the start and end cursors
    pub fn new(start: StrCursor<'a>, end: StrCursor<'a>, line: usize,
            offset: usize) -> Pretoken<'a> {
        Pretoken{ s:start.slice_between(end).unwrap(), line, offset}
    }
}

/******************************************************************************
 * Tokenizer
 *****************************************************************************/
pub struct PreTokenizer<'input> {
    /// Iterator to the current code point offset in the input string
    /// This iterator returns a pair with:
    /// 1) the next UTF-8 character, which may occupy more than one byte
    /// 2) The byte offset into the string
    pos: StrCursor<'input>,

    /// The current number of newlines encountered
    line: usize,
}

impl<'input> PreTokenizer<'input> {
    /// Create a new tokenizer
    pub fn new(s: &'input str) -> PreTokenizer {
        PreTokenizer{
            pos: StrCursor::new_at_start(s),
            line: 1,  // Line number are not zero-based
        }
    }

    pub fn make_pretok(&mut self, end: StrCursor<'input>) -> Option<Pretoken<'input>> {
        // If the current position hasn't moved, then return None.
        // This check simplifies corner cases like end-of-input.
        if end == self.pos {
            return None;
        }

        // Update the state of the pretokenizer to the end of this pretoken.
        let start = self.pos;
        self.pos = end;
        Some(Pretoken::new(start, end, self.line, start.byte_pos()))
    }
}

/// Advances the internal iterator to the next pretoken. Skips whitespace
/// and comments. If the result is OK(None), then we successfully reached
/// end of the input string.
impl <'a> std::iter::Iterator for PreTokenizer<'a> {
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
        let mut pretok = PreTokenizer::new("");
        assert!(pretok.next().is_none());
    }
    #[test]
    fn pretokenizer_test_1() {
        let mut pretok = PreTokenizer::new("foo");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_2() {
        let mut pretok = PreTokenizer::new("foo\n");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_3() {
        let mut pretok = PreTokenizer::new("\nfoo");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 1);
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_4() {
        let mut pretok = PreTokenizer::new("\nfoo\n");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 1);
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_5() {
        let mut pretok = PreTokenizer::new("/* */foo");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 5);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_6() {
        let mut pretok = PreTokenizer::new("\n/* */foo");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 6);
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_7() {
        let mut pretok = PreTokenizer::new("\n/* */\nfoo");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 7);
        assert_eq!(t.line, 3);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_8() {
        let mut pretok = PreTokenizer::new("// bar");
        let t = pretok.next();
        assert!(t.is_none());
    }

    #[test]
    fn pretokenizer_test_9() {
        let mut pretok = PreTokenizer::new("\n// bar");
        let t = pretok.next();
        assert!(t.is_none());
    }

    #[test]
    fn pretokenizer_test_10() {
        let mut pretok = PreTokenizer::new("// bar\n");
        let t = pretok.next();
        assert!(t.is_none());
    }

    #[test]
    fn pretokenizer_test_11() {
        let mut pretok = PreTokenizer::new("// bar\nfoo");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 7);
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_12() {
        let mut pretok = PreTokenizer::new("// bar\n\nfoo");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 8);
        assert_eq!(t.line, 3);
        assert_eq!(t.s, "foo");
    }

    #[test]
    fn pretokenizer_test_13() {
        let mut pretok = PreTokenizer::new("\"");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\"");
    }

    #[test]
    fn pretokenizer_test_14() {
        let mut pretok = PreTokenizer::new("\"\"");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\"\"");
    }

    #[test]
    fn pretokenizer_test_15() {
        let mut pretok = PreTokenizer::new("\"x\"");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\"x\"");
    }

    #[test]
    fn pretokenizer_test_16() {
        let mut pretok = PreTokenizer::new("\" x\"");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\" x\"");
    }

    #[test]
    fn pretokenizer_test_17() {
        let mut pretok = PreTokenizer::new("\" x x \"");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\" x x \"");
    }

    #[test]
    fn pretokenizer_test_18() {
        let mut pretok = PreTokenizer::new("//\" x x \"");
        let t = pretok.next();
        assert!(t.is_none());
    }

    #[test]
    fn pretokenizer_test_19() {
        let mut pretok = PreTokenizer::new("\"// x x \"");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\"// x x \"");
    }

    #[test]
    fn pretokenizer_test_20() {
        let mut pretok = PreTokenizer::new("\" /* x x */ \"");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\" /* x x */ \"");
    }

    #[test]
    fn pretokenizer_test_21() {
        let mut pretok = PreTokenizer::new("\" \\\" x \"");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\" \\\" x \"");
    }

    #[test]
    fn pretokenizer_test_22() {
        let mut pretok = PreTokenizer::new("\" \\");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\" \\");
    }

    #[test]
    fn pretokenizer_test_23() {
        let mut pretok = PreTokenizer::new("\" \\\"");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "\" \\\"");
    }

    #[test]
    fn pretokenizer_test_24() {
        // Found by fuzz testing
        let mut pretok = PreTokenizer::new("\" x\nx\"");
        let t = pretok.next();
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
        let mut pretok = PreTokenizer::new("x//x");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "x");
    }

    #[test]
    fn pretokenizer_test_26() {
        let mut pretok = PreTokenizer::new("x/*x*/");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "x");
    }

    #[test]
    fn pretokenizer_test_27() {
        let mut pretok = PreTokenizer::new("x/*y*/z");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "x");
        // Now get the z.
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 6);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "z");
    }

    #[test]
    fn pretokenizer_test_28() {
        let mut pretok = PreTokenizer::new("x y z");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 0);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "x");
        // Now get the 7.
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 2);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "y");
        // Now get the z.
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 4);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "z");
    }

    #[test]
    fn pretokenizer_test_29() {
        let mut pretok = PreTokenizer::new("  x\n y\n   z");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 2);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "x");
        // Now get the 7.
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 5);
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "y");
        // Now get the z.
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 10);
        assert_eq!(t.line, 3);
        assert_eq!(t.s, "z");
    }

    #[test]
    fn pretokenizer_test_30() {
        let mut pretok = PreTokenizer::new("  x // foo\ny\n   z");
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 2);
        assert_eq!(t.line, 1);
        assert_eq!(t.s, "x");
        // Now get the 7.
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 11);
        assert_eq!(t.line, 2);
        assert_eq!(t.s, "y");
        // Now get the z.
        let t = pretok.next();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.offset, 16);
        assert_eq!(t.line, 3);
        assert_eq!(t.s, "z");
    }

}


