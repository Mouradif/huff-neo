use huff_neo_utils::file::full_file_source::FullFileSource;
use huff_neo_utils::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::iter::Enumerate;
use std::{iter::Peekable, str::Chars};

lazy_static! {
    static ref TOKEN: HashMap<String, TokenKind> = HashMap::from_iter(vec![
        (TokenKind::Macro.to_string(), TokenKind::Macro),
        (TokenKind::Fn.to_string(), TokenKind::Fn),
        (TokenKind::Test.to_string(), TokenKind::Test),
        (TokenKind::Function.to_string(), TokenKind::Function),
        (TokenKind::Constant.to_string(), TokenKind::Constant),
        (TokenKind::Error.to_string(), TokenKind::Error),
        (TokenKind::Takes.to_string(), TokenKind::Takes),
        (TokenKind::Returns.to_string(), TokenKind::Returns),
        (TokenKind::Event.to_string(), TokenKind::Event),
        (TokenKind::NonPayable.to_string(), TokenKind::NonPayable),
        (TokenKind::Payable.to_string(), TokenKind::Payable),
        (TokenKind::Indexed.to_string(), TokenKind::Indexed),
        (TokenKind::View.to_string(), TokenKind::View),
        (TokenKind::Pure.to_string(), TokenKind::Pure),
        // First check for packed jump table
        (TokenKind::JumpTablePacked.to_string(), TokenKind::JumpTablePacked),
        // Match with jump table if not
        (TokenKind::JumpTable.to_string(), TokenKind::JumpTable),
        (TokenKind::CodeTable.to_string(), TokenKind::CodeTable),
    ]);
}

/// Defines a context in which the lexing happens.
/// Allows to differentiate between EVM types and opcodes that can either
/// be identical or the latter being a substring of the former (example : bytes32 and byte)
#[derive(Debug, PartialEq, Eq)]
pub enum Context {
    /// global context
    Global,
    /// Macro definition context
    MacroDefinition,
    /// Macro's body context
    MacroBody,
    /// Macro's argument context (definition or being called)
    MacroArgs,
    /// ABI context
    Abi,
    /// Lexing args of functions inputs/outputs and events
    AbiArgs,
    /// constant context
    Constant,
    /// Code table context
    CodeTableBody,
    // Built-in function context
    BuiltinFunction,
}

/// ## Lexer
///
/// The lexer encapsulated in a struct.
pub struct Lexer<'a> {
    /// The source code as peekable chars.
    /// WARN: SHOULD NEVER BE MODIFIED!
    pub chars: Peekable<Enumerate<Chars<'a>>>,
    position: usize,
    /// The previous lexed Token.
    /// NOTE: Cannot be a whitespace.
    pub lookback: Option<Token>,
    /// Bool indicating if we have reached EOF
    pub eof: bool,
    /// Current context.
    pub context: Context,
    /// The raw source code.
    pub source: FullFileSource<'a>,
}

pub type TokenResult = Result<Token, LexicalError>;

impl<'a> Lexer<'a> {
    pub fn new(source: FullFileSource<'a>) -> Self {
        Lexer {
            chars: source.source.chars().enumerate().peekable(),
            position: 0,
            lookback: None,
            eof: false,
            context: Context::Global,
            source,
        }
    }

    /// Consumes the next character
    pub fn consume(&mut self) -> Option<char> {
        let (index, c) = self.chars.next()?;
        self.position = index;
        Some(c)
    }

