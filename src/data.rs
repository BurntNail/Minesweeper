use rand::Rng;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use crate::tile::{Tile, TileInteractionState};

#[derive(Clone)]
pub struct Data {
    pub width: usize,
    pub number_of_mines: usize,
    pub tiles: Vec<Tile>,
}

#[derive(Debug)]
pub enum DataReadError {
    UnableToParseInteger(ParseIntError),
    NotEnoughElements,
    InvalidCharacter(char),
    InvalidDataFound(InvalidDataError),
}

impl From<ParseIntError> for DataReadError {
    fn from(value: ParseIntError) -> Self {
        Self::UnableToParseInteger(value)
    }
}

impl Display for DataReadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnableToParseInteger(e) => write!(f, "Error parsing integer: {e}"),
            Self::NotEnoughElements => {
                write!(f, "Not enough elements compared to length counts provided")
            }
            Self::InvalidCharacter(ch) => {
                write!(f, "Found non-integer, non-comma character: {ch:?}")
            }
            Self::InvalidDataFound(e) => write!(f, "Read in data which broke invariants: {e}"),
        }
    }
}

impl std::error::Error for DataReadError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            Self::UnableToParseInteger(e) => Some(e),
            Self::InvalidDataFound(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum InvalidDataError {
    TooSmallWidth,
    ZeroMines,
    TooManyMines,
}

impl Display for InvalidDataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroMines => write!(f, "Found data with zero mines"),
            Self::TooSmallWidth => write!(f, "Found data 1 or less width"),
            Self::TooManyMines => write!(f, "Found data with more mines than allowed mine spaces"),
        }
    }
}

impl std::error::Error for InvalidDataError {}

impl TryFrom<String> for Data {
    type Error = DataReadError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        //i could do a big state machine, but i cba and this works well enough

        let mut accum = String::new();
        let mut chars = value.chars();

        let width: usize = loop {
            let Some(ch) = chars.next() else {
                return Err(DataReadError::NotEnoughElements);
            };

            if ch.is_ascii_digit() {
                accum.push(ch);
            } else {
                let parsed = accum.parse()?;
                accum.clear();
                break parsed;
            }
        };

        if width <= 1 {
            return Err(DataReadError::InvalidDataFound(
                InvalidDataError::TooSmallWidth,
            ));
        }

        let mut tiles = Vec::with_capacity(width * width);

        //parse numbers
        for ch in chars {
            if ch.is_ascii_digit() {
                accum.push(ch);
            } else if ch == ',' {
                let parsed = accum.parse()?;
                accum.clear();
                tiles.push(Tile::from_state(parsed));
            }
        }

        tiles.push(Tile::from_state(accum.parse()?));

        let number_of_mines = tiles.iter().filter(|x| x.is_mine()).count();
        if number_of_mines == 0 {
            return Err(DataReadError::InvalidDataFound(InvalidDataError::ZeroMines));
        } else if number_of_mines > (width * width - 1) {
            return Err(DataReadError::InvalidDataFound(InvalidDataError::TooManyMines));
        }

        Ok(Self {
            width,
            number_of_mines,
            tiles,
        })
    }
}

impl From<Data> for String {
    fn from(
        Data {
            width,
            number_of_mines: _,
            tiles,
        }: Data,
    ) -> Self {
        let mut output = width.to_string();
        output.reserve(tiles.len() * 2);
        for tile in tiles.into_iter().map(|x| x.get_state()) {
            output.push_str(&format!(",{tile}"));
        }
        output
    }
}

impl Data {
    pub const fn index_to_coords(&self, idx: usize) -> (usize, usize) {
        (idx / self.width, idx % self.width)
    }

    pub const fn coords_to_index(&self, (x, y): (usize, usize)) -> usize {
        y * self.width + x
    }

    pub fn toggle_flag(&mut self, pos: (usize, usize)) {
        let index = self.coords_to_index(pos);

        if self.tiles[index].is_flagged() || self.tiles.iter().filter(|x| x.is_flagged()).count() < self.tiles.iter().filter(|x| x.is_mine()).count() {
            self.tiles[index].toggle_flag();
        }
    }

    pub fn get_neighbours(
        &self,
        (x, y): (usize, usize),
        include_diagonals: bool,
    ) -> impl Iterator<Item = usize> + use<'_> {
        //0, 0 is top left
        let left = x.checked_sub(1);
        let horiz_middle = Some(x);
        let right = if x < self.width { Some(x + 1) } else { None };

