use rand::Rng;
use rand::rngs::ThreadRng;
use std::collections::HashSet;
use std::default::Default;

pub struct Board {
    losing_mine_clicked_on: Option<(usize, usize)>,
    has_given_up: bool,
    data: Data,
    rng: ThreadRng,
}

#[derive(Clone)]
pub struct Data {
    pub width: usize,
    pub number_of_mines: usize,
    pub flagged: HashSet<(usize, usize)>,
    pub clicked: HashSet<(usize, usize)>,
    pub mines: HashSet<(usize, usize)>,
}

impl Data {
    pub fn new (width: usize, number_of_mines: usize) -> Self {
        Self {
            width,
            number_of_mines,
            flagged: HashSet::new(),
            clicked: HashSet::new(),
            mines: HashSet::new()
        }
    }
}

impl Board {
    pub fn new(width: usize, number_of_mines: usize) -> Option<Self> {
        if width <= 1 || number_of_mines == 0 || number_of_mines > (width * width) {
            return None;
        }

        Some(Self {
            losing_mine_clicked_on: None,
            has_given_up: false,
            data: Data::new(width, number_of_mines),
            rng: ThreadRng::default(),
        })
    }

    pub fn from_previous_data (data: Data) -> Self {
        let mut losing_mine_clicked_on = None;

        for mine in &data.mines {
            if data.clicked.contains(mine) {
                losing_mine_clicked_on = Some(*mine);
                break;
            }
        }

        Self {
            losing_mine_clicked_on,
            has_given_up: false,
            data,
            rng: ThreadRng::default()
        }
    }

    pub fn reset(&mut self, new_mines_width: Option<(usize, usize)>) {
        self.losing_mine_clicked_on = None;
        self.has_given_up = false;

        let (new_width, new_mines) = new_mines_width.unwrap_or((self.data.width, self.data.number_of_mines));
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

    pub fn successfully_flagged (&self) -> usize {
        self.data.flagged.intersection(&self.data.mines).count()
    }

    pub fn give_up(&mut self) {
        self.has_given_up = true;
    }

    pub fn get_data (&self) -> &Data {
        &self.data
    }

    const fn index_to_coords(&self, idx: usize) -> (usize, usize) {
        (idx / self.data.width, idx % self.data.width)
    }

    const fn coords_to_index(&self, (x, y): (usize, usize)) -> usize {
        y * self.data.width + x
    }

    pub fn toggle_flag(&mut self, pos: (usize, usize)) {
        if self.game_has_been_won() || self.game_has_been_lost() {
            return;
        }

        if self.data.flagged.contains(&pos) {
            self.data.flagged.remove(&pos);
        } else if self.data.flagged.len() < self.data.mines.len(){
            self.data.flagged.insert(pos);
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

        self.data.clicked.insert(pos);
        let clicked_idx = self.coords_to_index(pos);
        if self.data.mines.is_empty() {
            let mut left_to_rx = self.data.number_of_mines;

            loop {
                let new_mine_candidate = self.rng.random_range(0..(self.data.width * self.data.width));
                if new_mine_candidate == clicked_idx {
                    continue;
                }

                self.data.mines.insert(self.index_to_coords(new_mine_candidate));

                left_to_rx -= 1;
                if left_to_rx == 0 {
                    break;
                }
            }

            return false;
        }

        if self.data.mines.contains(&pos) {
            self.losing_mine_clicked_on = Some(pos);
        } else {
            let mut to_be_checked: Vec<_> = Self::get_neighbours(pos, self.data.width).collect();
            let mut to_be_ignored = HashSet::new();

            while let Some(neighbour) = to_be_checked.pop() {
                if self.data.mines.contains(&neighbour)
                    || self.data.clicked.contains(&neighbour)
                    || self.data.flagged.contains(&neighbour)
                    || to_be_ignored.contains(&neighbour)
                {
                    continue;
                }

                let neighbours_neighbours: Vec<_> =
                    Self::get_neighbours(neighbour, self.data.width).collect();
                let neighbours_neighbour_count = neighbours_neighbours
                    .iter()
                    .filter(|x| self.data.mines.contains(x))
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
        let mut grid = Vec::with_capacity(self.data.width * self.data.width);

        for y in 0..self.data.width {
            for x in 0..self.data.width {
                let pos = (x, y);
                let ty = if self
                    .losing_mine_clicked_on
                    .is_some_and(|whoops| whoops == pos)
                {
                    GridElementType::Exploded
                } else if self.data.mines.contains(&pos) && self.game_has_been_lost() {
                    GridElementType::Mine
                } else if self.data.clicked.contains(&pos) {
                    GridElementType::Discovered
                } else {
                    GridElementType::Undiscovered
                };

                let count = if ty == GridElementType::Undiscovered && !self.data.mines.contains(&pos)
                {
                    let mut count = 0;
                    let mut neighbour_was_clicked = false;

                    for ctc in Self::get_neighbours(pos, self.data.width) {
                        if self.data.mines.contains(&ctc) {
                            count += 1;
                        }

                        neighbour_was_clicked |= self.data.clicked.contains(&ctc);
                    }
                    neighbour_was_clicked &= ty != GridElementType::Discovered;

                    neighbour_was_clicked.then_some(count)
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
        !self.game_has_been_lost() && !self.data.mines.is_empty() && self.data.flagged == self.data.mines
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
