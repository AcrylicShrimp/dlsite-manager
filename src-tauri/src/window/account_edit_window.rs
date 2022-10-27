use super::WindowInfoProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountEditWindow {
    pub account_id: i64,
}

impl WindowInfoProvider for AccountEditWindow {
    fn label(&self) -> String {
        format!("account-edit-{}", self.account_id)
    }

    fn entry(&self) -> String {
        format!("account-edit/{}", self.account_id)
    }

    fn title(&self) -> String {
        "DLsite Manager - Edit Account".to_owned()
    }

    fn size(&self) -> (f64, f64) {
        (320f64, 450f64)
    }

    fn resizable(&self) -> bool {
        false
    }
}