    /// Try to peek at the next character from the source
    pub fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    fn next_token(&mut self) -> TokenResult {
        if let Some(ch) = self.consume() {
            let token = match ch {
                '/' => {
                    let mut comment_string = String::new();
                    let start = self.position;
                    comment_string.push(ch);
                    if let Some(ch2) = self.peek() {
                        match ch2 {
                            '/' => {
                                // Consume until newline
                                comment_string.push(ch2);
                                let (comment_string, start, end) = self.eat_while(Some(ch), |c| c != '\n');
                                Ok(TokenKind::Comment(comment_string).into_token_with_span(self.source.relative_span_by_pos(start, end)))
                            }
                            '*' => {
                                // ref: https://github.com/rust-lang/rust/blob/900c3540378c8422b8087ffa3db60fa6c8abfcad/compiler/rustc_lexer/src/lib.rs#L474
                                let c = self.consume();
                                comment_string.push(c.unwrap());
                                let mut depth = 1usize;
                                while let Some(c) = self.consume() {
                                    match c {
                                        '/' if self.peek() == Some('*') => {
                                            comment_string.push(c);
                                            let c2 = self.consume();
                                            comment_string.push(c2.unwrap());
                                            depth += 1;
                                        }
                                        '*' if self.peek() == Some('/') => {
                                            comment_string.push(c);
                                            let c2 = self.consume();
                                            comment_string.push(c2.unwrap());
                                            depth -= 1;
                                            if depth == 0 {
                                                // This block comment is closed, so for a
                                                // construction like "/* */ */"
                                                // there will be a successfully parsed block comment
                                                // "/* */"
                                                // and " */" will be processed separately.
                                                break;
                                            }
                                        }
                                        _ => {
                                            comment_string.push(c);
                                        }
                                    }
                                }

                                Ok(TokenKind::Comment(comment_string)
                                    .into_token_with_span(self.source.relative_span_by_pos(start, self.position)))
                            }
                            _ => self.single_char_token(TokenKind::Div),
                        }
                    } else {
                        self.single_char_token(TokenKind::Div)
                    }
                }

                // # keywords
                '#' => {
                    let (word, start, end) = self.eat_while(Some(ch), |ch| ch.is_ascii_alphabetic());

                    let mut found_kind: Option<TokenKind> = None;

                    let keys = [TokenKind::Define, TokenKind::Include];
                    for kind in keys.into_iter() {
                        let key = kind.to_string();
                        if key == word {
                            found_kind = Some(kind);
                            break;
                        }
                    }

                    if let Some(kind) = found_kind {
                        Ok(kind.into_token_with_span(self.source.relative_span_by_pos(start, end)))
                    } else if self.context == Context::Global && self.peek().unwrap() == '[' {
                        Ok(TokenKind::Pound.into_token_with_span(self.source.relative_span_by_pos(self.position, self.position)))
                    } else {
                        // Otherwise we don't support # prefixed indentifiers
                        tracing::error!(target: "lexer", "INVALID '#' CHARACTER USAGE");
                        return Err(LexicalError::new(
                            LexicalErrorKind::InvalidCharacter('#'),
                            self.source.relative_span_by_pos(self.position, self.position),
                        ));
                    }
                }
                // Alphabetical characters
                ch if ch.is_alphabetic() || ch.eq(&'_') => {
                    let (word, start, mut end) = self.eat_while(Some(ch), |c| c.is_alphanumeric() || c == '_');

                    let mut found_kind: Option<TokenKind> = None;
                    if self.context != Context::MacroBody {
                        if let Some(kind) = TOKEN.get(&word) {
                            found_kind = Some(kind.clone());
                        }
                    }

                    // Check to see if the found kind is, in fact, a keyword and not the name of
                    // a function. If it is, set `found_kind` to `None` so that it is set to a
                    // `TokenKind::Ident` in the following control flow.
                    if !self.check_keyword_rules(&found_kind) {
                        found_kind = None;
                    }

                    // Set the context based on the found token kind
                    if let Some(kind) = &found_kind {
                        match kind {
                            TokenKind::Macro | TokenKind::Fn | TokenKind::Test => self.context = Context::MacroDefinition,
                            TokenKind::Function | TokenKind::Event | TokenKind::Error => self.context = Context::Abi,
                            TokenKind::Constant => self.context = Context::Constant,
                            TokenKind::CodeTable => self.context = Context::CodeTableBody,
                            _ => (),
                        }
                    }

                    // Check for free storage pointer builtin
                    let fsp = "FREE_STORAGE_POINTER";
                    if fsp == word {
                        // Consume the parenthesis following the FREE_STORAGE_POINTER
                        // Note: This will consume `FREE_STORAGE_POINTER)` or
                        // `FREE_STORAGE_POINTER(` as well
                        if let Some('(') = self.peek() {
                            self.consume();
                        }
                        if let Some(')') = self.peek() {
                            self.consume();
                        }
                        end += 2;
                        found_kind = Some(TokenKind::FreeStoragePointer);
                    }

                    if let Some(':') = self.peek() {
                        found_kind = Some(TokenKind::Label(word.clone()));
                    }

                    // Syntax sugar: true evaluates to 0x01, false evaluates to 0x00
                    if matches!(word.as_str(), "true" | "false") {
                        found_kind = Some(TokenKind::Literal(str_to_bytes32(if word.as_str() == "true" { "1" } else { "0" })));
                        self.eat_while(None, |c| c.is_alphanumeric());
                    }

                    if !(self.context != Context::MacroBody || found_kind.is_some()) {
                        if let Some(o) = OPCODES_MAP.get(&word) {
                            found_kind = Some(TokenKind::Opcode(o.to_owned()));
                        }
                    }

                    if self.context == Context::AbiArgs {
                        let curr_char = self.peek().unwrap();
                        if !['(', ')'].contains(&curr_char) {
                            let (partial_raw_type, _, abi_args_end) =
                                self.eat_while(Some(ch), |c| c.is_alphanumeric() || c == '[' || c == ']');
                            let raw_type = word.clone() + &partial_raw_type[1..];

                            if raw_type == TokenKind::Calldata.to_string() {
                                found_kind = Some(TokenKind::Calldata);
                            } else if raw_type == TokenKind::Memory.to_string() {
                                found_kind = Some(TokenKind::Memory);
                            } else if raw_type == TokenKind::Storage.to_string() {
                                found_kind = Some(TokenKind::Storage);
                            } else if EVM_TYPE_ARRAY_REGEX.is_match(&raw_type) {
                                // split to get array size and type
                                // TODO: support multi-dimensional arrays
                                let words: Vec<String> = Regex::new(r"\[").unwrap().split(&raw_type).map(|x| x.replace(']', "")).collect();
                                let mut size_vec: Vec<usize> = Vec::new();
                                // go over all array sizes
                                let sizes = words.get(1..words.len()).unwrap();
                                for size in sizes.iter() {
                                    match size.is_empty() {
                                        true => size_vec.push(0),
                                        false => {
                                            let arr_size: usize = size
                                                .parse::<usize>()
                                                .map_err(|_| {
                                                    let err = LexicalError {
                                                        kind: LexicalErrorKind::InvalidArraySize(words[1].clone()),
                                                        span: self.source.relative_span_by_pos(start, end),
                                                    };
                                                    tracing::error!(target: "lexer", "{}", format!("{err:?}"));
                                                    err
                                                })
                                                .unwrap();
                                            size_vec.push(arr_size);
                                        }
                                    }
                                }
                                let primitive = PrimitiveEVMType::try_from(words[0].clone());
                                if let Ok(primitive) = primitive {
                                    found_kind = Some(TokenKind::ArrayType(primitive, size_vec));
                                } else {
                                    let err = LexicalError {
                                        kind: LexicalErrorKind::InvalidPrimitiveType(words[0].clone()),
                                        span: self.source.relative_span_by_pos(start, end),
                                    };
                                    tracing::error!(target: "lexer", "{}", format!("{err:?}"));
                                }
                            } else {
                                // We don't want to consider any argument names or the "indexed"
                                // keyword here.
                                let primitive = PrimitiveEVMType::try_from(word.clone());
                                if let Ok(primitive) = primitive {
                                    found_kind = Some(TokenKind::PrimitiveType(primitive));
                                }
                            }
                            end = abi_args_end;
                        } else {
                            // We don't want to consider any argument names or the "indexed"
                            // keyword here.
                            let primitive = PrimitiveEVMType::try_from(word.clone());
                            if let Ok(primitive) = primitive {
                                found_kind = Some(TokenKind::PrimitiveType(primitive));
                            }
                        }
                    }

                    let kind = if let Some(kind) = found_kind {
                        kind
                    } else if (self.context == Context::MacroBody
                        || self.context == Context::BuiltinFunction
                        || self.context == Context::CodeTableBody
                        || self.context == Context::Constant)
                        && BuiltinFunctionKind::try_from(&word).is_ok()
                    {
                        TokenKind::BuiltinFunction(word)
                    } else {
                        TokenKind::Ident(word)
                    };

                    Ok(kind.into_token_with_span(self.source.relative_span_by_pos(start, end)))
                }
                // If it's the start of a hex literal
                ch if ch == '0' && self.peek().unwrap() == 'x' => self.eat_hex_digit(ch),
                '=' => self.single_char_token(TokenKind::Assign),
                '(' => {
                    match self.context {
                        Context::Abi => self.context = Context::AbiArgs,
                        Context::MacroBody => match self.lookback.as_ref().unwrap().kind {
                            TokenKind::BuiltinFunction(_) => self.context = Context::BuiltinFunction,
                            _ => self.context = Context::MacroArgs,
                        },
                        _ => {}
                    }
                    self.single_char_token(TokenKind::OpenParen)
                }
                ')' => {
                    match self.context {
                        Context::AbiArgs => self.context = Context::Abi,
                        Context::MacroArgs => self.context = Context::MacroBody,
                        Context::BuiltinFunction => self.context = Context::MacroBody,
                        _ => {}
                    }
                    self.single_char_token(TokenKind::CloseParen)
                }
                '[' => self.single_char_token(TokenKind::OpenBracket),
                ']' => self.single_char_token(TokenKind::CloseBracket),
                '{' => {
                    if self.context == Context::MacroDefinition {
                        self.context = Context::MacroBody;
                    }
                    self.single_char_token(TokenKind::OpenBrace)
                }
                '}' => {
                    if matches!(self.context, Context::MacroBody | Context::CodeTableBody) {
                        self.context = Context::Global;
                    }
                    self.single_char_token(TokenKind::CloseBrace)
                }
                '+' => self.single_char_token(TokenKind::Add),
                '-' => self.single_char_token(TokenKind::Sub),
                '*' => self.single_char_token(TokenKind::Mul),
                '<' => self.single_char_token(TokenKind::LeftAngle),
                '>' => self.single_char_token(TokenKind::RightAngle),
                // NOTE: TokenKind::Div is lexed further up since it overlaps with comment
                ':' => self.single_char_token(TokenKind::Colon),
                // identifiers
                ',' => self.single_char_token(TokenKind::Comma),
                '0'..='9' => self.eat_digit(ch),
                // Lexes Spaces and Newlines as Whitespace
                ch if ch.is_ascii_whitespace() => {
                    let (_, start, end) = self.eat_whitespace();
                    Ok(TokenKind::Whitespace.into_token_with_span(self.source.relative_span_by_pos(start, end)))
                }
                // String literals. String literals can also be wrapped by single quotes
                '"' | '\'' => Ok(self.eat_string_literal()),
                ch => {
                    tracing::error!(target: "lexer", "UNSUPPORTED TOKEN '{}'", ch);
                    return Err(LexicalError::new(
                        LexicalErrorKind::InvalidCharacter(ch),
                        self.source.relative_span_by_pos(self.position, self.position),
                    ));
                }
            }?;

            if token.kind != TokenKind::Whitespace {
                self.lookback = Some(token.clone());
            }

            Ok(token)
        } else {
            self.eof = true;
            Ok(Token { kind: TokenKind::Eof, span: self.source.relative_span_by_pos(self.position, self.position) })
        }
    }

