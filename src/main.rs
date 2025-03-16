#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use minesweeper::app::MinesweeperApp;
use minesweeper::board::SpriteAtlas;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    //sensible defaults for first time running
    let size = 10;
    let number_of_mines = 10;
    let sprite_atlas = SpriteAtlas::default();

    //don't need to change any of the native options
    let options = eframe::NativeOptions::default();


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

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("minesweeper_canvas_id")
            .expect("Failed to find minesweeper_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        //sensible defaults for first time running
        let size = 10;
        let number_of_mines = 10;
        let sprite_atlas = SpriteAtlas::default();


        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(move |cc| {
                    Ok(Box::new(
                        MinesweeperApp::new(size, size, number_of_mines, sprite_atlas, cc)
                            .expect("unable to create board"),
                    ))
                }),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}