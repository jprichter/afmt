use crate::{
    data_model::DocBuild,
    doc::{BraceStyle, Doc, DocRef, PrettyConfig},
    enum_def::BodyMember,
};
use typed_arena::Arena;

pub struct DocBuilder<'a> {
    arena: Arena<Doc<'a>>,
    pub(crate) config: PrettyConfig,
}

impl<'a> DocBuilder<'a> {
    pub fn new(config: PrettyConfig) -> Self {
        Self {
            arena: Arena::new(),
            config,
        }
    }

    // NOTE: group does NOT work with b.nl() so don't wrap b.nl() in any inputs
    pub fn group_surround(
        &'a self,
        elems: &[DocRef<'a>],
        sep: Insertable<'a>,
        open: Insertable<'a>,
        close: Insertable<'a>,
    ) -> DocRef<'a> {
        self.group(self.surround(elems, sep, open, close))
    }

    pub fn surround(
        &'a self,
        elems: &[DocRef<'a>],
        sep: Insertable<'a>,
        open: Insertable<'a>,
        close: Insertable<'a>,
    ) -> DocRef<'a> {
        if elems.is_empty() {
            return self.concat(vec![
                self.txt(open.str.unwrap()),
                self.txt(close.str.unwrap()),
            ]);
        }

        let mut docs = Vec::new();

        if let Some(n) = open.pre {
            docs.push(n);
        }
        if let Some(n) = open.str {
            docs.push(self.txt(n));
        }
        if let Some(n) = open.suf {
            docs.push(self.indent(n));
        }

        docs.push(self.indent(self.intersperse(elems, sep)));

        if let Some(n) = close.pre {
            docs.push(n);
        }
        if let Some(n) = close.str {
            docs.push(self.txt(n));
        }
        if let Some(n) = close.suf {
            docs.push(n);
        }

        self.concat(docs)
    }

    pub fn intersperse(&'a self, elems: &[DocRef<'a>], sep: Insertable<'a>) -> DocRef<'a> {
        if elems.is_empty() {
            return self.nil();
        }

        let mut parts = Vec::with_capacity(elems.len() * 2 - 1);
        for (i, &elem) in elems.iter().enumerate() {
            if i > 0 {
                if let Some(n) = sep.pre {
                    parts.push(n);
                }
                if let Some(ref n) = sep.str {
                    parts.push(self.txt(n));
                }
                if let Some(n) = sep.suf {
                    parts.push(n);
                }
            }
            parts.push(elem);
        }
        self.concat(parts)
    }

    pub fn surround_body_members<M>(
        &'a self,
        elems: &[BodyMember<M>],
        open: &str,
        close: &str,
    ) -> DocRef<'a>
    where
        M: DocBuild<'a>,
    {
        if elems.is_empty() {
            return self.concat(vec![self.txt(open), self.nl(), self.txt(close)]);
        }

        let multi_line = self.concat(vec![
            self.txt(open),
            self.indent(self.nl()),
            self.indent(self.intersperse_body_members(elems)),
            self.nl(),
            self.txt(close),
        ]);
        multi_line
    }

