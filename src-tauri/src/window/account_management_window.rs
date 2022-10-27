use super::WindowInfoProvider;

pub struct AccountManagementWindow;

impl WindowInfoProvider for AccountManagementWindow {
    fn label(&self) -> String {
        "account-management".to_owned()
    }

    fn entry(&self) -> String {
        "account-management".to_owned()
    }

    fn title(&self) -> String {
        "DLsite Manager - Account Management".to_owned()
    }

    fn size(&self) -> (f64, f64) {
        (600f64, 500f64)
    }

    fn resizable(&self) -> bool {
        true
    }
}
