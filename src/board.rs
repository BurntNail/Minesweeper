use crate::data::{Data, InvalidDataError};
use rand::rngs::ThreadRng;
use std::collections::HashSet;
use std::default::Default;
use crate::tile::{Tile, TileInteractionState};

pub struct Board {
    has_given_up: bool,
    data: Data,
    rng: ThreadRng,
}

impl Data {
    pub fn new(width: usize, number_of_mines: usize) -> Self {
        Self {
            width,
            number_of_mines,
            tiles: vec![Tile::new(false, TileInteractionState::Undiscovered); width * width],
        }
    }
}

impl TryFrom<Data> for Board {
    type Error = InvalidDataError;

    fn try_from(data: Data) -> Result<Self, Self::Error> {
        if data.width <= 1 {
            return Err(InvalidDataError::TooSmallWidth);
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
    pub fn new(width: usize, number_of_mines: usize) -> Result<Self, InvalidDataError> {
        if width <= 1 {
            return Err(InvalidDataError::TooSmallWidth);
        } else if number_of_mines == 0 {
            return Err(InvalidDataError::ZeroMines);
        } else if number_of_mines > (width * width - 1) {
            return Err(InvalidDataError::TooManyMines);
        }

        Ok(Self {
            has_given_up: false,
            data: Data::new(width, number_of_mines),
            rng: ThreadRng::default(),
        })
    }

    pub fn from_previous_data(data: Data) -> Result<Self, InvalidDataError> {
        Self::try_from(data)
    }

    pub fn reset(&mut self, new_mines_width: Option<(usize, usize)>) {
        self.has_given_up = false;

        let (new_width, new_mines) =
            new_mines_width.unwrap_or((self.data.width, self.data.number_of_mines));
        self.data = Data::new(new_width, new_mines);
    }

    pub const fn get_width(&self) -> usize {
        self.data.width
    }

    pub const fn total_mines(&self) -> usize {
        self.data.number_of_mines
    }

    pub fn flags_placed(&self) -> usize {
        0
        // self.data.flagged.len()
    }

    pub fn successfully_flagged(&self) -> usize {
        0
        // self.data.flagged.intersection(&self.data.mines).count()
    }

    pub fn give_up(&mut self) {
        self.has_given_up = true;
    }

    pub const fn get_data(&self) -> &Data {
        &self.data
    }

    pub fn toggle_flag(&mut self, pos: (usize, usize)) {
        if self.game_has_been_won() || self.game_has_been_lost()
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
        self.data.tiles.iter().enumerate().map(|(i, tile)| {
            let ty = if tile.is_mine() && tile.is_flagged() {
                GridElementType::Exploded
            } else if tile.is_mine() && (self.game_has_been_lost() || self.game_has_been_won()) {
                GridElementType::Mine
            } else if tile.is_discovered() {
                GridElementType::Discovered
            } else {
                GridElementType::Undiscovered
            };

            let should_display_count = ty == GridElementType::Discovered
                && self
                .data
                .get_neighbours(self.data.index_to_coords(i), true)
                .any(|neighbour| !self.data.tiles[neighbour].is_discovered());

            RenderedGridElement {
                ty,
                flagged: tile.is_flagged(),
                should_display_count,
            }
        }).collect()
    }

    pub fn game_has_been_won(&self) -> bool {
        !self.game_has_been_lost() && self.data.game_has_been_won()
    }

    pub fn game_has_been_lost(&self) -> bool {
        self.has_given_up || self.data.game_has_been_lost()
    }

    pub fn generate_counts(&self) -> Vec<u8> {
        self.data.generate_counts()
    }
}

pub struct RenderedGridElement {
    pub ty: GridElementType,
    pub flagged: bool,
    pub should_display_count: bool,
}

#[derive(Eq, PartialEq)]
pub enum GridElementType {
    Exploded,
    Discovered,
    Undiscovered,
    Mine,
}
