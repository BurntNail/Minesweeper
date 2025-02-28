use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use rand::Rng;

#[derive(Clone)]
pub struct Data {
    pub width: usize,
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
    InvalidDataFound,
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
            Self::InvalidDataFound => write!(f, "Read in data which broke invariants"),
        }
    }
}

impl std::error::Error for DataReadError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        if let Self::UnableToParseInteger(e) = &self {
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
                if i == 3 {
                    break;
                }

                i += 1;
            }
        }

        let [width, n_flagged, n_clicked, number_of_mines] = lengths;
        if width == 0 || number_of_mines == 0 || number_of_mines > (width * width - 1) {
            return Err(DataReadError::InvalidDataFound);
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
            number_of_mines: _,
            flagged,
            clicked,
            mines,
        }: Data,
    ) -> Self {
        let mut output = format!(
            "{width},{},{},{}",
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
        (idx / self.width, idx % self.width)
    }

    #[allow(dead_code)]
    const fn coords_to_index(&self, (x, y): (usize, usize)) -> usize {
        y * self.width + x
    }

    pub fn toggle_flag (&mut self, pos: (usize, usize)) {
        if self.flagged.contains(&pos) {
            self.flagged.remove(&pos);
        } else if self.flagged.len() < self.mines.len() {
            self.flagged.insert(pos);
        }
    }

    pub fn get_neighbours((x, y): (usize, usize), size: usize, include_diagonals: bool) -> impl Iterator<Item = (usize, usize)> {
        //0, 0 is top left
        let left = x.checked_sub(1);
        let horiz_middle = Some(x);
        let right = if x < size { Some(x + 1) } else { None };

        let above = y.checked_sub(1);
        let vert_middle = Some(y);
        let below = if y < size { Some(y + 1) } else { None };

        let optional = |neighbour| -> Option<(usize, usize)> {
            include_diagonals.then_some(neighbour).flatten()
        };


        left.zip(vert_middle)
            .into_iter()
            .chain(left.zip(vert_middle))
            .chain(right.zip(vert_middle))
            .chain(horiz_middle.zip(above))
            .chain(horiz_middle.zip(below))
            .chain(optional(left.zip(above)))
            .chain(optional(left.zip(below)))
            .chain(optional(right.zip(above)))
            .chain(optional(right.zip(below)))
    }

    pub fn click (&mut self, pos: (usize, usize), rng: &mut impl Rng) -> bool {
        if self.mines.is_empty() {
            let mut left_to_place = self.number_of_mines;

            loop {
                let new_mine_candidate = rng
                    .random_range(0..(self.width * self.width));
                let new_mine_candidate = self.index_to_coords(new_mine_candidate);
                if new_mine_candidate == pos || self.mines.contains(&new_mine_candidate) {
                    continue;
                }

                self
                    .mines
                    .insert(new_mine_candidate);

                left_to_place -= 1;
                if left_to_place == 0 {
                    break;
                }
            }
        }

        if self.clicked.contains(&pos) {
            return false;
        }
        self.clicked.insert(pos);

        if self.mines.contains(&pos) {
            return true;
        }
        self.flagged.remove(&pos);

        let mut to_be_checked: Vec<_> = Self::get_neighbours(pos, self.width, true).collect();

        while let Some(candidate) = to_be_checked.pop() {
            if self.mines.contains(&candidate) //can't click on a mine lol
                || self.clicked.contains(&candidate) //can't re-click
                || self.flagged.contains(&candidate) //shouldn't click on a flagged one
            {
                continue;
            }

            let neighbour_count = Self::get_neighbours(candidate, self.width, true).filter(|x| self.mines.contains(x)).count();

            match neighbour_count {
                0 => {
                    //if it has zero neighbours, simulate a 'click'
                    self.click(candidate, rng);
                },
                _ => {
                    //if not, then just mark it as discovered
                    self.clicked.insert(candidate);
                }
            }

        }

        false
    }
}