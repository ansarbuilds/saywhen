//! Labels for the menu-bar / tray menu. The tray builder uses this list so the
//! wording can be tested without opening a window.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayLabel {
    pub id: &'static str,
    pub label: &'static str,
}

pub fn tray_menu_labels() -> &'static [TrayLabel] {
    &[
        TrayLabel { id: "open", label: "Open saywhen" },
        TrayLabel { id: "camera", label: "Pause or resume the camera" },
        TrayLabel { id: "hud", label: "Show or hide the pill" },
        TrayLabel { id: "place", label: "Move the pill to this display" },
        TrayLabel { id: "cancel", label: "Cancel the current line" },
        TrayLabel { id: "quit", label: "Quit saywhen" },
    ]
}

pub fn tray_label(id: &str) -> Option<&'static str> {
    tray_menu_labels()
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_menu_has_the_six_actions() {
        let labels = tray_menu_labels();
        assert_eq!(labels.len(), 6);
        assert_eq!(tray_label("cancel"), Some("Cancel the current line"));
        assert_eq!(tray_label("missing"), None);
        assert!(labels.iter().any(|item| item.id == "quit"));
    }
}
