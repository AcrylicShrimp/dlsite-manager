use super::WindowInfoProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountEditWindow {
    pub account_id: i64,
}

impl WindowInfoProvider for AccountEditWindow {
    fn label(&self) -> String {
        "account-edit".to_owned()
    }

    fn entry(&self) -> String {
        "account-edit".to_owned()
    }

    fn title(&self) -> String {
        "Edit Account - DLsite Manager".to_owned()
    }

    fn size(&self) -> (f64, f64) {
        (320f64, 450f64)
    }

    fn resizable(&self) -> bool {
        false
    }

    fn init_scripts(&self) -> Vec<String> {
        vec![format!("window.accountId = {}", self.account_id)]
    }
}