    pub fn intersperse_body_members<'b, M>(&'a self, members: &[BodyMember<M>]) -> DocRef<'a>
    where
        M: DocBuild<'a>,
    {
        if members.is_empty() {
            return self.nil();
        }

        let mut member_docs = Vec::new();
        for (i, m) in members.iter().enumerate() {
            member_docs.push(m.member.build(self));

            if i < members.len() - 1 {
                if m.has_trailing_newline {
                    member_docs.push(self.empty_new_line());
                } else {
                    member_docs.push(self.nl());
                }
            }
        }
        self.concat(member_docs)
    }

    pub fn to_docs<'b, T>(&'a self, items: impl IntoIterator<Item = &'b T>) -> Vec<DocRef<'a>>
    where
        T: DocBuild<'a> + 'b,
    {
        items.into_iter().map(|item| item.build(self)).collect()
    }

    pub fn group_indent_concat(
        &'a self,
        doc_refs: impl IntoIterator<Item = DocRef<'a>>,
    ) -> DocRef<'a> {
        self.group_indent(self.concat(doc_refs))
    }

    pub fn group_indent(&'a self, doc_ref: DocRef<'a>) -> DocRef<'a> {
        self.group(self.indent(doc_ref))
    }

    pub fn group_concat(&'a self, doc_refs: impl IntoIterator<Item = DocRef<'a>>) -> DocRef<'a> {
        self.group(self.concat(doc_refs))
    }

    pub fn nil(&'a self) -> DocRef<'a> {
        self.txt("")
    }

    pub fn nl(&'a self) -> DocRef<'a> {
        self.arena.alloc(Doc::Newline)
    }

    pub fn force_break(&'a self) -> DocRef<'a> {
        self.arena.alloc(Doc::ForceBreak)
    }

    pub fn softline(&'a self) -> DocRef<'a> {
        self.arena.alloc(Doc::Softline)
    }

    pub fn maybeline(&'a self) -> DocRef<'a> {
        self.arena.alloc(Doc::Maybeline)
    }

    pub fn empty_new_line(&'a self) -> DocRef<'a> {
        self.concat(vec![self.nl_with_no_indent(), self.nl()])
    }

    fn nl_with_no_indent(&'a self) -> DocRef<'a> {
        self.arena.alloc(Doc::NewlineWithNoIndent)
    }

    pub fn txt(&'a self, text: impl ToString) -> DocRef<'a> {
        let s = text.to_string();
        let width = s.len() as u32;
        self.arena.alloc(Doc::Text(s, width))
    }

    pub fn verbatim(&'a self, text: impl ToString) -> DocRef<'a> {
        self.arena.alloc(Doc::Verbatim(text.to_string()))
    }

    pub fn _txt(&'a self, text: impl ToString) -> DocRef<'a> {
        let s = text.to_string();
        let space_s = format!(" {}", s);
        self.txt(space_s)
    }

    pub fn txt_(&'a self, text: impl ToString) -> DocRef<'a> {
        let s = text.to_string();
        let s_space = format!("{} ", s);
        self.txt(s_space)
    }

    pub fn _txt_(&'a self, text: impl ToString) -> DocRef<'a> {
        let s = text.to_string();
        let space_s_space = format!(" {} ", s);
        self.txt(space_s_space)
    }

    pub fn flat(&'a self, doc_ref: DocRef<'a>) -> DocRef<'a> {
        self.arena.alloc(Doc::Flat(doc_ref))
    }

    pub fn indent(&'a self, doc_ref: DocRef<'a>) -> DocRef<'a> {
        let relative_indent = self.config.indent_size;
        self.arena.alloc(Doc::Indent(relative_indent, doc_ref))
    }

    /// Separator emitted between a construct's header and its opening brace.
    ///
    /// This is the sole difference between K&R and Allman: a space keeps the
    /// brace on the header line; a newline drops it to its own line at the
    /// header's indent. Callers that currently push `b.txt(" ")` before a body
    /// block should push this instead. Must not be placed inside a `group(...)`
    /// (newlines don't compose with groups).
    pub fn body_open_sep(&'a self) -> DocRef<'a> {
        match self.config.brace_style {
            BraceStyle::KAndR => self.txt(" "),
            BraceStyle::Allman => self.nl(),
        }
    }

    /// Separator between adjacent clauses such as `try`/`catch`/`finally`.
    pub fn clause_sep(&'a self) -> DocRef<'a> {
        self.body_open_sep()
    }

    /// Whether single-statement clause bodies are wrapped in synthesized braces.
    pub fn wraps_single(&self) -> bool {
        self.config.wrap_single_statements
    }

    /// Whether known Apex annotation names should use canonical casing.
    pub fn normalizes_annotation_casing(&self) -> bool {
        self.config.normalize_annotation_casing
    }

    /// Brace-wrap an already-built single (non-block) statement: separator +
    /// `{` + indented statement + `}` on its own line. Honors `brace_style` via
    /// `body_open_sep`.
    fn wrap_single(&'a self, body: DocRef<'a>) -> DocRef<'a> {
        self.concat(vec![
            self.body_open_sep(),
            self.txt("{"),
            self.indent(self.nl()),
            self.indent(body),
            self.nl(),
            self.txt("}"),
        ])
    }

    /// Emit an empty brace block when single-statement bodies are enabled.
    pub fn empty_clause_body(&'a self) -> DocRef<'a> {
        if self.config.wrap_single_statements {
            self.concat(vec![
                self.body_open_sep(),
                self.txt("{"),
                self.nl(),
                self.txt("}"),
            ])
        } else {
            self.nil()
        }
    }

    /// Emit a control-flow clause body (an `if`/`else`/loop body) with brace
    /// placement and single-statement wrapping driven by config.
    ///
    /// - Block body: `body_open_sep` + the block (already brace-wrapped).
    /// - Single statement + `wrap_single_statements`: synthesize a brace block.
    /// - Single statement otherwise: the legacy brace-less form. `break_single`
    ///   selects the fallback layout that must be preserved per construct:
    ///   `true` puts the statement on its own indented line (`if` style),
    ///   `false` keeps it inline after a space (loop style).
    pub fn clause_body(
        &'a self,
        body: DocRef<'a>,
        is_block: bool,
        break_single: bool,
    ) -> DocRef<'a> {
        if is_block {
            self.concat(vec![self.body_open_sep(), body])
        } else if self.config.wrap_single_statements {
            self.wrap_single(body)
        } else if break_single {
            self.concat(vec![self.indent(self.nl()), self.indent(body)])
        } else {
            self.concat(vec![self.txt(" "), body])
        }
    }

    pub fn dedent(&'a self, doc_ref: DocRef<'a>) -> DocRef<'a> {
        let relative_indent = self.config.indent_size;
        self.arena.alloc(Doc::Dedent(relative_indent, doc_ref))
    }

    //fn align(&'a self, relative_col_offset: u32, doc_ref: DocRef<'a>) -> DocRef<'a> {
    //    self.arena.alloc(Doc::Align(relative_col_offset, doc_ref))
    //}

    pub fn concat(&'a self, doc_refs: impl IntoIterator<Item = DocRef<'a>>) -> DocRef<'a> {
        let n_vec = doc_refs.into_iter().collect::<Vec<_>>();
        self.arena.alloc(Doc::Concat(n_vec))
    }

    pub fn choice(&'a self, first: DocRef<'a>, second: DocRef<'a>) -> DocRef<'a> {
        self.arena.alloc(Doc::Choice(first, second))
    }

    pub fn group(&'a self, doc_ref: DocRef<'a>) -> DocRef<'a> {
        self.choice(self.flat(doc_ref), doc_ref)
    }
}

pub struct Insertable<'a> {
    pub pre: Option<DocRef<'a>>,
    pub str: Option<String>,
    pub suf: Option<DocRef<'a>>,
}

impl<'a> Insertable<'a> {
    pub fn new<S: ToString>(
        pre: Option<DocRef<'a>>,
        s: Option<S>,
        suf: Option<DocRef<'a>>,
    ) -> Self {
        Self {
            pre,
            str: s.map(|s_value| s_value.to_string()),
            suf,
        }
    }
}

impl<'a> DocBuild<'a> for Insertable<'a> {
    fn build_inner(&self, b: &'a DocBuilder<'a>, result: &mut Vec<DocRef<'a>>) {
        if let Some(n) = self.pre {
            result.push(n);
        }
        if let Some(ref n) = self.str {
            result.push(b.txt(n));
        }
        if let Some(n) = self.suf {
            result.push(n);
        }
    }
}
