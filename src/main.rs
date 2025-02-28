#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use crate::app::MinesweeperApp;
use eframe::NativeOptions;

mod app;
mod board;
mod data;

fn main() {
    //sensible defaults for first time running
    let size = 25;
    let number_of_mines = 50;

    //don't need to change any of the native options
    let options = NativeOptions::default();

    eframe::run_native(
        "Minesweeper",
        options,
        Box::new(|cc| {
            Ok(Box::new(
                MinesweeperApp::new(size, number_of_mines, cc).expect("unable to create board"),
            ))
        }),
    )
    .expect("unable to create app");
}
