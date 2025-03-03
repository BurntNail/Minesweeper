use rand::Rng;
use rand::prelude::IteratorRandom;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;

#[derive(Clone)]
pub struct Data {
    pub width: usize,
    pub height: usize,
    pub number_of_mines: usize,
    pub flagged: HashSet<(usize, usize)>,
    pub clicked: HashSet<(usize, usize)>,
    pub mines: HashSet<(usize, usize)>,
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
    TooSmallHeight,
    ZeroMines,
    TooManyMines,
}

impl Display for InvalidDataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroMines => write!(f, "Found data with zero mines"),
            Self::TooSmallWidth => write!(f, "Found data 1 or less width"),
            Self::TooSmallHeight => write!(f, "Found data 1 or less height"),
            Self::TooManyMines => write!(f, "Found data with more mines than allowed mine spaces"),
        }
    }
}

impl std::error::Error for InvalidDataError {}

impl TryFrom<String> for Data {
    type Error = DataReadError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        //i could do a big state machine, but i cba and this works well enough
        let mut lengths = [0; 5];
        let mut numbers = vec![];

        let mut accum = String::new();
        let mut chars = value.chars();

        //parse lengths
        let mut i = 0;
        for ch in &mut chars {
            if ch.is_ascii_digit() {
                accum.push(ch);
            } else {
                let parsed = accum.parse()?;
                accum.clear();

                lengths[i] = parsed;
                if i == lengths.len() - 1 {
                    break;
                }

                i += 1;
            }
        }

        let [width, height, n_flagged, n_clicked, number_of_mines] = lengths;
        if width <= 1 {
            return Err(DataReadError::InvalidDataFound(
                InvalidDataError::TooSmallWidth,
            ));
        } else if number_of_mines == 0 {
            return Err(DataReadError::InvalidDataFound(InvalidDataError::ZeroMines));
        } else if number_of_mines > (width * height - 1) {
            return Err(DataReadError::InvalidDataFound(
                InvalidDataError::TooManyMines,
            ));
        }

        numbers.reserve(n_flagged + n_clicked + number_of_mines);

        //parse numbers
        for ch in chars {
            if ch.is_ascii_digit() {
                accum.push(ch);
            } else if ch == ',' {
                let parsed = accum.parse()?;
                accum.clear();
                numbers.push(parsed);
            }
        }
        numbers.push(accum.parse()?);

        let mut get_hashset = |count| {
            let mut set = HashSet::new();
            for _ in 0..count {
                let Some(y) = numbers.pop() else {
                    return Err(DataReadError::NotEnoughElements);
                };
                let Some(x) = numbers.pop() else {
                    return Err(DataReadError::NotEnoughElements);
                };

                set.insert((x, y));
            }

            Ok(set)
        };

        let mines = get_hashset(number_of_mines)?;
        let clicked = get_hashset(n_clicked)?;
        let flagged = get_hashset(n_flagged)?;

        Ok(Self {
            width,
            height,
            number_of_mines,
            flagged,
            clicked,
            mines,
        })
    }
}

impl From<Data> for String {
    fn from(
        Data {
            width,
            height,
            number_of_mines: _,
            flagged,
            clicked,
            mines,
        }: Data,
    ) -> Self {
        let mut output = format!(
            "{width},{height},{},{},{}",
            flagged.len(),
            clicked.len(),
            mines.len()
        );
        for (x, y) in flagged.into_iter().chain(clicked).chain(mines.into_iter()) {
            output.push_str(&format!(",{x},{y}"));
        }
        output
    }
}

impl Data {
    const fn index_to_coords(&self, idx: usize) -> (usize, usize) {
        (idx % self.width, idx / self.width)
    }

    #[allow(dead_code)]
    const fn coords_to_index(&self, (x, y): (usize, usize)) -> usize {
        y * self.width + x
    }

    pub fn toggle_flag(&mut self, pos: (usize, usize)) {
        if self.flagged.contains(&pos) {
            self.flagged.remove(&pos);
        } else if self.flagged.len() < self.mines.len() {
            self.flagged.insert(pos);
        }
    }

    pub fn get_neighbours(
        &self,
        (x, y): (usize, usize),
        include_diagonals: bool,
    ) -> impl Iterator<Item = (usize, usize)> + use<> {
        //0, 0 is top left
        let left = x.checked_sub(1);
        let horiz_middle = Some(x);
        let right = if x < self.width { Some(x + 1) } else { None };

        let above = y.checked_sub(1);
        let vert_middle = Some(y);
        let below = if y < self.height { Some(y + 1) } else { None };

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
    }

