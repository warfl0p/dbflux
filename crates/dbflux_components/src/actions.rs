use gpui::{Action, App, Global, Keystroke, SharedString, actions};

actions!(
    dbflux,
    [
        Cancel,
        // List navigation
        SelectNext,
        SelectPrev,
        Execute,
        // CRUD / item actions
        Delete,
        ToggleFavorite,
        Rename,
        // Search / saved queries
        FocusSearch,
        SaveQuery,
        // Save the value of a modal editor (cell editor, document preview)
        SaveEdit,
        // Scroll a modal body from the keyboard
        ScrollUp,
        ScrollDown,
        ScrollPageUp,
        ScrollPageDown,
        ScrollToTop,
        ScrollToBottom,
    ]
);

/// Runs a keymap command.
///
/// Every keybinding generated from the keymap for a context owned by a
/// window root dispatches this action, carrying the command's action id
/// (`Command::action_id`). The window root handles it; an element that owns
/// some commands handles it first and propagates the ones it does not own.
#[derive(Clone, Debug, PartialEq, Eq, Action)]
#[action(namespace = dbflux, no_json)]
pub struct RunCommand {
    pub command: SharedString,
    /// The binding is one the user changed. An element that takes keys of
    /// its own ahead of the default bindings (the code editor with Vim
    /// editing) leaves the key to a user binding.
    pub from_user_binding: bool,
}

impl RunCommand {
    pub fn new(command: impl Into<SharedString>) -> Self {
        Self {
            command: command.into(),
            from_user_binding: false,
        }
    }

    /// The action of a binding the user changed.
    pub fn from_user_binding(command: impl Into<SharedString>) -> Self {
        Self {
            command: command.into(),
            from_user_binding: true,
        }
    }
}

/// Answers "which keys run this command here?" for keycaps drawn by
/// components, which cannot read the app keymap themselves. The keymap
/// installs it at startup (see `dbflux_ui_base::keymap::init_keymap`).
///
/// The arguments are a context id and a command id
/// (`ContextId::id`, `Command::id`); the answer is the label of the keys, or
/// `None` when the command has no shortcut there.
pub struct ShortcutLabels(pub Box<dyn Fn(&str, &str) -> Option<SharedString>>);

impl gpui::Global for ShortcutLabels {}

/// The label of the keys that run `command` in `context`.
///
/// Without an installed [`ShortcutLabels`] (component tests) the default
/// label `fallback` is returned; with one, `None` means the user removed the
/// shortcut, so the keycap is left out.
pub fn shortcut_label(
    cx: &gpui::App,
    context: &str,
    command: &str,
    fallback: &str,
) -> Option<SharedString> {
    match cx.try_global::<ShortcutLabels>() {
        Some(labels) => (labels.0)(context, command),
        None => Some(SharedString::from(fallback.to_string())),
    }
}

/// Answers "which keys run the command of the element with this id?" for
/// tooltips drawn by components. Installed at startup from the keyboard
/// coverage registries (`dbflux_ui_base::keyboard_coverage::install_element_shortcuts`).
pub struct ElementShortcuts(pub Box<dyn Fn(&str) -> Option<SharedString>>);

impl gpui::Global for ElementShortcuts {}

/// The keys that run the command of the element `id`, `None` when it has no
/// registered command or the command has no shortcut.
pub fn element_shortcut(cx: &gpui::App, id: &gpui::ElementId) -> Option<SharedString> {
    let gpui::ElementId::Name(name) = id else {
        return None;
    };

    (cx.try_global::<ElementShortcuts>()?.0)(name)
}

/// `text` with the shortcut of the element `id` after it ("Notifications
/// (Ctrl Shift B)"). Returns `text` unchanged when the element has no
/// shortcut or `text` already names it.
pub fn tooltip_with_shortcut(
    cx: &gpui::App,
    id: &gpui::ElementId,
    text: SharedString,
) -> SharedString {
    match element_shortcut(cx, id) {
        Some(shortcut) if !text.contains(shortcut.as_ref()) => {
            SharedString::from(format!("{text} ({shortcut})"))
        }
        _ => text,
    }
}

/// The last key pressed in any window, and the interceptor that records it.
#[derive(Default)]
struct LastKeystroke(Option<Keystroke>, Option<gpui::Subscription>);

impl Global for LastKeystroke {}

/// Starts recording the last key pressed in any window, for
/// [`last_keystroke`]. The keymap calls it once at startup
/// (`dbflux_ui_base::keymap::init_keymap`).
pub fn record_last_keystroke(cx: &mut App) {
    let recorder = cx.intercept_keystrokes(|event, _window, cx| {
        cx.default_global::<LastKeystroke>().0 = Some(event.keystroke.clone());
    });
    cx.default_global::<LastKeystroke>().1 = Some(recorder);
}

/// The key whose binding is being dispatched: interceptors run before key
/// bindings, so while a binding's action runs this is the keystroke that
/// matched it (the last one of a key sequence).
pub fn last_keystroke(cx: &App) -> Option<Keystroke> {
    cx.try_global::<LastKeystroke>()
        .and_then(|recorded| recorded.0.clone())
}

#[cfg(test)]
mod tests {
    use super::{ElementShortcuts, tooltip_with_shortcut};
    use gpui::{ElementId, SharedString, TestAppContext};

    #[gpui::test]
    fn a_tooltip_names_the_shortcut_once(cx: &mut TestAppContext) {
        let id = ElementId::Name("notifications-bell".into());

        cx.update(|cx| {
            let plain = SharedString::from("Notifications");
            assert_eq!(tooltip_with_shortcut(cx, &id, plain.clone()), plain);

            cx.set_global(ElementShortcuts(Box::new(|id| {
                (id == "notifications-bell").then(|| SharedString::from("Ctrl Shift B"))
            })));

            assert_eq!(
                tooltip_with_shortcut(cx, &id, plain.clone()),
                "Notifications (Ctrl Shift B)"
            );
            assert_eq!(
                tooltip_with_shortcut(cx, &id, "Notifications (Ctrl Shift B)".into()),
                "Notifications (Ctrl Shift B)"
            );
            assert_eq!(
                tooltip_with_shortcut(cx, &ElementId::Name("other".into()), plain.clone()),
                plain
            );
        });
    }
}
