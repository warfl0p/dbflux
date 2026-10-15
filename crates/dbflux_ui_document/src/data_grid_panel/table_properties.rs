//! The Properties view of a table or view, next to its Data grid: the
//! structure of its columns, their statistics when the connection profiles
//! columns (the Columns view of `table_views`), and the source of a view.
//!
//! Every table source gets it. The structure comes from `table_details` and
//! the source from `view_definition`, both read when the view is first shown;
//! a driver that reads no view definitions simply shows no Source section.

use std::sync::Arc;

use dbflux_components::components::data_table::model::{
    CellValue, ColumnKind, ColumnSpec, RowData,
};
use dbflux_components::components::data_table::{DataTable, DataTableState, TableModel};
use dbflux_components::controls::ReadOnlyEditor;
use dbflux_components::icons::AppIcon;
use dbflux_components::primitives::{Icon, Text};
use dbflux_components::tokens::{FontSizes, Spacing, ui};
use dbflux_core::ColumnInfo;
use gpui::prelude::*;
use gpui::*;
use gpui_component::ActiveTheme;
use gpui_component::input::EditorState;

use super::table_views::TableTab;
use super::{DataGridPanel, DataSource};

/// Columns, Statistics or Source, the sections of the Properties view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PropertiesSection {
    #[default]
    Columns,
    Statistics,
    Source,
}

impl PropertiesSection {
    pub(crate) fn id(self) -> &'static str {
        match self {
            PropertiesSection::Columns => "properties-section-columns",
            PropertiesSection::Statistics => "properties-section-statistics",
            PropertiesSection::Source => "properties-section-source",
        }
    }

    fn label(self) -> String {
        match self {
            PropertiesSection::Columns => dbflux_i18n::t!("document.data.table_views.tab.columns"),
            PropertiesSection::Statistics => {
                dbflux_i18n::t!("document.data.table_views.section.statistics")
            }
            PropertiesSection::Source => {
                dbflux_i18n::t!("document.data.table_views.section.source")
            }
        }
    }

    pub(crate) fn icon(self) -> AppIcon {
        match self {
            PropertiesSection::Columns => AppIcon::Columns,
            PropertiesSection::Statistics => AppIcon::ChartColumnBig,
            PropertiesSection::Source => AppIcon::FileCode,
        }
    }
}

/// The Properties view of a table source. Nothing is read until it is shown.
#[derive(Default)]
pub(super) struct TablePropertiesState {
    pub(super) section: PropertiesSection,
    requested: bool,
    loading: bool,
    /// Bumped on every read, so a late answer to an older read is dropped.
    generation: u64,
    columns: Option<Result<ColumnsTable, SharedString>>,
    /// `None` while unread and for anything that is not a view.
    source: Option<Result<Entity<EditorState>, SharedString>>,
}

struct ColumnsTable {
    state: Entity<DataTableState>,
    table: Entity<DataTable>,
    empty: bool,
}

impl DataGridPanel {
    /// The sections the Properties view offers: Columns always, Statistics
    /// with a column profile, Source for a view whose driver reads it.
    pub(super) fn properties_sections(&self) -> Vec<PropertiesSection> {
        let mut sections = vec![PropertiesSection::Columns];

        if self.table_views.profile_view.is_some() {
            sections.push(PropertiesSection::Statistics);
        }

        if self.table_views.properties.source.is_some() {
            sections.push(PropertiesSection::Source);
        }

        sections
    }

    /// The section on screen: the chosen one, or Columns when the chosen
    /// one is not offered (a refresh found the view gone).
    pub(super) fn properties_section(&self) -> PropertiesSection {
        let section = self.table_views.properties.section;

        if self.properties_sections().contains(&section) {
            section
        } else {
            PropertiesSection::Columns
        }
    }

    pub(super) fn set_properties_section(
        &mut self,
        section: PropertiesSection,
        cx: &mut Context<Self>,
    ) {
        self.table_views.properties.section = section;
        cx.notify();
    }

    /// Reads the properties the first time the view is shown.
    pub(super) fn ensure_table_properties(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.table_views.properties.requested {
            self.load_table_properties(window, cx);
        }
    }

