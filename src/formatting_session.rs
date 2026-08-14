use crate::context::CommentMap;
use crate::utility::{
    clear_thread_comment_map, clear_thread_source_code, collect_comments, set_thread_comment_map,
    set_thread_source_code,
};
use tree_sitter::Tree;

pub(crate) struct FormattingSession;

impl FormattingSession {
    pub(crate) fn new(source_code: &str, ast_tree: &Tree) -> Self {
        clear_thread_source_code();
        clear_thread_comment_map();
        let session = Self;

        set_thread_source_code(source_code.to_string());

        let mut cursor = ast_tree.walk();
        let mut comment_map = CommentMap::new();
        collect_comments(&mut cursor, &mut comment_map);

        set_thread_comment_map(comment_map);

        session
    }
}

impl Drop for FormattingSession {
    fn drop(&mut self) {
        clear_thread_source_code();
        clear_thread_comment_map();
    }
}
