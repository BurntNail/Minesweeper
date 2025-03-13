#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use crate::app::MinesweeperApp;
use crate::board::SpriteAtlas;
use eframe::NativeOptions;

mod app;
mod board;
mod data;
mod ser;
mod time_sampler;

fn main() {
    //sensible defaults for first time running
    let size = 10;
    let number_of_mines = 10;

    //don't need to change any of the native options
    let options = NativeOptions::default();

    let sprite_atlas = SpriteAtlas::default();

    eframe::run_native(
        "Minesweeper",
        options,
        Box::new(|cc| {
            Ok(Box::new(
                MinesweeperApp::new(size, size, number_of_mines, sprite_atlas, cc)
                    .expect("unable to create board"),
            ))
        }),
    )
    .expect("unable to create app");
}
