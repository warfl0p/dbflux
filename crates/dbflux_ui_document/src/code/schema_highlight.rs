//! Colors a bare schema name as a schema.
//!
//! The SQL grammar reads `FROM layer` as a table reference, so a schema name
//! only gets the schema color once a table follows it (`layer.t`). The
//! connection's catalog knows better: a table-colored identifier that names a
//! schema, and no table or view, is recolored with the schema role.

use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::Range;
use std::rc::Rc;

use gpui::{Context, HighlightStyle, SharedString, Window};
use gpui_component::input::{
    FoldRange, HighlightStyleResolver, InputEdit, InputHighlighter, InputHighlighterFactory, Rope,
};

use crate::completion_support::normalize_identifier;

/// Schema and relation names of the document's connection, normalized with
/// `normalize_identifier`. Refreshed whenever the editor syncs with its
/// connection.
#[derive(Default)]
pub(crate) struct CatalogNames {
    pub(crate) schemas: HashSet<String>,
    pub(crate) relations: HashSet<String>,
}

pub(crate) type SharedCatalogNames = Rc<RefCell<CatalogNames>>;

/// gpui-component's tree-sitter factory, with SQL highlighters wrapped so a
/// bare schema name gets the schema color.
pub(crate) fn schema_aware_highlighter_factory(
    names: SharedCatalogNames,
) -> InputHighlighterFactory {
    let tree_sitter = gpui_component::highlighter::input_highlighter_factory();
    Rc::new(move |language: &str| {
        let inner = tree_sitter(language)?;
        if language != "sql" {
            return Some(inner);
        }
        Some(Box::new(SchemaAwareHighlighter {
            inner,
            text: Rope::new(),
            names: names.clone(),
        }))
    })
}

struct SchemaAwareHighlighter {
    inner: Box<dyn InputHighlighter>,
    text: Rope,
    names: SharedCatalogNames,
}

impl InputHighlighter for SchemaAwareHighlighter {
    fn language(&self) -> SharedString {
        self.inner.language()
    }

    fn update(
        &mut self,
        edit: Option<InputEdit>,
        text: &Rope,
        folding: bool,
        window: &mut Window,
        cx: &mut Context<super::GpuiEditorState>,
    ) {
        self.text = text.clone();
        self.inner.update(edit, text, folding, window, cx);
    }

    fn styles(
        &self,
        range: &Range<usize>,
        resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let mut styles = self.inner.styles(range, resolver);
        if let (Some(table), Some(schema)) = (resolver.style("type"), resolver.style("namespace")) {
            recolor_schema_names(&mut styles, &self.text, &self.names.borrow(), table, schema);
        }
        styles
    }

    fn fold_ranges(&self, text: &Rope) -> Vec<FoldRange> {
        self.inner.fold_ranges(text)
    }

    fn fold_ranges_for_edit(&self, range: Range<usize>, text: &Rope) -> Vec<FoldRange> {
        self.inner.fold_ranges_for_edit(range, text)
    }
}

/// Gives `schema` to every `table`-styled span whose text names a schema and
/// no relation.
fn recolor_schema_names(
    styles: &mut [(Range<usize>, HighlightStyle)],
    text: &Rope,
    names: &CatalogNames,
    table: HighlightStyle,
    schema: HighlightStyle,
) {
    if names.schemas.is_empty() {
        return;
    }
    for (span, style) in styles {
        if *style != table || span.end > text.len() {
            continue;
        }
        let name = normalize_identifier(&text.slice(span.clone()).to_string());
        if names.schemas.contains(&name) && !names.relations.contains(&name) {
            *style = schema;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{FontStyle, FontWeight};

    #[test]
    fn a_table_colored_schema_name_takes_the_schema_color() {
        let table = HighlightStyle {
            font_weight: Some(FontWeight::BOLD),
            ..Default::default()
        };
        let schema = HighlightStyle {
            font_style: Some(FontStyle::Italic),
            ..Default::default()
        };
        let text = Rope::from("SELECT * FROM Layer JOIN users JOIN shared");
        let names = CatalogNames {
            schemas: ["layer", "shared"].map(String::from).into(),
            relations: ["users", "shared"].map(String::from).into(),
        };
        let mut styles = vec![
            (0..6, HighlightStyle::default()),
            (14..19, table),
            (25..30, table),
            (36..42, table),
        ];

        recolor_schema_names(&mut styles, &text, &names, table, schema);

        assert_eq!(styles[1].1, schema, "a schema name gets the schema color");
        assert_eq!(styles[2].1, table, "a table keeps the table color");
        assert_eq!(
            styles[3].1, table,
            "a name that is both a schema and a table stays a table"
        );
    }
}