    /// Reads the column structure and, for a view, its source.
    pub(super) fn load_table_properties(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let DataSource::Table {
            profile_id,
            database,
            table,
            ..
        } = &self.source
        else {
            return;
        };

        let Some(connection) = self.table_view_connection(cx) else {
            return;
        };

        let Some(database) = self
            .app_state
            .read(cx)
            .connections()
            .get(profile_id)
            .map(|connected| Self::table_details_database(connected, database.as_deref()))
        else {
            return;
        };

        let table = table.clone();
        let properties = &mut self.table_views.properties;
        properties.requested = true;
        properties.loading = true;
        properties.generation += 1;
        let generation = properties.generation;
        cx.notify();

        let task = cx.background_executor().spawn(async move {
            let schema = table.schema.as_deref();
            let details = connection.table_details(&database, schema, &table.name);
            let source = connection.view_definition(&database, schema, &table.name);
            (details, source)
        });

        cx.spawn_in(window, async move |this, cx| {
            let (details, source) = task.await;

            let update = this.update_in(cx, |panel, window, cx| {
                if panel.table_views.properties.generation != generation {
                    return;
                }

                let columns = details
                    .map(|details| {
                        panel.build_columns_table(details.columns.as_deref().unwrap_or(&[]), cx)
                    })
                    .map_err(|error| {
                        SharedString::from(dbflux_i18n::t!(
                            "document.data.table_views.properties.columns_failed",
                            error = error.to_string()
                        ))
                    });

                let source = match source {
                    Ok(Some(definition)) => Some(Ok(cx.new(|cx| {
                        let mut state = EditorState::new(window, cx)
                            .language("sql")
                            .line_number(true);
                        state.set_value(&definition, window, cx);
                        state
                    }))),
                    Ok(None) => None,
                    Err(error) => Some(Err(SharedString::from(dbflux_i18n::t!(
                        "document.data.table_views.properties.source_failed",
                        error = error.to_string()
                    )))),
                };

                let properties = &mut panel.table_views.properties;
                properties.loading = false;
                properties.columns = Some(columns);
                properties.source = source;
                cx.notify();
            });

            if let Err(error) = update {
                log::debug!("data grid released before its properties arrived: {error}");
            }
        })
        .detach();
    }

    fn build_columns_table(&self, columns: &[ColumnInfo], cx: &mut Context<Self>) -> ColumnsTable {
        let model = Arc::new(columns_table_model(columns));
        let state = cx.new(|cx| DataTableState::new(model, cx));
        let table = cx.new(|cx| DataTable::new("properties-columns-table", state.clone(), cx));

        ColumnsTable {
            state,
            table,
            empty: columns.is_empty(),
        }
    }

    /// Puts the keyboard in the section on screen.
    pub(super) fn focus_table_properties(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let properties = &self.table_views.properties;

        match self.properties_section() {
            PropertiesSection::Columns => {
                if let Some(Ok(columns)) = &properties.columns {
                    let state = columns.state.clone();
                    state.update(cx, |state, cx| state.focus(window, cx));
                }
            }
            PropertiesSection::Statistics => {
                if let Some(view) = self.table_views.profile_view.clone() {
                    view.update(cx, |view, cx| view.focus(window, cx));
                }
            }
            PropertiesSection::Source => {
                if let Some(Ok(editor)) = &properties.source {
                    let editor = editor.clone();
                    editor.update(cx, |editor, cx| editor.focus(window, cx));
                }
            }
        }
    }

