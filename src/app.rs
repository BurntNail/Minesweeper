use std::fmt::{Display, Formatter};
use std::num::{ParseIntError};
use crate::board::{Board, Data, GridElementType};
use eframe::epaint::StrokeKind;
use eframe::{App, CreationContext, Frame, Storage};
use egui::{Align2, Color32, Context, FontId, Rect, Sense, Stroke, pos2, vec2, Grid, Slider, Widget, CursorIcon, Scene};
use std::collections::HashSet;

pub struct MinesweeperApp {
    board: Board,
    board_rect: Rect,
    next_width: usize,
    next_mines: usize,
}

#[derive(Debug)]
pub enum DataReadError {
    UnableToParseInteger(ParseIntError),
    NotEnoughElements,
    InvalidCharacter(char)
}

impl From<ParseIntError> for DataReadError {
    fn from(value: ParseIntError) -> Self {
        Self::UnableToParseInteger(value)
    }
}


impl Display for DataReadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DataReadError::UnableToParseInteger(e) => write!(f, "Error parsing integer: {e}"),
            DataReadError::NotEnoughElements => write!(f, "Not enough elements compared to length counts provided"),
            DataReadError::InvalidCharacter(ch) => write!(f, "Found non-integer, non-comma character: {ch:?}"),
        }
    }
}

impl std::error::Error for DataReadError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        if let DataReadError::UnableToParseInteger(e) = &self {
            Some(e)
        } else {
            None
        }
    }
}

impl TryFrom<String> for Data {
    type Error = DataReadError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        //i could do a big state machine, but i cba and this works well enough
        let mut lengths = [0; 4];
        let mut numbers = vec![];

        let mut accum = String::new();

        let mut i = 0;
        for ch in value.chars() {
            if ch.is_ascii_digit() {
                accum.push(ch);
            } else if ch == ',' {
                let parsed = accum.parse()?;
                accum.clear();

                if i <= 3 {
                    lengths[i] = parsed;
                } else {
                    numbers.push(parsed);
                }

                if i == 3 {
                    numbers.reserve(lengths[1] + lengths[2] + lengths[3]);
                }

                i += 1;
            }
        }
        numbers.push(accum.parse()?);

        let [width, n_flagged, n_clicked, number_of_mines] = lengths;

        let (mut flagged, mut clicked, mut mines) = (HashSet::new(), HashSet::new(), HashSet::new());
        //TODO: DRY
        for _ in 0..number_of_mines {
            let Some(y) = numbers.pop() else {
                return Err(DataReadError::NotEnoughElements);
            };
            let Some(x) = numbers.pop() else {
                return Err(DataReadError::NotEnoughElements);
            };

            mines.insert((x, y));
        }
        for _ in 0..n_clicked {
            let Some(y) = numbers.pop() else {
                return Err(DataReadError::NotEnoughElements);
            };
            let Some(x) = numbers.pop() else {
                return Err(DataReadError::NotEnoughElements);
            };

            clicked.insert((x, y));
        }
        for _ in 0..n_flagged {
            let Some(y) = numbers.pop() else {
                return Err(DataReadError::NotEnoughElements);
            };
            let Some(x) = numbers.pop() else {
                return Err(DataReadError::NotEnoughElements);
            };

            flagged.insert((x, y));
        }


        Ok(Data {
            width,
            number_of_mines,
            flagged,
            clicked,
            mines
        })
    }
}

impl From<Data> for String {
    fn from(Data{ width, number_of_mines: _, flagged, clicked, mines }: Data) -> Self {
        let mut output = format!("{width},{},{},{}", flagged.len(), clicked.len(), mines.len());
        for (x, y) in flagged.into_iter().chain(clicked).chain(mines.into_iter()) {
            output.push_str(&format!(",{x},{y}"));
        }
        output
    }
}