    fn single_char_token(&self, token_kind: TokenKind) -> TokenResult {
        Ok(token_kind.into_token_with_span(self.source.relative_span_by_pos(self.position, self.position)))
    }

    /// Keeps consuming tokens as long as the predicate is satisfied
    fn eat_while<F: Fn(char) -> bool>(&mut self, initial_char: Option<char>, predicate: F) -> (String, usize, usize) {
        let start = self.position;

        // This function is only called when we want to continue consuming a character of the same
        // type. For example, we see a digit, and we want to consume the whole integer.
        // Therefore, the current character which triggered this function will need to be appended.
        let mut word = String::new();
        if let Some(init_char) = initial_char {
            word.push(init_char)
        }

        // Keep checking that we are not at the EOF
        while let Some(peek_char) = self.peek() {
            // Then check for the predicate, if predicate matches append char and increment the
            // cursor If not, return word. The next character will be analyzed on the
            // next iteration of next_token, Which will increment the cursor
            if !predicate(peek_char) {
                return (word, start, self.position);
            }
            word.push(peek_char);

            // If we arrive at this point, then the char has been added to the word and we should
            // increment the cursor
            self.consume();
        }

        (word, start, self.position)
    }

    fn eat_digit(&mut self, initial_char: char) -> TokenResult {
        let (integer_str, start, end) = self.eat_while(Some(initial_char), |ch| ch.is_ascii_digit());

        let integer = integer_str.parse().unwrap();
        let integer_token = TokenKind::Num(integer);

        Ok(Token { kind: integer_token, span: self.source.relative_span_by_pos(start, end) })
    }