    /// The Properties view: the section list on the left, the section on
    /// the right.
    pub(super) fn render_table_properties(&self, cx: &mut Context<Self>) -> AnyElement {
        debug_assert_eq!(self.table_tab(), TableTab::Properties);

        let theme = cx.theme().clone();
        let current = self.properties_section();

        let section_list = div()
            .id("properties-sections")
            .flex()
            .flex_col()
            .flex_shrink_0()
            .w(ui(140.0))
            .py(Spacing::XS)
            .border_r_1()
            .border_color(theme.border)
            .children(self.properties_sections().into_iter().map(|section| {
                let selected = section == current;

                div()
                    .id(section.id())
                    .role(Role::Tab)
                    .aria_selected(selected)
                    .flex()
                    .items_center()
                    .gap(Spacing::SM)
                    .px(Spacing::MD)
                    .py(Spacing::XS)
                    .text_size(FontSizes::SM)
                    .cursor_pointer()
                    .when(selected, |item| {
                        item.bg(theme.secondary).text_color(theme.foreground)
                    })
                    .when(!selected, |item| {
                        item.text_color(theme.muted_foreground)
                            .hover(|item| item.bg(theme.secondary))
                    })
                    .child(Icon::new(section.icon()).small())
                    .child(section.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_properties_section(section, cx);
                    }))
            }));

        let body = match current {
            PropertiesSection::Columns => self.render_properties_columns(),
            PropertiesSection::Statistics => self
                .table_views
                .profile_view
                .clone()
                .map(IntoElement::into_any_element)
                .unwrap_or_else(|| div().into_any_element()),
            PropertiesSection::Source => self.render_properties_source(cx),
        };

        div()
            .id("table-properties")
            .flex()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(section_list)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .overflow_hidden()
                    .child(body),
            )
            .into_any_element()
    }

    fn render_properties_columns(&self) -> AnyElement {
        let properties = &self.table_views.properties;

        match &properties.columns {
            Some(Ok(columns)) if !columns.empty => columns.table.clone().into_any_element(),
            Some(Ok(_)) => properties_notice(dbflux_i18n::t!(
                "document.data.table_views.properties.no_columns"
            )),
            Some(Err(message)) => properties_notice(message.to_string()),
            None => properties_notice(dbflux_i18n::t!(
                "document.data.table_views.properties.loading"
            )),
        }
    }

    fn render_properties_source(&self, cx: &mut Context<Self>) -> AnyElement {
        match &self.table_views.properties.source {
            Some(Ok(editor)) => div()
                .flex_1()
                .min_h_0()
                .p(Spacing::SM)
                .child(
                    ReadOnlyEditor::new(editor)
                        .appearance(false)
                        .size_full()
                        .font_family(dbflux_components::fonts::editor_family(cx)),
                )
                .into_any_element(),
            Some(Err(message)) => properties_notice(message.to_string()),
            None => properties_notice(dbflux_i18n::t!(
                "document.data.table_views.properties.loading"
            )),
        }
    }
}

fn properties_notice(text: String) -> AnyElement {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .child(Text::caption(text))
        .into_any_element()
}

/// One row per column: name, position, type, not null, default, key.
pub(super) fn columns_table_model(columns: &[ColumnInfo]) -> TableModel {
    let spec = |id: &str, title: String, kind: ColumnKind, align: TextAlign| ColumnSpec {
        id: Arc::from(id),
        title: Arc::from(title),
        kind,
        align,
        type_name: Arc::from(""),
    };

    let specs = vec![
        spec(
            "name",
            dbflux_i18n::t!("document.data.table_views.properties.column.name"),
            ColumnKind::Text,
            TextAlign::Left,
        ),
        spec(
            "position",
            dbflux_i18n::t!("document.data.table_views.properties.column.position"),
            ColumnKind::Integer,
            TextAlign::Right,
        ),
        spec(
            "type",
            dbflux_i18n::t!("document.data.table_views.properties.column.type"),
            ColumnKind::Text,
            TextAlign::Left,
        ),
        spec(
            "not_null",
            dbflux_i18n::t!("document.data.table_views.properties.column.not_null"),
            ColumnKind::Text,
            TextAlign::Left,
        ),
        spec(
            "default",
            dbflux_i18n::t!("document.data.table_views.properties.column.default"),
            ColumnKind::Text,
            TextAlign::Left,
        ),
        spec(
            "primary_key",
            dbflux_i18n::t!("document.data.table_views.properties.column.primary_key"),
            ColumnKind::Text,
            TextAlign::Left,
        ),
    ];

    let check = |on: bool| CellValue::text(if on { "✓" } else { "" });

    let rows = columns
        .iter()
        .enumerate()
        .map(|(index, column)| RowData {
            cells: vec![
                CellValue::text(&column.name),
                CellValue::int(index as i64 + 1),
                CellValue::text(&column.type_name),
                check(!column.nullable),
                CellValue::text(column.default_value.as_deref().unwrap_or("")),
                check(column.is_primary_key),
            ],
        })
        .collect();

    TableModel::new(specs, rows)
}
