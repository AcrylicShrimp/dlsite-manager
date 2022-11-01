use super::WindowInfoProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SettingWindow;

impl WindowInfoProvider for SettingWindow {
    fn label(&self) -> String {
        "setting".to_owned()
    }

    fn entry(&self) -> String {
        "setting".to_owned()
    }

    fn title(&self) -> String {
        "Settings - DLsite Manager".to_owned()
    }

    fn size(&self) -> (f64, f64) {
        (600f64, 300f64)
    }

    fn resizable(&self) -> bool {
        false
    }
}