    fn eat_hex_digit(&mut self, initial_char: char) -> TokenResult {
        let (integer_str, mut start, end) = self.eat_while(Some(initial_char), |ch| ch.is_ascii_hexdigit() | (ch == 'x'));
        if integer_str.matches('x').count() != 1 {
            return Err(LexicalError::new(
                LexicalErrorKind::InvalidHexLiteral(integer_str.clone()),
                self.source.relative_span_by_pos(start, end),
            ));
        }

        let kind = if self.context == Context::CodeTableBody || self.context == Context::Constant {
            // In code tables, or constant values the bytecode provided is of arbitrary length. We pass
            // the code as an Ident, and parse it later.

            // For constants only max 32 Bytes is allowed for hex string 0x. 2 + 64 = 66 characters
            if self.context == Context::Constant && integer_str.len() > 66 {
                return Err(LexicalError::new(
                    LexicalErrorKind::HexLiteralTooLong(integer_str.clone()),
                    self.source.relative_span_by_pos(start, end),
                ));
            }
            let hex_string = format_even_bytes(integer_str[2..].to_lowercase());
            TokenKind::Bytes(hex_string)
        } else {
            // See above comment for the 66-character limit
            if integer_str.len() > 66 {
                return Err(LexicalError::new(
                    LexicalErrorKind::HexLiteralTooLong(integer_str.clone()),
                    self.source.relative_span_by_pos(start, end),
                ));
            }
            TokenKind::Literal(str_to_bytes32(integer_str[2..].as_ref()))
        };

        start += 2;

        Ok(Token { kind, span: self.source.relative_span_by_pos(start, end) })
    }

