use tui::backend::Backend;
use tui::Frame;

use crate::ui::ui_app::UIApp;

/// render ui components
pub fn render<B>(rect: &mut Frame<B>, app: &UIApp)
    where
        B: Backend,
{
    println!("render ui")
}
