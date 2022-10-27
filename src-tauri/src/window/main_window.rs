use super::WindowInfoProvider;

pub struct MainWindow;

impl WindowInfoProvider for MainWindow {
    fn label(&self) -> String {
        "main".to_owned()
    }

    fn entry(&self) -> String {
        "main".to_owned()
    }

    fn title(&self) -> String {
        "DLsite Manager".to_owned()
    }

    fn size(&self) -> (f64, f64) {
        (800f64, 600f64)
    }

    fn resizable(&self) -> bool {
        true
    }
}