    /// Skips white space. They are not significant in the source language
    fn eat_whitespace(&mut self) -> (String, usize, usize) {
        self.eat_while(None, |ch| ch.is_whitespace())
    }

    fn eat_string_literal(&mut self) -> Token {
        let (str_literal, start_span, end_span) = self.eat_while(None, |ch| ch != '"' && ch != '\'');
        let str_literal_token = TokenKind::Str(str_literal);
        self.consume(); // Advance past the closing quote
        str_literal_token.into_token_with_span(self.source.relative_span_by_pos(start_span, end_span + 1))
    }

    /// Checks the previous token kind against the input.
    pub fn checked_lookback(&self, kind: TokenKind) -> bool {
        self.lookback.as_ref().and_then(|t| if t.kind == kind { Some(true) } else { None }).is_some()
    }

    /// Check if a given keyword follows the keyword rules in the `source`. If not, it is a
    /// `TokenKind::Ident`.
    ///
    /// Rules:
    /// - The `macro`, `fn`, `test`, `function`, `constant`, `event`, `jumptable`,
    ///   `jumptable__packed`, and `table` keywords must be preceded by a `#define` keyword.
    /// - The `takes` keyword must be preceded by an assignment operator: `=`.
    /// - The `nonpayable`, `payable`, `view`, and `pure` keywords must be preceeded by one of these
    ///   keywords or a close paren.
    /// - The `returns` keyword must be succeeded by an open parenthesis and must *not* be succeeded
    ///   by a colon or preceded by the keyword `function`
    pub fn check_keyword_rules(&mut self, found_kind: &Option<TokenKind>) -> bool {
        match found_kind {
            Some(TokenKind::Macro)
            | Some(TokenKind::Fn)
            | Some(TokenKind::Test)
            | Some(TokenKind::Function)
            | Some(TokenKind::Constant)
            | Some(TokenKind::Error)
            | Some(TokenKind::Event)
            | Some(TokenKind::JumpTable)
            | Some(TokenKind::JumpTablePacked)
            | Some(TokenKind::CodeTable) => self.checked_lookback(TokenKind::Define),
            Some(TokenKind::NonPayable) | Some(TokenKind::Payable) | Some(TokenKind::View) | Some(TokenKind::Pure) => {
                let keys = [TokenKind::NonPayable, TokenKind::Payable, TokenKind::View, TokenKind::Pure, TokenKind::CloseParen];
                for key in keys {
                    if self.checked_lookback(key) {
                        return true;
                    }
                }
                false
            }
            Some(TokenKind::Takes) => self.checked_lookback(TokenKind::Assign),
            Some(TokenKind::Returns) => {
                self.eat_whitespace();
                // Allow for loose and tight syntax (e.g. `returns   (0)`, `returns(0)`, ...)
                self.peek().unwrap_or(')') == '(' && !self.checked_lookback(TokenKind::Function)
            }
            _ => true,
        }
    }

