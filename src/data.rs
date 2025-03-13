use fastrand::Rng;
use std::collections::HashSet;

#[derive(Clone)]
pub struct Data {
    pub width: usize,
    pub height: usize,
    pub number_of_mines: usize,
    pub flagged: HashSet<(usize, usize)>,
    pub clicked: HashSet<(usize, usize)>,
    pub mines: HashSet<(usize, usize)>,
}

impl Data {
    pub fn new_blank(width: usize, height: usize, number_of_mines: usize) -> Self {
        Self {
            width,
            height,
            number_of_mines,
            flagged: HashSet::new(),
            clicked: HashSet::new(),
            mines: HashSet::new(),
        }
    }

    #[inline]
    pub const fn index_to_coords(idx: usize, width: usize) -> (usize, usize) {
        (idx % width, idx / width)
    }

    #[inline]
    pub const fn coords_to_index((x, y): (usize, usize), width: usize) -> usize {
        y * width + x
    }

    pub fn total_uninteracted<const FLAGS_ARE_INTERACTION: bool>(&self) -> usize {
        //for each column
        (0..self.width)
            //and each row
            .flat_map(|x| (0..self.height).map(move |y| (x, y)))
            //remove every pos that has been clicked/flagged
            .filter(|pos| {
                !(self.clicked.contains(pos)
                    || (self.flagged.contains(pos) && FLAGS_ARE_INTERACTION))
            })
            //and count the remaining ones
            .count()
    }

    pub fn shitty_hash(&self) -> usize {
        let sum = |iter: &HashSet<_>, multiplier| {
            iter.iter()
                .copied()
                .map(|pos| Self::coords_to_index(pos, self.width) * multiplier)
                .sum::<usize>()
        };

        self.height * self.width * self.number_of_mines
            + sum(&self.mines, 1)
            + sum(&self.clicked, 3)
            + sum(&self.flagged, 5)
    }

    pub fn toggle_flag(&mut self, pos: (usize, usize)) -> bool {
        if self.flagged.contains(&pos) {
            self.flagged.remove(&pos);
        } else if self.flagged.len() < self.mines.len()
            && !self.mines.is_empty()
            && !self.clicked.contains(&pos)
        {
            self.flagged.insert(pos);

            return self.total_uninteracted::<false>() == self.mines.len();
        }

        false
    }

    pub fn get_neighbours(
        &self,
        (x, y): (usize, usize),
        include_diagonals: bool,
    ) -> impl Iterator<Item = (usize, usize)> + use<> {
        //0, 0 is top left
        let left = x.checked_sub(1);
        let horiz_middle = Some(x);
        let right = if x < (self.width - 1) {
            Some(x + 1)
        } else {
            None
        };

        let above = y.checked_sub(1);
        let vert_middle = Some(y);
        let below = if y < (self.height - 1) {
            Some(y + 1)
        } else {
            None
        };

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

    pub fn click(&mut self, pos: (usize, usize), rng: &mut Rng) -> bool {
        //if mines are empty, we need to add more mines!
        if self.mines.is_empty() {
            self.mines.extend(rng.choose_multiple(
                (0..(self.width * self.height))
                    .map(|x| Self::index_to_coords(x, self.width))
                    .filter(|candidate| pos != *candidate),
                self.number_of_mines,
            ));
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
        let mut maybe_auto_flag_these = HashSet::new();

        //TODO: DRY on checking
        let mut mine_is_next_to_chosen = false;
        for mine_neighbour in neighbours_to_check
            .iter()
            .filter(|x| self.mines.contains(*x))
        {
            maybe_auto_flag_these.insert(*mine_neighbour);
            mine_is_next_to_chosen = true;
        }

        //don't expand if we're next to a mine
        if mine_is_next_to_chosen {
            neighbours_to_check.clear();
        }

        while let Some(neighbour) = neighbours_to_check.pop() {
            //if this square is a mine, add it to the tobechecked list and continue to the next element
            if self.mines.contains(&neighbour) {
                maybe_auto_flag_these.insert(neighbour);
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
                maybe_auto_flag_these.insert(mine);
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
        for mine in maybe_auto_flag_these {
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
        !self.mines.is_empty() //there are mines
            && self.total_uninteracted::<false>() == self.mines.len() //the number of squares unconfirmed is the same as the number of mines
            && self.mines.is_disjoint(&self.clicked) //we didn't click on any mines
    }

    pub fn game_has_been_lost(&self) -> bool {
        //because iterators are lazy, this method isn't quite as bad as it looks
        //if you dig into the code, it basically just goes through all of `self.mines`, and finds the first element that `self.clicked` also contains
        self.mines.intersection(&self.clicked).next().is_some()
    }

    pub fn game_is_over(&self) -> bool {
        //should maybe be faster than calling `game_has_been_won() || game_has_been_lost()`
        (!self.mines.is_empty())
            && (self.mines.intersection(&self.clicked).next().is_some()
                || self.total_uninteracted::<false>() == self.mines.len())
    }
}
