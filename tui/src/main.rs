#![allow(clippy::result_large_err)]

use nest_error::NestResult;
use nest_tui::prelude::*;

mod screens;
use screens::MainScreen;

fn main() -> NestResult<()> {
    TuiApp::new("finch").screen(MainScreen::new()).try_run()
}
