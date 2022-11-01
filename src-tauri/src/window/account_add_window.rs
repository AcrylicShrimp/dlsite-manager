use super::WindowInfoProvider;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountAddWindow;

impl WindowInfoProvider for AccountAddWindow {
    fn label(&self) -> String {
        "account-add".to_owned()
    }

    fn entry(&self) -> String {
        "account-add".to_owned()
    }

    fn title(&self) -> String {
        "Add Account - DLsite Manager".to_owned()
    }

    fn size(&self) -> (f64, f64) {
        (320f64, 450f64)
    }

    fn resizable(&self) -> bool {
        false
    }
}
