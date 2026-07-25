use crate::ui::{LogosUI, UIFocus};
use std::time::Duration;

#[test]
fn test_focus_switch() {
    let mut ui = LogosUI::new();
    assert_eq!(ui.focus, UIFocus::Input);

    ui.handle_focus_switch();
    assert_eq!(ui.focus, UIFocus::Chat);

    ui.handle_focus_switch();
    assert_eq!(ui.focus, UIFocus::KB);

    ui.handle_focus_switch();
    assert_eq!(ui.focus, UIFocus::Planning);

    ui.handle_focus_switch();
    assert_eq!(ui.focus, UIFocus::Input);
}

#[test]
fn test_tab_autocomplete_navigation() {
    let mut ui = LogosUI::new();
    // Manually mock some suggestions
    ui.autocomplete_suggestions = vec!["help".to_string(), "clear".to_string(), "kb".to_string()];
    ui.autocomplete_index = 0;

    // First tab: move to index 1
    ui.handle_tab();
    assert_eq!(ui.autocomplete_index, 1);

    // Second tab: move to index 2
    ui.handle_tab();
    assert_eq!(ui.autocomplete_index, 2);

    // Third tab: select last suggestion
    ui.handle_tab();
    assert_eq!(ui.input_textarea.text(), "/kb");
}