    pub fn click(&mut self, pos: (usize, usize), rng: &mut impl Rng) -> bool {
        //if mines are empty, we need to add more mines!
        if self.mines.is_empty() {
            self.mines.extend(
                (0..(self.width * self.height))
                    .map(|x| self.index_to_coords(x))
                    .filter(|x| *x != pos)
                    .choose_multiple(rng, self.number_of_mines),
            );
        }

        //if we've already clicked it, skip out
        if self.clicked.contains(&pos) {
            return false;
        }
        //'click' it and unflag it
        self.clicked.insert(pos);
        self.flagged.remove(&pos);

        //if it's a mine, game over
        if self.mines.contains(&pos) {
            return true;
        }

        //make a list of potential squares to check - we're collecting it here to be able to add to it later
        let mut neighbours_to_check: Vec<_> = self.get_neighbours(pos, true).collect();
        //we're also building up a list of mines to check for auto-flagging
        let mut mines_to_double_check = HashSet::new();

        while let Some(neighbour) = neighbours_to_check.pop() {
            //if this square is a mine, add it to the tobechecked list and continue to the next element
            if self.mines.contains(&neighbour) {
                mines_to_double_check.insert(neighbour);
                continue;
                //if this square is already clicked or flagged, skip it!
            } else if self.clicked.contains(&neighbour) || self.flagged.contains(&neighbour) {
                continue;
            }

            //since this cell is adjacent and not clicked/flagged/mine, 'click' it
            self.clicked.insert(neighbour);

            //get the neighbours of this cell
            let mut neighbours: Vec<_> = self.get_neighbours(neighbour, true).collect();

            //go through the neighbours, and if there are any mines, add them to the list
            let mut has_a_mine_nearby = false;
            for mine in neighbours
                .iter()
                .filter(|x| self.mines.contains(x))
                .copied()
            {
                has_a_mine_nearby = true;
                mines_to_double_check.insert(mine);
            }

            //if there isn't a mine next to this cell, add the non-flagged non-clicked neighbours to the check list to expand the selection. this is vulnerable to duplicates, but this seems efficient enough rn
            //TODO: check if it's worth adding a 'checked' list to avoid further duplicates
            if !has_a_mine_nearby {
                neighbours.retain(|candidate| {
                    !self.clicked.contains(candidate)
                        && !self.flagged.contains(candidate)
                        && !neighbours_to_check.contains(candidate)
                });
                neighbours_to_check.extend(neighbours);
            }
        }

        //for all the mines that were neighbours of cells
        for mine in mines_to_double_check {
            //if we've clicked all of the adjacent cells
            if self
                .get_neighbours(mine, true)
                .all(|x| self.clicked.contains(&x))
            {
                //auto-flag that mine
                self.flagged.insert(mine);
            }
        }

        //we already checked if this click would lose the game, so we now only need to check if this click won the game
        self.game_has_been_won()
    }

    pub fn generate_counts(&self) -> Option<Vec<u8>> {
        if self.mines.is_empty() {
            return None;
        }
        let mut counts = Vec::with_capacity(self.width * self.height);

        for y in 0..self.height {
            for x in 0..self.width {
                let pos = (x, y);

                let count = self
                    .get_neighbours(pos, true)
                    .filter(|pos| self.mines.contains(pos))
                    .count() as u8;
                counts.push(count);
            }
        }

        Some(counts)
    }

    pub fn game_has_been_won(&self) -> bool {
        let check_all_squares = || {
            for y in 0..self.height {
                for x in 0..self.width {
                    let pos = (x, y);

                    let is_flagged = self.flagged.contains(&pos);
                    let is_mine = self.mines.contains(&pos);
                    let is_discovered = self.clicked.contains(&pos);

                    #[allow(clippy::nonminimal_bool)]
                    if (is_flagged && !is_mine) //badly flagged mine
                        || (is_discovered && is_mine) //exploded mine
                        || (!is_discovered && !is_mine && !is_flagged)
                    //undiscovered square
                    {
                        return false;
                    }
                }
            }

            true
        };

        //use a closure to allow short-circuiting
        !self.game_has_been_lost() && !self.mines.is_empty() && check_all_squares()
    }

    pub fn game_has_been_lost(&self) -> bool {
        //because iterators are lazy, this method isn't quite as bad as it looks
        self.mines.intersection(&self.clicked).next().is_some()
    }
}
