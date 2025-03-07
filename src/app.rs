use crate::board::Board;
use crate::data::{Data, InvalidDataError};
use eframe::epaint::ColorImage;
use eframe::{App, CreationContext, Frame, Storage};
use egui::{
    Color32, Context, CursorIcon, Grid, Rect, Scene, Sense, Slider, TextureHandle, TextureOptions,
    Widget, pos2,
};
use image::{ImageFormat, ImageReader};
use std::io::Cursor;
use std::time::{Duration, Instant};

///Struct to keep a hold of all things related to the minesweeper UI/app
pub struct MinesweeperApp {
    ///The actual minesweeper board
    board: Board,
    ///When the game started - can be [`None`] if nothing has been placed yet
    game_started: Option<Instant>,
    ///When the game finished - will be [`None`] until the game finishes.
    game_stopped: Option<Instant>,
    ///The [`Rect`] used to draw the board - this is used for the [`Scene`] that allows pan/zoom-ing.
    board_rect: Rect,
    ///The width of the next board to be created
    next_width: usize,
    ///The width of the next board to be created
    next_height: usize,
    ///The number of mines in the next board to be created
    next_mines: usize,
    cached_counts: Vec<u8>,
    image_handle: TextureHandle,
}

impl MinesweeperApp {
    pub fn new(
        width: usize,
        height: usize,
        number_of_mines: usize,
        cc: &CreationContext,
    ) -> Result<Self, InvalidDataError> {
        //assume we can't get any previous data
        let mut previous_data = None;

        let image_handle = {
            const BYTES: &[u8] = include_bytes!("../WinmineXP.png");

            let bytes = Cursor::new(BYTES);

            let dynimage = ImageReader::with_format(bytes, ImageFormat::Png)
                .decode()
                .expect("unable to decode image")
                .to_rgba8();
            let (w, h) = dynimage.dimensions();
            let pixels = dynimage.as_flat_samples();
            let img =
                ColorImage::from_rgba_unmultiplied([w as usize, h as usize], pixels.as_slice());

            cc.egui_ctx
                .load_texture("winminexptex", img, TextureOptions::NEAREST)
        };

        //but if we can get a data key
        if let Some(data) = cc.storage.and_then(|x| x.get_string("data")) {
            //and we can parse it
            match Data::try_from(data) {
                //then now we have the previous data
                Ok(x) => previous_data = Some(x),
                Err(e) => {
                    //if not, then we can leave `previous_data` as is, and just print an error with why it failed
                    eprintln!("Error parsing previous data: {e:?}");
                }
            }
        }

        //then we use that to either create a board with the previous data, or we just use the defaults
        //both of the Board creation methods return Results which avoid logic errors
        let board = previous_data.map_or_else(
            || Board::new(width, height, number_of_mines),
            Board::from_previous_data,
        )?;

        Ok(Self {
            next_width: board.get_width(),
            next_height: board.get_height(),
            next_mines: board.total_mines(),
            board_rect: Rect::ZERO,
            game_started: None,
            game_stopped: None,
            cached_counts: board.generate_counts().unwrap_or_default(),
            board,
            image_handle,
        })
    }
}

