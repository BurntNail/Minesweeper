use crate::data::Data;
use crate::ser::InvalidDataError;
use egui::{Rect, pos2};
use fastrand::Rng;
use std::default::Default;

pub struct Board {
    ///Has the player chosen to give up?
    has_given_up: bool,
    ///The current board data
    data: Data,
    ///The RNG used for random number generation
    rng: Rng,
}

impl TryFrom<Data> for Board {
    type Error = InvalidDataError;

    fn try_from(data: Data) -> Result<Self, Self::Error> {
        //check various invariants for creating from previous data
        if data.width <= 1 {
            return Err(InvalidDataError::TooSmallWidth);
        } else if data.height <= 1 {
            return Err(InvalidDataError::TooSmallHeight);
        } else if data.number_of_mines == 0 {
            return Err(InvalidDataError::ZeroMines);
        } else if data.number_of_mines > (data.width * data.width - 1) {
            return Err(InvalidDataError::TooManyMines);
        }

        Ok(Self {
            has_given_up: false,
            data,
            rng: Rng::default(),
        })
    }
}

impl Board {
    pub fn new(
        width: usize,
        height: usize,
        number_of_mines: usize,
    ) -> Result<Self, InvalidDataError> {
        Self::try_from(Data::new_blank(width, height, number_of_mines))
    }

    pub fn from_previous_data(data: Data) -> Result<Self, InvalidDataError> {
        Self::try_from(data)
    }

    pub fn reset(&mut self, new_width_height_mines: Option<(usize, usize, usize)>) {
        self.has_given_up = false;

        let (new_width, new_height, new_mines) = new_width_height_mines.unwrap_or((
            self.data.width,
            self.data.height,
            self.data.number_of_mines,
        ));
        self.data = Data::new_blank(new_width, new_height, new_mines);
    }

    pub const fn get_width(&self) -> usize {
        self.data.width
    }
    pub const fn get_height(&self) -> usize {
        self.data.height
    }

    pub const fn total_mines(&self) -> usize {
        self.data.number_of_mines
    }

    pub fn total_uninteracted(&self) -> usize {
        self.data.total_uninteracted::<true>()
    }

    pub fn flags_placed(&self) -> usize {
        self.data.flagged.len()
    }

    pub fn successfully_flagged(&self) -> usize {
        self.data.flagged.intersection(&self.data.mines).count()
    }

    pub fn give_up(&mut self) {
        self.has_given_up = true;
    }

    pub const fn get_data(&self) -> &Data {
        &self.data
    }

    ///returns whether the game is over
    pub fn toggle_flag(&mut self, pos: (usize, usize)) -> bool {
        if self.game_is_over() {
            //ensure can't flag when game over
            return true;
        }
        self.data.toggle_flag(pos)
    }

    ///returns whether the game is over
    pub fn click(&mut self, pos: (usize, usize)) -> bool {
        if self.game_is_over() {
            //ensure can't click when game over
            return true;
        }

        self.data.click(pos, &mut self.rng)
    }

    pub fn render(&self) -> Vec<RenderedGridElement> {
        let game_is_over = self.game_is_over();

        //for each column
        (0..self.data.height)
            //and each row
            .flat_map(|y| (0..self.data.width).map(move |x| (x, y)))
            .map(|pos| {
                //work out the type, based off of various factors
                let ty = if self.data.mines.contains(&pos) && self.data.clicked.contains(&pos) {
                    GridElementType::Exploded
                } else if self.data.mines.contains(&pos) && game_is_over {
                    GridElementType::Mine
                } else if self.data.clicked.contains(&pos) {
                    GridElementType::Discovered {
                        should_display_count: self
                            .data
                            .get_neighbours(pos, true)
                            .any(|neighbour| !self.data.clicked.contains(&neighbour)),
                    }
                } else {
                    GridElementType::Undiscovered
                };

                //and return the rendered grid element
                RenderedGridElement {
                    ty,
                    flagged: self.data.flagged.contains(&pos),
                }
            })
            .collect()
    }

    pub fn game_has_been_won(&self) -> bool {
        self.data.game_has_been_won()
    }

    pub fn game_has_been_lost(&self) -> bool {
        self.has_given_up || self.data.game_has_been_lost()
    }

    pub fn game_is_over(&self) -> bool {
        self.has_given_up || self.data.game_is_over()
    }

    pub fn generate_counts(&self) -> Option<Vec<u8>> {
        self.data.generate_counts()
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SpriteAtlas {
    WinMine
}

impl SpriteAtlas {
    pub fn get_png_bytes (self) -> &'static [u8] {
        match self {
            SpriteAtlas::WinMine => include_bytes!("../WinmineXP.png")
        }
    }
}

///A grid element that has been rendered - to display, use the [`RenderedGridElement::to_uv`] method
#[derive(Copy, Clone, Debug)]
pub struct RenderedGridElement {
    ty: GridElementType,
    flagged: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum GridElementType {
    Exploded,
    Discovered { should_display_count: bool },
    Undiscovered,
    Mine,
}

impl RenderedGridElement {
    pub fn to_uv(self, count: u8, game_is_over: bool, sprite_atlas: SpriteAtlas) -> Rect {
        let rect = |x, y| {
            let (x, y) = (x as f32, y as f32);
            Rect {
                min: pos2(0.25 * x, 0.25 * y),
                max: pos2(0.25 * (x + 1.0), 0.25 * (y + 1.0)),
            }
        };

        match sprite_atlas {
            SpriteAtlas::WinMine => {
                if self.flagged {
                    return if game_is_over && self.ty != GridElementType::Mine {
                        rect(3, 2)
                    } else {
                        rect(2, 2)
                    };
                }

                match self.ty {
                    GridElementType::Exploded => rect(3, 3),
                    GridElementType::Undiscovered => rect(1, 2),
                    GridElementType::Discovered {
                        should_display_count,
                    } => {
                        if should_display_count {
                            match count {
                                1 => rect(0, 0),
                                2 => rect(1, 0),
                                3 => rect(2, 0),
                                4 => rect(3, 0),
                                5 => rect(0, 1),
                                6 => rect(1, 1),
                                7 => rect(2, 1),
                                8 => rect(3, 1),
                                _ => rect(0, 2),
                            }
                        } else {
                            rect(0, 2)
                        }
                    }
                    GridElementType::Mine => rect(2, 3),
                }
            }
        }
    }
}
