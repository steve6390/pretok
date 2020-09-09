# pretok

Pretok is a pre-tokenizer intended to simplify subsequent tokenizers.  Pretok assumes C-like syntax for comments, whitespace and string handling.  Pretok operates as an iterator over an input string.

Given an input string, pretok does the following.
* Implements the iterator trait where ``next()`` returns a sequence of ``Option<Pretoken>`` structures.
* Filters ``// line comments`` from the input string.
* Filters ``/* block comments */`` from the input string
* Returns ``"quoted strings with \"escapes\""`` as a single ``Pretoken``.
* Skips consecutive whitespace characters.
* After above filters, returns ``Pretokens`` on whitespace boundaries.
* Returns the line number and byte offset of each pretoken

Pretok operates on UTF-8 code points.