impl App for MinesweeperApp {
    #[allow(clippy::too_many_lines)]
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::TopBottomPanel::top("top panel").show(ctx, |ui| {
            let status = match (
                self.board.game_has_been_lost(),
                self.board.game_has_been_won(),
            ) {
                (true, _) => format!(
                    "Game Lost: {} correct flag(s) in {:?}",
                    self.board.successfully_flagged(),
                    match self.game_started.zip(self.game_stopped) {
                        Some((start, stop)) => stop - start,
                        None => Duration::from_secs(0),
                    }
                ),
                (_, true) => format!("Game Won in {:?}", {
                    match self.game_started.zip(self.game_stopped) {
                        Some((start, stop)) => stop - start,
                        None => Duration::from_secs(u64::MAX),
                    }
                }),
                _ => format!("Game in progress for {}s", {
                    self.game_started.map_or(0, |start| {
                        ctx.request_repaint_after_secs(0.25);
                        start.elapsed().as_secs()
                    })
                }),
            };

            Grid::new("top bit grid").show(ui, |ui| {
                {
                    ui.label(status);

                    #[allow(clippy::useless_let_if_seq)]
                    let mut reset_vars = false;
                    if ui.button("Give Up?").clicked() {
                        self.board.give_up();
                        reset_vars = true;
                    }
                    if ui.button("Reset Game?").clicked() {
                        self.board.reset(Some((
                            self.next_width,
                            self.next_height,
                            self.next_mines,
                        )));
                        reset_vars = true;
                    }

                    if reset_vars {
                        self.game_started = None;
                        self.game_stopped = None;
                        self.cached_counts.clear();
                    }
                }
                ui.end_row();
                {
                    ui.label("Width: ");
                    let min_width = self.next_height.min(2);
                    Slider::new(&mut self.next_width, min_width..=100)
                        .logarithmic(true)
                        .ui(ui);

                    ui.label(format!("Flags Placed: {}", self.board.flags_placed()));
                }
                ui.end_row();
                {
                    ui.label("Height: ");
                    let min_height = self.next_width.min(2);
                    Slider::new(&mut self.next_height, min_height..=100)
                        .logarithmic(true)
                        .ui(ui);

                    ui.label(format!("Total Mines: {}", self.board.total_mines()));
                }
                ui.end_row();
                {
                    ui.label("Mines: ");
                    let max_mines = self.next_width * self.next_height - 1;
                    Slider::new(&mut self.next_mines, 1..=max_mines)
                        .logarithmic(true)
                        .ui(ui);

                    ui.label(format!(
                        "Undiscovered & Unflagged: {}",
                        self.board.total_uninteracted()
                    ));
                }
                ui.end_row();
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut inner_rect = Rect::ZERO;

            let rsp = Scene::new()
                .zoom_range(0.05..=5.0)
                .show(ui, &mut self.board_rect, |ui| {
                    let mut rect = ui.available_rect_before_wrap();
                    let available_aspect_ratio = rect.width() / rect.height();

                    let (board_width, board_height) = (
                        self.board.get_width() as f32,
                        self.board.get_height() as f32,
                    );
                    let board_aspect_ratio = board_width / board_height;

                    let (sf_x, sf_y) = if available_aspect_ratio > board_aspect_ratio {
                        (available_aspect_ratio / board_aspect_ratio, 1.0)
                    } else {
                        (1.0, board_aspect_ratio / available_aspect_ratio)
                    };

                    rect.max.x = rect.min.x + rect.width() / sf_x;
                    rect.max.y = rect.min.y + rect.height() / sf_y;

                    let width_to_be_used = rect.width() * 0.9;
                    let height_to_be_used = rect.height() * 0.9;

                    let cell_size = rect.width() / board_width;

                    let start_x = rect.left() + (rect.width() - width_to_be_used) / 2.0;
                    let mut start_y = rect.top() + (rect.height() - height_to_be_used) / 2.0;

                    let counts = {
                        if self.cached_counts.is_empty() {
                            if let Some(counts) = self.board.generate_counts() {
                                self.cached_counts = counts;
                            }
                        }

                        self.cached_counts.as_slice()
                    };

                    let game_is_over =
                        self.board.game_has_been_won() || self.board.game_has_been_lost();

                    let mut row = 0;
                    for (index, cell) in self.board.render().into_iter().enumerate() {
                        let column = index % self.board.get_width();

                        let entire_thing_rect = Rect {
                            min: pos2(cell_size.mul_add(column as f32, start_x), start_y),
                            max: pos2(
                                cell_size.mul_add((column + 1) as f32, start_x),
                                start_y + cell_size,
                            ),
                        };

                        ui.painter().image(
                            self.image_handle.id(),
                            entire_thing_rect,
                            cell.to_uv(
                                counts.get(index).copied().unwrap_or_default(),
                                game_is_over,
                            ),
                            Color32::WHITE,
                        );

                        let rsp = ui
                            .allocate_rect(entire_thing_rect, Sense::CLICK)
                            .on_hover_cursor(CursorIcon::PointingHand);

                        let pos = (column, row);
                        if rsp.clicked() {
                            let caused_stop = self.board.click(pos);

                            if self.game_started.is_none() {
                                self.game_started = Some(Instant::now());
                            }
                            if caused_stop && self.game_stopped.is_none() {
                                self.game_stopped = Some(Instant::now());
                            }
                        } else if rsp.secondary_clicked() {
                            if self.game_started.is_none() {
                                self.game_started = Some(Instant::now());
                            }
                            self.board.toggle_flag(pos);
                        }

                        if column == self.board.get_width() - 1 {
                            start_y += cell_size;
                            row += 1;
                        }
                    }

                    inner_rect = ui.min_rect();
                });

            if rsp.response.double_clicked() {
                self.board_rect = inner_rect;
            }
        });
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        let data = String::from(self.board.get_data().clone());
        storage.set_string("data", data);
    }
}
