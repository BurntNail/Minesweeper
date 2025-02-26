#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use crate::app::MinesweeperApp;
use eframe::NativeOptions;

mod app;
mod board;

fn main() {
    let size = 25;
    let number_of_mines = 50;

    let options = NativeOptions::default();

    eframe::run_native(
        "Minesweeper EGUI",
        options,
        Box::new(|cc| {
            Ok(Box::new(
                MinesweeperApp::new(size, number_of_mines, cc).expect("unable to create board"),
            ))
        }),
    )
    .expect("unable to create app");
}
