use rand::Rng;
use rand::prelude::IteratorRandom;
use std::collections::{HashSet, VecDeque};
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
    NotEnoughElements(usize, usize),
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
            Self::NotEnoughElements(found, ex) => {
                write!(f, "Not enough elements compared to length counts provided - expected {ex}, found {found}")
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

        let mut accum = String::new();
        let mut chars = value.chars();

        let [width, height, n_flagged, n_clicked, n_mines] = {
            let mut lengths = Vec::with_capacity(5);

            for ch in &mut chars {
                if ch.is_ascii_digit() {
                    accum.push(ch);
                } else {
                    lengths.push(accum.parse()?);
                    accum.clear();

                    if lengths.len() == 5 {
                        break;
                    }
                }
            }

            lengths.try_into().unwrap()
        };

        if width <= 1 {
            return Err(DataReadError::InvalidDataFound(
                InvalidDataError::TooSmallWidth,
            ));
        } else if n_mines == 0 {
            return Err(DataReadError::InvalidDataFound(InvalidDataError::ZeroMines));
        } else if n_mines > (width * height - 1) {
            return Err(DataReadError::InvalidDataFound(
                InvalidDataError::TooManyMines,
            ));
        }

        let mut numbers = VecDeque::with_capacity(n_flagged + n_clicked + n_mines);

        //parse numbers
        for ch in chars {
            if ch.is_ascii_digit() {
                accum.push(ch);
            } else {
                numbers.push_back(accum.parse()?);
                accum.clear();
            }
        }
        numbers.push_back(accum.parse()?);

        assert_eq!(numbers.len(), n_flagged + n_clicked + n_mines);

        let mut get_hashset = |count| {
            (0..count)
                .into_iter()
                .map(|i| numbers.pop_front().ok_or(DataReadError::NotEnoughElements(i, count)).map(|index| Data::index_to_coords(index, width)))
                .collect::<Result<_, _>>()
        };

        let flagged = get_hashset(n_flagged)?;
        let clicked = get_hashset(n_clicked)?;
        let mines = get_hashset(n_mines)?;

        assert!(numbers.is_empty());

        Ok(Self {
            width,
            height,
            number_of_mines: n_mines,
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
        let numbers: Vec<_> = [width, height, flagged.len(), clicked.len(), mines.len()]
            .into_iter()
            .chain(
                flagged.into_iter().chain(clicked).chain(mines)
                    .map(|pos| Data::coords_to_index(pos, width))
            )
            .map(|x| x.to_string())
            .collect();
        numbers.join(",")
    }
}

impl Data {
    #[inline]
    pub const fn index_to_coords(idx: usize, width: usize) -> (usize, usize) {
        (idx % width, idx / width)
    }

    #[inline]
    pub const fn coords_to_index((x, y): (usize, usize), width: usize) -> usize {
        y * width + x
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
                    .map(|x| Data::index_to_coords(x, self.width))
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

        //TODO: DRY on checking
        let mut mine_is_next_to_chosen = false;
        for current_ntc in &neighbours_to_check {
            if self.mines.contains(current_ntc) {
                mines_to_double_check.insert(*current_ntc);
                mine_is_next_to_chosen = true;
            }
        }

        //don't expand if we're next to a mine
        if mine_is_next_to_chosen {
            neighbours_to_check.clear();
        }

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
            (0..self.height)
                .flat_map(|y| (0..self.width).map(move |x| (x, y)))
                .all(|pos| {
                    let is_mine = self.mines.contains(&pos);
                    let is_discovered = self.clicked.contains(&pos);

                    is_discovered && !is_mine
                     || !is_discovered && is_mine
                })
        };

        //use a closure to allow short-circuiting
        !self.game_has_been_lost() && !self.mines.is_empty() && check_all_squares()
    }

    pub fn game_has_been_lost(&self) -> bool {
        //because iterators are lazy, this method isn't quite as bad as it looks
        self.mines.intersection(&self.clicked).next().is_some()
    }
}
