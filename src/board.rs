use rand::Rng;
use rand::rngs::ThreadRng;
use std::collections::HashSet;

pub struct Board {
    width: usize,
    number_of_mines: usize,
    losing_mine_clicked_on: Option<(usize, usize)>,
    has_given_up: bool,
    flagged: HashSet<(usize, usize)>,
    clicked: HashSet<(usize, usize)>,
    mines: HashSet<(usize, usize)>,
    rng: ThreadRng,
}

impl Board {
    pub fn new(width: usize, number_of_mines: usize) -> Option<Self> {
        if width <= 1 || number_of_mines == 0 || number_of_mines > (width * width) {
            return None;
        }

        Some(Self {
            width,
            number_of_mines,
            losing_mine_clicked_on: None,
            has_given_up: false,
            flagged: HashSet::new(),
            mines: HashSet::new(),
            clicked: HashSet::new(),
            rng: ThreadRng::default(),
        })
    }

    pub fn reset(&mut self, new_mines: Option<usize>, new_width: Option<usize>) {
        self.flagged = HashSet::new();
        self.mines = HashSet::new();
        self.clicked = HashSet::new();
        self.losing_mine_clicked_on = None;
        self.has_given_up = false;

        if let Some(new_mines) = new_mines {
            self.number_of_mines = new_mines;
        }
        if let Some(new_width) = new_width {
            self.width = new_width;
        }
    }

    pub const fn get_width_height(&self) -> usize {
        self.width
    }

    pub const fn total_mines(&self) -> usize {
        self.number_of_mines
    }

    pub fn flags_placed(&self) -> usize {
        self.flagged.len()
    }

    pub fn successfully_flagged (&self) -> usize {
        self.flagged.intersection(&self.mines).count()
    }

    pub fn give_up(&mut self) {
        self.has_given_up = true;
    }

    const fn index_to_coords(&self, idx: usize) -> (usize, usize) {
        (idx / self.width, idx % self.width)
    }

    const fn coords_to_index(&self, (x, y): (usize, usize)) -> usize {
        y * self.width + x
    }

    pub fn toggle_flag(&mut self, pos: (usize, usize)) {
        if self.game_has_been_won() || self.game_has_been_lost() {
            return;
        }

        if self.flagged.contains(&pos) {
            self.flagged.remove(&pos);
        } else if self.flagged.len() < self.mines.len(){
            self.flagged.insert(pos);
        }
    }

    fn get_neighbours((x, y): (usize, usize), size: usize) -> impl Iterator<Item = (usize, usize)> {
        //0, 0 is top left
        let left = x.checked_sub(1);
        let horiz_middle = Some(x);
        let right = if x < size { Some(x + 1) } else { None };

        let above = y.checked_sub(1);
        let vert_middle = Some(y);
        let below = if y < size { Some(y + 1) } else { None };

        left.zip(above)
            .into_iter()
            .chain(left.zip(vert_middle))
            .chain(left.zip(below))
            .chain(horiz_middle.zip(above))
            .chain(horiz_middle.zip(below))
            .chain(right.zip(above))
            .chain(right.zip(vert_middle))
            .chain(right.zip(below))
    }

    ///returns whether or not game over has occured
    pub fn click(&mut self, pos: (usize, usize)) -> bool {
        if self.game_has_been_won() || self.game_has_been_lost() {
            return true;
        }

        self.clicked.insert(pos);
        let clicked_idx = self.coords_to_index(pos);
        if self.mines.is_empty() {
            let mut left_to_rx = self.number_of_mines;

            loop {
                let new_mine_candidate = self.rng.random_range(0..(self.width * self.width));
                if new_mine_candidate == clicked_idx {
                    continue;
                }

                self.mines.insert(self.index_to_coords(new_mine_candidate));

                left_to_rx -= 1;
                if left_to_rx == 0 {
                    break;
                }
            }

            return false;
        }

        if self.mines.contains(&pos) {
            self.losing_mine_clicked_on = Some(pos);
        } else {
            let mut to_be_checked: Vec<_> = Self::get_neighbours(pos, self.width).collect();
            let mut to_be_ignored = HashSet::new();

            while let Some(neighbour) = to_be_checked.pop() {
                if self.mines.contains(&neighbour)
                    || self.clicked.contains(&neighbour)
                    || self.flagged.contains(&neighbour)
                    || to_be_ignored.contains(&neighbour)
                {
                    continue;
                }

                let neighbours_neighbours: Vec<_> =
                    Self::get_neighbours(neighbour, self.width).collect();
                let neighbours_neighbour_count = neighbours_neighbours
                    .iter()
                    .filter(|x| self.mines.contains(x))
                    .count() as u8;

                if neighbours_neighbour_count > 0 {
                    to_be_ignored.insert(neighbour);
                    continue;
                }

                to_be_checked.extend(neighbours_neighbours);
                self.click(neighbour);
            }
        }

        self.losing_mine_clicked_on.is_some()
    }

    pub fn render(&self) -> Vec<RenderedGridElement> {
        let mut grid = Vec::with_capacity(self.width * self.width);

        for y in 0..self.width {
            for x in 0..self.width {
                let pos = (x, y);
                let ty = if self
                    .losing_mine_clicked_on
                    .is_some_and(|whoops| whoops == pos)
                {
                    GridElementType::Exploded
                } else if self.mines.contains(&pos) && self.game_has_been_lost() {
                    GridElementType::Mine
                } else if self.clicked.contains(&pos) {
                    GridElementType::Discovered
                } else {
                    GridElementType::Undiscovered
                };

                let count = if ty == GridElementType::Undiscovered && !self.mines.contains(&pos)
                {
                    let mut count = 0;
                    let mut neighbour_was_clicked = false;

                    for ctc in Self::get_neighbours(pos, self.width) {
                        if self.mines.contains(&ctc) {
                            count += 1;
                        }

                        neighbour_was_clicked |= self.clicked.contains(&ctc);
                    }
                    neighbour_was_clicked &= ty != GridElementType::Discovered;

                    neighbour_was_clicked.then_some(count)
                } else {
                    None
                };

                grid.push(RenderedGridElement {
                    ty,
                    flagged: self.flagged.contains(&pos),
                    count,
                });
            }
        }

        grid
    }

    pub fn game_has_been_won(&self) -> bool {
        !self.game_has_been_lost() && !self.mines.is_empty() && self.flagged == self.mines
    }

    pub const fn game_has_been_lost(&self) -> bool {
        self.losing_mine_clicked_on.is_some() || self.has_given_up
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