    /// Lex all imports
    /// Example import: `// #include "./Utils.huff"`
    pub fn lex_imports(source: &str) -> Vec<String> {
        let mut imports = vec![];
        let mut peekable_source = source.chars().peekable();
        let mut include_chars_iterator = "#include".chars().peekable();
        while peekable_source.peek().is_some() {
            while let Some(nc) = peekable_source.next() {
                if nc.eq(&'/') {
                    if let Some(nnc) = peekable_source.peek() {
                        if nnc.eq(&'/') {
                            // Iterate until newline
                            while let Some(lc) = &peekable_source.next() {
                                if lc.eq(&'\n') {
                                    break;
                                }
                            }
                        } else if nnc.eq(&'*') {
                            // Iterate until '*/'
                            while let Some(lc) = peekable_source.next() {
                                if lc.eq(&'*') {
                                    if let Some(llc) = peekable_source.peek() {
                                        if *llc == '/' {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if include_chars_iterator.peek().is_none() {
                    // Reset the include chars iterator
                    include_chars_iterator = "#include".chars().peekable();

                    // Skip over whitespace
                    while peekable_source.peek().is_some() {
                        if !peekable_source.peek().unwrap().is_whitespace() {
                            break;
                        } else {
                            peekable_source.next();
                        }
                    }

                    // Then we should have an import path between quotes
                    #[allow(clippy::collapsible_match)]
                    if let Some(char) = peekable_source.peek() {
                        match char {
                            '"' | '\'' => {
                                peekable_source.next();
                                let mut import = String::new();
                                while peekable_source.peek().is_some() {
                                    if let Some(c) = peekable_source.next() {
                                        if matches!(c, '"' | '\'') {
                                            imports.push(import);
                                            break;
                                        } else {
                                            import.push(c);
                                        }
                                    }
                                }
                            }
                            _ => { /* Ignore non-include tokens */ }
                        }
                    }
                } else if nc.ne(&include_chars_iterator.next().unwrap()) {
                    include_chars_iterator = "#include".chars().peekable();
                    break;
                }
            }
        }
        imports
    }
}

impl Iterator for Lexer<'_> {
    type Item = TokenResult;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            None
        } else {
            Some(self.next_token())
        }
    }
}