        let above = y.checked_sub(1);
        let vert_middle = Some(y);
        let below = if y < self.width { Some(y + 1) } else { None };

        let optional = |neighbour| -> Option<(usize, usize)> {
            include_diagonals.then_some(neighbour).flatten()
        };

        left.zip(vert_middle)
            .into_iter()
            .chain(right.zip(vert_middle))
            .chain(horiz_middle.zip(above))
            .chain(horiz_middle.zip(below))
            .chain(optional(left.zip(above)))
            .chain(optional(left.zip(below)))
            .chain(optional(right.zip(above)))
            .chain(optional(right.zip(below)))
            .map(|pos| self.coords_to_index(pos))
    }

    pub fn click(&mut self, pos: (usize, usize), rng: &mut impl Rng) -> bool {
        let pos_index = self.coords_to_index(pos);

        if self.tiles.iter().all(|tile| !tile.is_mine()) {
            let mut left_to_place = self.number_of_mines;
            let mut mines_to_place = HashSet::new();

            loop {
                let new_mine_candidate = rng.random_range(0..(self.width * self.width));
                if new_mine_candidate == pos_index || mines_to_place.contains(&new_mine_candidate) {
                    continue;
                }

                mines_to_place.insert(new_mine_candidate);

                left_to_place -= 1;
                if left_to_place == 0 {
                    break;
                }
            }

            for mine in mines_to_place {
                self.tiles[mine] = Tile::new(true, TileInteractionState::Undiscovered);
            }
        }

        if self.tiles[pos_index].is_discovered() {
            return false;
        }
        self.tiles[pos_index].click();

        if self.tiles[pos_index].is_mine() {
            return true;
        }
        //TODO: remove_flag method
        if self.tiles[pos_index].is_flagged() {
            self.tiles[pos_index].toggle_flag();
        }

        let mut neighbours_to_check: Vec<_> = self.get_neighbours(pos, true).collect();
        let mut mines_to_double_check = HashSet::new();

        while let Some(neighbour) = neighbours_to_check.pop() {
            if self.tiles[neighbour].is_mine() {
                mines_to_double_check.insert(neighbour);
                continue;
            } else if self.tiles[neighbour].is_discovered() || self.tiles[neighbour].is_flagged() {
                continue;
            }

            let mut neighbours: Vec<_> = self.get_neighbours(self.index_to_coords(neighbour), true).collect();

            let mut has_a_mine_nearby = false;
            for mine in neighbours
                .iter()
                .filter(|x| self.tiles[**x].is_mine())
                .copied()
            {
                has_a_mine_nearby = true;
                mines_to_double_check.insert(mine);
            }

            if !has_a_mine_nearby {
                neighbours.retain(|candidate| {
                    !self.tiles[*candidate].is_discovered()
                        && !self.tiles[*candidate].is_flagged()
                        && !neighbours_to_check.contains(candidate)
                });
                neighbours_to_check.extend(neighbours);
            }

            self.tiles[neighbour].click();
        }

        for mine in mines_to_double_check {
            if self
                .get_neighbours(self.index_to_coords(mine), true)
                .all(|x| self.tiles[x].is_discovered())
            {
                self.tiles[mine].toggle_flag();
            }
        }

        self.game_has_been_won()
    }

    pub fn generate_counts(&self) -> Vec<u8> {
        if self.tiles.iter().all(|x| !x.is_mine()) {
            return vec![];
        }
        let mut counts = Vec::with_capacity(self.width * self.width);

        for row in 0..self.width {
            for col in 0..self.width {
                let pos = (col, row);

                let count = self
                    .get_neighbours(pos, true)
                    .filter(|pos| self.tiles[*pos].is_mine())
                    .count() as u8;
                counts.push(count);
            }
        }

        counts
    }

    pub fn game_has_been_won(&self) -> bool {
        let check_all_squares = || {
            for tile in &self.tiles {
                let is_flagged = tile.is_flagged();
                let is_mine = tile.is_mine();
                let is_discovered = tile.is_discovered();

                #[allow(clippy::nonminimal_bool)]
                if (is_flagged && !is_mine) //badly flagged mine
                    || (is_discovered && is_mine) //exploded mine
                    || (!is_discovered && !is_mine && !is_flagged)
                //undiscovered square
                {
                    return false;
                }

            }

            true
        };

        !self.game_has_been_lost() && !self.tiles.iter().all(|x| !x.is_mine()) && check_all_squares()
    }

    pub fn game_has_been_lost(&self) -> bool {
        false
        // self.mines.intersection(&self.clicked).next().is_some()
    }
}
