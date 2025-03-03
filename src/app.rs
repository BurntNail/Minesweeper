use crate::board::{Board, GridElementType};
use crate::data::{Data, InvalidDataError};
use eframe::epaint::StrokeKind;
use eframe::{App, CreationContext, Frame, Storage};
use egui::{
    Align2, Color32, Context, CursorIcon, FontId, Grid, Rect, Scene, Sense, Slider, Stroke, Widget,
    pos2, vec2,
};
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
    ///The number of mines in the next board to be created
    next_mines: usize,
    cached_counts: Vec<u8>,
}

impl MinesweeperApp {
    pub fn new(
        width: usize,
        number_of_mines: usize,
        cc: &CreationContext,
    ) -> Result<Self, InvalidDataError> {
        //assume we can't get any previous data
        let mut previous_data = None;

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
            || Board::new(width, number_of_mines),
            Board::from_previous_data,
        )?;

        Ok(Self {
            next_width: board.get_width(),
            next_mines: board.total_mines(),
            board_rect: Rect::ZERO,
            game_started: None,
            game_stopped: None,
            cached_counts: board.generate_counts(),
            board,
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
                        ctx.request_repaint_after_secs(1.0);
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
                        self.board.reset(Some((self.next_width, self.next_mines)));
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
                    ui.label("Width/Height: ");
                    let min_width = ((self.next_mines as f32).sqrt().ceil() as usize).max(2);
                    Slider::new(&mut self.next_width, min_width..=100)
                        .logarithmic(true)
                        .ui(ui);

                    ui.label(format!("Flags Placed: {}", self.board.flags_placed()));
                }
                ui.end_row();
                {
                    ui.label("Mines: ");
                    let max_mines = self.next_width * self.next_width - 1;
                    Slider::new(&mut self.next_mines, 1..=max_mines)
                        .logarithmic(true)
                        .ui(ui);

                    ui.label(format!("Total Mines: {}", self.board.total_mines()));
                }
                ui.end_row();
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut inner_rect = Rect::ZERO;

            let rsp = Scene::new()
                .zoom_range(0.05..=5.0)
                .show(ui, &mut self.board_rect, |ui| {
                    let available_space = ui.available_rect_before_wrap();

                    let width_to_be_used =
                        available_space.width().min(available_space.height()) * 0.95;
                    let cell_width = width_to_be_used / self.board.get_width() as f32;
                    let stroke_width = (cell_width * 0.1).max(1.0);

                    let flag_cell_width = cell_width * 0.5;
                    let flag_cell_delta_pos = (cell_width - flag_cell_width) / 2.0;

                    let start_x = available_space.left()
                        + (available_space.width() - width_to_be_used - stroke_width) / 2.0;
                    let mut start_y = available_space.top()
                        + (available_space.height() - width_to_be_used - stroke_width) / 2.0;

                    let counts = {
                        if self.cached_counts.is_empty() {
                            self.cached_counts = self.board.generate_counts();
                        }

                        self.cached_counts.as_slice()
                    };

                    let mut row = 0;
                    for (index, cell) in self.board.render().into_iter().enumerate() {
                        let column = index % self.board.get_width();

                        let entire_thing_rect = Rect {
                            min: pos2(cell_width.mul_add(column as f32, start_x), start_y),
                            max: pos2(
                                cell_width.mul_add((column + 1) as f32, start_x),
                                start_y + cell_width,
                            ),
                        };

                        let colour = match cell.ty {
                            GridElementType::Discovered => Color32::DARK_GRAY,
                            GridElementType::Exploded => Color32::RED,
                            GridElementType::Mine => {
                                if self.board.game_has_been_won() {
                                    Color32::GREEN
                                } else {
                                    Color32::PURPLE
                                }
                            }
                            GridElementType::Undiscovered => Color32::WHITE,
                        };

                        ui.painter().rect(
                            entire_thing_rect,
                            0.0,
                            colour,
                            Stroke::new(stroke_width, Color32::GRAY),
                            StrokeKind::Middle,
                        );
                        if cell.flagged {
                            let min = entire_thing_rect.min
                                + vec2(flag_cell_delta_pos, flag_cell_delta_pos);
                            let rect = Rect {
                                min,
                                max: min + vec2(flag_cell_width, flag_cell_width),
                            };
                            ui.painter().rect_filled(rect, 0.0, Color32::BLUE);
                        }
                        if cell.should_display_count {
                            ui.painter().text(
                                pos2(
                                    entire_thing_rect.min.x + cell_width / 2.0,
                                    entire_thing_rect.min.y + cell_width / 2.0,
                                ),
                                Align2::CENTER_CENTER,
                                counts[index].to_string(),
                                FontId::monospace(cell_width / 4.0 * 3.0),
                                Color32::BLACK,
                            );
                        }

                        let rsp = ui
                            .allocate_rect(
                                {
                                    let delta = stroke_width;
                                    Rect {
                                        min: entire_thing_rect.min + vec2(delta, delta),
                                        max: entire_thing_rect.max - vec2(delta, delta),
                                    }
                                },
                                Sense::CLICK,
                            )
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
                            start_y += cell_width;
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
