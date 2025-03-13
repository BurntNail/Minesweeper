use crate::board::{Board, SpriteAtlas, TextureCache};
use crate::ser::{InvalidDataError, deserialise_extra_time, serialise_extra_time};
use crate::time_sampler::TimeSampler;
use eframe::{App, CreationContext, Frame, Storage};
use egui::{
    Color32, Context, CursorIcon, Grid, Rect, Scene, Sense, Slider, TextureHandle, Widget, pos2,
};
use std::time::{Duration, Instant};

///Struct to keep a hold of all things related to the minesweeper UI/app
pub struct MinesweeperApp {
    ///The actual minesweeper board
    board: Board,
    ///Any extra time to add to the count from previous sessions
    extra_time: Duration,
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
    ///A cached copy of the hints used for how many mines are nearby
    cached_counts: Vec<u8>,
    ///A handle to the sprite atlas
    image_handle: TextureHandle,
    ///A sampler for frametimes
    frametime_counter: TimeSampler<50>,
    sprite_atlas: SpriteAtlas,
    texture_cache: TextureCache,
}

impl MinesweeperApp {
    pub fn new(
        width: usize,
        height: usize,
        number_of_mines: usize,
        sprite_atlas: SpriteAtlas,
        cc: &CreationContext,
    ) -> Result<Self, InvalidDataError> {
        //assume we can't get any previous data
        let mut previous_data = None;
        let mut extra_time = Duration::new(0, 0);
        let mut game_started = None;
        let mut texture_cache = TextureCache::default();

        let image_handle = {
            cc.egui_ctx.input_mut(|input_state| {
                input_state.max_texture_side = SpriteAtlas::MAX_TEXTURE_SIDE;
            });

            texture_cache.get(sprite_atlas, &cc.egui_ctx)
        };

        //if we have storage
        if let Some(storage) = cc.storage {
            //try to get the data
            if let Some(data) = storage.get_string("data") {
                //and parse it
                match data.parse() {
                    //then now we have the previous data
                    Ok(x) => previous_data = Some(x),
                    Err(e) => {
                        //if not, then we can leave `previous_data` as is, and just print an error with why it failed
                        eprintln!("Error parsing previous data: {e:?}");
                    }
                }
            }

            //try to get previous session time
            if let Some(sered) = storage.get_string("extratime") {
                //and deserialise it
                match deserialise_extra_time(sered) {
                    Ok(dur) => {
                        //if it isn't zero, assume we're still playing and set the start time to now
                        if !dur.is_zero() {
                            extra_time = dur;
                            game_started = Some(Instant::now());
                        }
                    }
                    Err(e) => eprintln!("Error deser-ing extra time: {e:?}"),
                }
            }
        }

        //then we use that to either create a board with the previous data, or we just use the defaults
        //both of the Board creation methods return Results which avoid logic errors
        let board = previous_data.map_or_else(
            || Board::new(width, height, number_of_mines),
            Board::from_previous_data,
        )?;

        //if the game is over, cancel the start time because otherwise it'll cause shenanigans
        if board.game_is_over() {
            game_started = None;
        }

        Ok(Self {
            next_width: board.get_width(),
            next_height: board.get_height(),
            next_mines: board.total_mines(),
            board_rect: Rect::ZERO,
            game_started,
            game_stopped: None,
            cached_counts: board.generate_counts().unwrap_or_default(),
            board,
            image_handle,
            extra_time,
            sprite_atlas,
            frametime_counter: TimeSampler::new(),
            texture_cache,
        })
    }
}

