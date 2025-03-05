use crate::data::{Data, InvalidDataError};
use rand::rngs::ThreadRng;
use std::collections::HashSet;
use std::default::Default;
use egui::{pos2, Rect};

pub struct Board {
    has_given_up: bool,
    data: Data,
    rng: ThreadRng,
}

impl Data {
    pub fn new(width: usize, height: usize, number_of_mines: usize) -> Self {
        Self {
            width,
            height,
            number_of_mines,
            flagged: HashSet::new(),
            clicked: HashSet::new(),
            mines: HashSet::new(),
        }
    }
}

impl TryFrom<Data> for Board {
    type Error = InvalidDataError;

    fn try_from(data: Data) -> Result<Self, Self::Error> {
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
            rng: ThreadRng::default(),
        })
    }
}

impl Board {
    pub fn new(
        width: usize,
        height: usize,
        number_of_mines: usize,
    ) -> Result<Self, InvalidDataError> {
        if width <= 1 {
            return Err(InvalidDataError::TooSmallWidth);
        } else if number_of_mines == 0 {
            return Err(InvalidDataError::ZeroMines);
        } else if number_of_mines > (width * height - 1) {
            return Err(InvalidDataError::TooManyMines);
        }

        Ok(Self {
            has_given_up: false,
            data: Data::new(width, height, number_of_mines),
            rng: ThreadRng::default(),
        })
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
        self.data = Data::new(new_width, new_height, new_mines);
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

    pub fn toggle_flag(&mut self, pos: (usize, usize)) {
        if self.game_has_been_won() || self.game_has_been_lost() || self.data.clicked.contains(&pos)
        {
            return;
        }
        self.data.toggle_flag(pos);
    }

    ///returns whether game over has occured
    pub fn click(&mut self, pos: (usize, usize)) -> bool {
        if self.game_has_been_won() || self.game_has_been_lost() {
            return true;
        }

        self.data.click(pos, &mut self.rng)
    }

    pub fn render(&self) -> Vec<RenderedGridElement> {
        let mut grid = Vec::with_capacity(self.data.width * self.data.height);

        for y in 0..self.data.height {
            for x in 0..self.data.width {
                let pos = (x, y);
                let ty = if self.data.mines.contains(&pos) && self.data.clicked.contains(&pos) {
                    GridElementType::Exploded
                } else if self.data.mines.contains(&pos)
                    && (self.game_has_been_lost() || self.game_has_been_won())
                {
                    GridElementType::Mine
                } else if self.data.clicked.contains(&pos) {
                    GridElementType::Discovered
                } else {
                    GridElementType::Undiscovered
                };

                let should_display_count = ty == GridElementType::Discovered
                    && self
                        .data
                        .get_neighbours(pos, true)
                        .any(|neighbour| !self.data.clicked.contains(&neighbour));

                grid.push(RenderedGridElement {
                    ty,
                    flagged: self.data.flagged.contains(&pos),
                    should_display_count,
                });
            }
        }

        grid
    }

    pub fn game_has_been_won(&self) -> bool {
        !self.game_has_been_lost() && self.data.game_has_been_won()
    }

    pub fn game_has_been_lost(&self) -> bool {
        self.has_given_up || self.data.game_has_been_lost()
    }

    pub fn generate_counts(&self) -> Option<Vec<u8>> {
        self.data.generate_counts()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RenderedGridElement {
    pub ty: GridElementType,
    pub flagged: bool,
    pub should_display_count: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GridElementType {
    Exploded,
    Discovered,
    Undiscovered,
    Mine,
}

impl RenderedGridElement {
    pub fn to_uv (self, count: u8, game_is_over: bool) -> Rect {
        let rect = |x, y| {
            let (x, y) = (x as f32, y as f32);
            Rect {
                min: pos2(0.25 * x, 0.25 * y),
                max: pos2(0.25 * (x + 1.0), 0.25 * (y + 1.0))
            }
        };

        if self.should_display_count {
            return match count {
                1 => rect(0, 0),
                2 => rect(1, 0),
                3 => rect(2, 0),
                4 => rect(3, 0),
                5 => rect(0, 1),
                6 => rect(1, 1),
                7 => rect(2, 1),
                8 => rect(3, 1),
                _ => rect(0, 2)
            };
        }

        if self.flagged {
            return if game_is_over && self.ty != GridElementType::Mine {
                rect(3, 2)
            } else {
                rect(2, 2)
            };
        }

        match self.ty {
            GridElementType::Exploded => rect(3, 3),
            GridElementType::Discovered => rect(0, 2),
            GridElementType::Undiscovered => rect(1, 2),
            GridElementType::Mine => rect(2, 3)
        }
    }
}