impl MinesweeperApp {
    pub fn new(width: usize, number_of_mines: usize, cc: &CreationContext) -> Option<Self> {
        let mut previous_data = None;

        if let Some(data) = cc.storage.and_then(|x| x.get_string("data")) {
            match data.try_into() {
                Ok(x) => previous_data = Some(x),
                Err(e) => {
                    eprintln!("Error parsing previous data: {e:?}");
                }
            }
        }

        match previous_data {
            Some(x) => {
                let board = Board::from_previous_data(x);
                Some(Self {
                    next_width: board.get_width(),
                    next_mines: board.total_mines(),
                    board,
                    board_rect: Rect::ZERO
                })
            }
            None => {
                Board::new(width, number_of_mines).map(|board| {
                    Self { board,
                        next_width: width,
                        next_mines: number_of_mines,
                        board_rect: Rect::ZERO
                    }
                })
            }
        }

    }
}

impl App for MinesweeperApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::TopBottomPanel::top("top panel").show(ctx, |ui| {
            let status = match (
                self.board.game_has_been_lost(),
                self.board.game_has_been_won(),
            ) {
                (true, _) => format!("Game Lost: {} correct flag(s)", self.board.successfully_flagged()),
                (_, true) => "Game Won".to_string(),
                _ => "Game still being played".to_string(),
            };

            Grid::new("top bit grid").show(ui, |ui| {
                {
                    ui.label(status);

                    if ui.button("Give Up?").clicked() {
                        self.board.give_up();
                    }
                    if ui.button("Reset Game?").clicked() {
                        self.board.reset(Some((self.next_width, self.next_mines)));
                    }
                }
                ui.end_row();
                {
                    ui.label("Width/Height: ");
                    let min_width = ((self.next_mines as f32).sqrt().ceil() as usize).max(2);
                    Slider::new(&mut self.next_width, min_width..=100).logarithmic(true).ui(ui);

                    ui.label(format!("Flags Placed: {}", self.board.flags_placed()));

                }
                ui.end_row();
                {
                    ui.label("Mines: ");
                    let max_mines = self.next_width * self.next_width - 1;
                    Slider::new(&mut self.next_mines, self.next_width..=max_mines).logarithmic(true).ui(ui);

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

                    let width_to_be_used = available_space.width().min(available_space.height()) * 0.95;
                    let cell_width = width_to_be_used / self.board.get_width() as f32;

                    let flag_cell_width = cell_width * 0.5;
                    let flag_cell_delta_pos = (cell_width - flag_cell_width) / 2.0;

                    let start_x =
                        available_space.left() + (available_space.width() - width_to_be_used) / 2.0;
                    let mut start_y =
                        available_space.top() + (available_space.height() - width_to_be_used) / 2.0;

                    let mut row = 0;
                    for (index, cell) in self.board.render().into_iter().enumerate() {
                        let column = index % self.board.get_width();
                        let stroke_width = (cell_width * 0.2).max(1.0);
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
                            GridElementType::Mine => Color32::PURPLE,
                            GridElementType::Undiscovered => Color32::WHITE,
                        };

                        ui.painter().rect(
                            entire_thing_rect,
                            0.0,
                            colour,
                            Stroke::new((cell_width * 0.2).max(1.0), Color32::GRAY),
                            StrokeKind::Middle,
                        );
                        if cell.flagged {
                            let min = entire_thing_rect.min + vec2(flag_cell_delta_pos, flag_cell_delta_pos);
                            let rect = Rect {
                                min,
                                max: min + vec2(flag_cell_width, flag_cell_width),
                            };
                            ui.painter().rect_filled(rect, 0.0, Color32::BLUE);
                        }
                        if let Some(count) = cell.count {
                            ui.painter().text(
                                pos2(entire_thing_rect.min.x + cell_width / 2.0, entire_thing_rect.min.y + cell_width / 2.0),
                                Align2::CENTER_CENTER,
                                count.to_string(),
                                FontId::monospace(cell_width / 4.0 * 3.0),
                                Color32::BLACK,
                            );
                        }

                        let rsp = ui.allocate_rect({
                                                       let delta = stroke_width;
                                                       Rect {
                                                           min: entire_thing_rect.min + vec2(delta, delta),
                                                           max: entire_thing_rect.max - vec2(delta, delta),
                                                       }
                                                   }, Sense::CLICK).on_hover_cursor(CursorIcon::PointingHand);

                        let pos = (column, row);
                        if rsp.clicked() {
                            self.board.click(pos);
                        } else if rsp.secondary_clicked() {
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