impl App for MinesweeperApp {
    #[allow(clippy::too_many_lines)]
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.frametime_counter.start_timer();
        egui::TopBottomPanel::top("top panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                //start a grid
                Grid::new("top bit grid").show(ui, |ui| {
                    {
                        //get the status text depending on the game state
                        ui.label(
                            match (
                                self.board.game_has_been_lost(),
                                self.board.game_has_been_won(),
                            ) {
                                (true, _) => format!(
                                    "Game Lost: {} correct flag(s) in {:?}",
                                    self.board.successfully_flagged(),
                                    match self.game_started.zip(self.game_stopped) {
                                        Some((start, stop)) => stop - start + self.extra_time,
                                        None => self.extra_time,
                                    }
                                ),
                                (_, true) => format!("Game Won in {:?}", {
                                    match self.game_started.zip(self.game_stopped) {
                                        Some((start, stop)) => stop - start + self.extra_time,
                                        None => self.extra_time,
                                    }
                                }),
                                _ => format!("Game in progress for {}s", {
                                    (self.game_started.map_or(Duration::new(0, 0), |start| {
                                        ctx.request_repaint_after_secs(0.25);
                                        start.elapsed()
                                    }) + self.extra_time)
                                        .as_secs()
                                }),
                            },
                        );

                        //allow either giving up or resetting
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

                        //if we did either, reset various variables
                        if reset_vars {
                            self.game_started = None;
                            self.game_stopped = None;
                            self.extra_time = Duration::new(0, 0);
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

                ui.vertical(|ui| {
                    let fps = {
                        let secs = self.frametime_counter.get_average().as_secs_f64();
                        1.0 / secs
                    };

                    ui.label(format!(
                        "Current FPS: {fps:?}",
                    ));

                    let old_sprite_atlas = self.sprite_atlas;

                    for (atlas, name) in SpriteAtlas::ALL_VARIANTS
                        .into_iter()
                        .map(|x| (x, x.as_static_str()))
                    {
                        ui.radio_value(&mut self.sprite_atlas, atlas, name);
                    }

                    if old_sprite_atlas != self.sprite_atlas {
                        self.image_handle = self.texture_cache.get(self.sprite_atlas, ctx);
                    }
                });
            })
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            //get the response (for double-click checking), also wrapping the reset rect
            let rsp = Scene::new()
                .zoom_range(0.05..=10.0)
                .show(ui, &mut self.board_rect, |ui| {
                    //work out the display aspect ratio
                    let mut rect = ui.available_rect_before_wrap();
                    let available_aspect_ratio = rect.width() / rect.height();

                    //work out the board aspect ratio
                    let (board_width, board_height) = (
                        self.board.get_width() as f32,
                        self.board.get_height() as f32,
                    );
                    let board_aspect_ratio = board_width / board_height;

                    //get scale factors for the space to actually use so the board is as big as can be
                    let (sf_x, sf_y) = if available_aspect_ratio > board_aspect_ratio {
                        (available_aspect_ratio / board_aspect_ratio, 1.0)
                    } else {
                        (1.0, board_aspect_ratio / available_aspect_ratio)
                    };

                    //modify the display rect using the factors
                    rect.max.x = rect.min.x + rect.width() / sf_x;
                    rect.max.y = rect.min.y + rect.height() / sf_y;

                    //padding around the edges - 5% on each side
                    let width_to_be_used = rect.width() * 0.9;
                    let height_to_be_used = rect.height() * 0.9;

                    let cell_size = width_to_be_used / board_width;

                    let start_x = rect.left() + (rect.width() - width_to_be_used) / 2.0;
                    let mut start_y = rect.top() + (rect.height() - height_to_be_used) / 2.0;

                    //get the hints
                    let counts = {
                        //if we don't have any
                        if self.cached_counts.is_empty() {
                            //try to regenerate them - this could be `None` if the mines haven't been generated yet
                            if let Some(counts) = self.board.generate_counts() {
                                self.cached_counts = counts;
                            }
                        }

                        self.cached_counts.as_slice()
                    };

                    let game_is_over = self.board.game_is_over();

                    let mut column = 0;
                    let mut row = 0;
                    for (index, cell) in self.board.render().into_iter().enumerate() {
                        let pos = (column, row);

                        //work out the rect for the whole cell
                        let entire_thing_rect = Rect {
                            min: pos2(cell_size.mul_add(column as f32, start_x), start_y),
                            max: pos2(
                                cell_size.mul_add((column + 1) as f32, start_x),
                                start_y + cell_size,
                            ),
                        };
                        //get the UV coordinates on the sprite atlas
                        let uv_rect = cell.to_uv(
                            counts.get(index).copied().unwrap_or_default(),
                            game_is_over,
                            self.sprite_atlas,
                        );

                        ui.painter().image(
                            self.image_handle.id(),
                            entire_thing_rect,
                            uv_rect,
                            Color32::WHITE,
                        );

                        //allocate a rect for checking clicks and making the cursor correct
                        let rsp = ui
                            .allocate_rect(entire_thing_rect, Sense::CLICK)
                            .on_hover_cursor(CursorIcon::PointingHand);

                        let mut interaction_happened = false;
                        let mut game_is_now_over = false;

                        if rsp.clicked() {
                            interaction_happened = true;
                            game_is_now_over = self.board.click(pos);
                        } else if rsp.secondary_clicked() {
                            interaction_happened = true;
                            game_is_now_over = self.board.toggle_flag(pos);
                        }

                        if interaction_happened && self.game_started.is_none() {
                            self.game_started = Some(Instant::now());
                        }
                        if game_is_now_over && self.game_stopped.is_none() {
                            self.game_stopped = Some(Instant::now());
                        }

                        if column == self.board.get_width() - 1 {
                            start_y += cell_size;
                            row += 1;
                        }
                        column = (column + 1) % self.board.get_width();
                    }

                    ui.min_rect()
                });

            if rsp.response.double_clicked() {
                self.board_rect = rsp.inner;
            }
        });

        self.frametime_counter.stop_timer();
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        let data = String::from(self.board.get_data().clone());
        storage.set_string("data", data);

        let extra_time = serialise_extra_time(match (self.game_started, self.game_stopped) {
            (None, None) => Duration::new(0, 0),
            (Some(started), None) => started.elapsed() + self.extra_time,
            (None, Some(_stopped)) => unreachable!("cannot have stopped w/o started"),
            (Some(_started), Some(_stopped)) => Duration::new(0, 0),
        });

        storage.set_string("extratime", extra_time);
    }
}
