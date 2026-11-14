//! Rendering for the command palette: the card, the windowed rows, the
//! section headers and the footer.
//!
//! The engine stays in the parent module; this file is the part that turns its
//! state into elements.

use super::*;

// The product implements `EventEmitter` for its concrete palette at the old
// path, for its own selection and its own closed event (the host's
// [`PaletteHost::Closed`]): one generic impl over `T::Selection` would
// conflict with the closed event, the way two blanket impls of one event
// trait always do.

impl<T: PaletteEntry, H: PaletteHost> Render for CommandPalette<T, H>
where
    Self: EventEmitter<T::Selection> + EventEmitter<H::Closed> + EventEmitter<T::Removed>,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div().into_any_element();
        }

        let theme = cx.theme();

        // The list shows a window of VISIBLE_ITEMS rows; the thumb shows
        // where that window sits in the whole result.
        // ponytail: display only; drag it if wheel and arrow keys fall short.
        let total = self.filtered.len();
        let scrollbar_thumb = (total > VISIBLE_ITEMS).then(|| {
            div()
                .absolute()
                .right(PaletteMetrics::SCROLLBAR_INSET)
                .w(PaletteMetrics::SCROLLBAR_WIDTH)
                .top(relative(self.scroll_offset as f32 / total as f32))
                .h(relative(VISIBLE_ITEMS as f32 / total as f32))
                .rounded_full()
                .bg(theme.scrollbar_thumb)
        });

        // `filtered` is already in section order, so the window is too. The
        // first row of each section carries the section's name on its right,
        // and a section after the first starts under a separator line, as
        // VS Code labels its "recently opened" group.
        #[expect(
            clippy::indexing_slicing,
            reason = "every `FilteredItem` index comes from `.enumerate()` over \
                      `self.items`, so it stays below `self.items.len()`"
        )]
        let list_rows: Vec<AnyElement> = self
            .filtered
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(VISIBLE_ITEMS)
            .map(|(display_idx, filtered)| {
                let item = &self.items[filtered.index];
                let section = item.section();
                let starts_section = display_idx == 0
                    || self
                        .filtered
                        .get(display_idx - 1)
                        .map(|previous| self.items[previous.index].section())
                        != Some(section);
                let is_selected = display_idx == self.selected_index;

                let row = self
                    .render_palette_item(
                        display_idx,
                        item,
                        is_selected,
                        starts_section.then(|| section.label(cx)),
                        cx,
                    )
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.selected_index = display_idx;
                        this.execute_selected(OpenPlacement::ReuseTab, window, cx);
                    }));

                if starts_section && display_idx > 0 {
                    div()
                        .flex_shrink_0()
                        .border_t_1()
                        .border_color(theme.border)
                        .child(row)
                        .into_any_element()
                } else {
                    row.into_any_element()
                }
            })
            .collect();

        let surface = inspect_surface_role(SurfaceRole::Modal);
        let card_shape = Chamfer::new(ChamferCut::CARD)
            .fill(surface.fill.resolve(theme))
            .border(surface.border.resolve(theme));

        let max_height = window.viewport_size().height - PaletteMetrics::TOP_OFFSET * 2.0;

        let handle = cx.entity().downgrade();
        let overlay = div()
            .id("command-palette-overlay")
            .absolute()
            .inset_0()
            .flex()
            .justify_center()
            .items_start()
            .pt(PaletteMetrics::TOP_OFFSET)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.hide(cx);
                }),
            );

        // The product's keymap binds its CommandPalette layer to its
        // run-command action; the host registers the listener, and anything
        // the palette does not answer goes on to the workspace.
        let overlay = H::attach_run_command(
            overlay,
            Arc::new(move |command_id, window, cx| {
                let Some(entity) = handle.upgrade() else {
                    return false;
                };

                entity.update(cx, |this, cx| {
                    this.handle_run_command(command_id, window, cx)
                })
            }),
        );

        overlay
            .key_context(H::key_context())
            .on_modifiers_changed(
                cx.listener(|this, event: &ModifiersChangedEvent, window, cx| {
                    this.modifiers_changed(&event.modifiers, window, cx);
                }),
            )
            .child(
                div()
                    .id("command-palette-container")
                    .relative()
                    .w(PaletteMetrics::WIDTH)
                    .max_w_full()
                    .max_h(max_height)
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .p(PaletteMetrics::CARD_PADDING)
                    .gap(PaletteMetrics::LIST_GAP)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(card_shape)
                    .child(self.render_search_row(cx))
                    .child(
                        div()
                            .id("command-palette-list")
                            .relative()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_h_0()
                            .overflow_y_hidden()
                            // Room for the thumb, so it never covers a label.
                            .when(scrollbar_thumb.is_some(), |list| {
                                list.pr(PaletteMetrics::SCROLLBAR_GUTTER)
                            })
                            .on_scroll_wheel(cx.listener(
                                |this, event: &ScrollWheelEvent, _window, cx| {
                                    let delta = event.delta.pixel_delta(px(1.0));
                                    if delta.y < px(0.0) {
                                        this.scroll_down(cx);
                                    } else if delta.y > px(0.0) {
                                        this.scroll_up(cx);
                                    }
                                },
                            ))
                            .children(list_rows)
                            .children(scrollbar_thumb)
                            .when(self.filtered.is_empty(), |list| {
                                list.child(
                                    div()
                                        .flex()
                                        .justify_center()
                                        .h(PaletteMetrics::ROW_HEIGHT)
                                        .items_center()
                                        .child(
                                            Text::body(
                                                strings::text(cx, "palette.empty").to_string(),
                                            )
                                            .muted_foreground(),
                                        ),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }
}
