use rand::rngs::ThreadRng;
use std::collections::HashSet;
use std::default::Default;
use std::ops::BitXor;
use crate::data::Data;

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
            flagged: HashSet::new(),
            clicked: HashSet::new(),
            mines: HashSet::new(),
        }
    }
}

impl Board {
    pub fn new(width: usize, number_of_mines: usize) -> Option<Self> {
        if width <= 1 || number_of_mines == 0 || number_of_mines > (width * width) {
            return None;
        }

        Some(Self {
            has_given_up: false,
            data: Data::new(width, number_of_mines),
            rng: ThreadRng::default(),
        })
    }

    pub fn from_previous_data(data: Data) -> Self {
        Self {
            has_given_up: false,
            data,
            rng: ThreadRng::default(),
        }
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
        if self.game_has_been_won() || self.game_has_been_lost() || self.data.clicked.contains(&pos) {
            return;
        }

        self.data.toggle_flag(pos);
    }

    ///returns whether game over has occured
    pub fn click(&mut self, pos: (usize, usize)) -> bool {
        if self.game_has_been_won() || self.game_has_been_lost(){
            return true;
        }

        self.data.click(pos, &mut self.rng)
    }

    pub fn render(&self) -> Vec<RenderedGridElement> {
        let mut grid = Vec::with_capacity(self.data.width * self.data.width);

        for y in 0..self.data.width {
            for x in 0..self.data.width {
                let pos = (x, y);
                let ty = if self.data.mines.contains(&pos) && self.data.clicked.contains(&pos)
                {
                    GridElementType::Exploded
                } else if self.data.mines.contains(&pos) && (self.game_has_been_lost() || self.game_has_been_won()) {
                    GridElementType::Mine
                } else if self.data.clicked.contains(&pos) {
                    GridElementType::Discovered
                } else {
                    GridElementType::Undiscovered
                };

                let count = if ty == GridElementType::Discovered {
                    let mut count = 0;
                    let mut neighbour_was_clicked = true;

                    for neighbour in Data::get_neighbours(pos, self.data.width, true) {
                        if self.data.mines.contains(&neighbour) {
                            count += 1;
                        }

                        neighbour_was_clicked &= self.data.clicked.contains(&neighbour);
                    }

                    (!neighbour_was_clicked && count > 0).then_some(count)
                } else {
                    None
                };

                grid.push(RenderedGridElement {
                    ty,
                    flagged: self.data.flagged.contains(&pos),
                    count,
                });
            }
        }

        grid
    }

    pub fn game_has_been_won(&self) -> bool {
        let check_all_squares = || {
            for x in 0..self.data.width {
                for y in 0..self.data.width {
                    let pos = (x, y);

                    let is_flagged_mine = self.data.mines.contains(&pos) && self.data.flagged.contains(&pos);
                    let is_discovered = self.data.clicked.contains(&pos);

                    if !is_flagged_mine.bitxor(is_discovered) {
                        return false;
                    }
                }
            }

            true
        };

        !self.game_has_been_lost()
            && !self.data.mines.is_empty()
            && check_all_squares()
    }

    pub fn game_has_been_lost(&self) -> bool {
        self.has_given_up || self.data.mines.intersection(&self.data.clicked).next().is_some()
    }
}

pub struct RenderedGridElement {
    pub ty: GridElementType,
    pub flagged: bool,
    pub count: Option<u8>,
}

#[derive(Eq, PartialEq)]
pub enum GridElementType {
    Exploded,
    Discovered,
    Undiscovered,
    Mine,
}
