#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Tile {
    //first bit is mine
    //second bit is flagged
    //third bit is discovered
    state: u8
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TileInteractionState {
    Undiscovered,
    Flagged,
    Discovered
}

impl Tile {
    pub fn new (is_mine: bool, tis: TileInteractionState) -> Self {
        let first_part = is_mine as u8;
        let second_part = match tis {
            TileInteractionState::Discovered => 0b01,
            TileInteractionState::Flagged => 0b10,
            TileInteractionState::Undiscovered => 0b00
        };

        Self {
            state: first_part | (second_part << 1)
        }
    }

    pub fn get_state(&self) -> u8 {
        self.state
    }

    //TODO: check invariants lollll
    pub fn from_state (state: u8) -> Self {
        Self {state}
    }

    pub fn is_mine (&self) -> bool {
        (self.state & 0b1) > 0
    }

    pub fn is_flagged (&self) -> bool {
        (self.state & (0b1 << 1)) > 0 && (self.state & (0b1 << 2)) == 0
    }

    pub fn is_discovered (&self) -> bool {
        (self.state & (0b1 << 2)) > 0
    }

    pub fn get_tis (&self) -> TileInteractionState {
        if self.state & (0b1 << 2) > 0 {
            TileInteractionState::Discovered
        } else if self.state & (0b1 << 1) > 0 {
            TileInteractionState::Flagged
        } else {
            TileInteractionState::Undiscovered
        }
    }

    pub fn toggle_flag (&mut self) {
        self.state ^= 0b1 << 1;
    }

    pub fn click (&mut self) {
        self.state |= 0b1 << 2;
    }
}

#[cfg(test)]
mod tests {
    use crate::tile::{Tile, TileInteractionState};

    #[test]
    fn check_tis_state_transitions() {
        for is_mine in &[true, false] {
            let mut tile = Tile::new(*is_mine, TileInteractionState::Undiscovered);

            assert_eq!(tile.get_tis(), TileInteractionState::Undiscovered);
            assert_eq!(tile.is_mine(), *is_mine);
            assert!(!tile.is_discovered());
            assert!(!tile.is_flagged());

            tile.toggle_flag();
            assert_eq!(tile.get_tis(), TileInteractionState::Flagged);
            assert_eq!(tile.is_mine(), *is_mine);
            assert!(!tile.is_discovered());
            assert!(tile.is_flagged());

            tile.toggle_flag();
            assert_eq!(tile.get_tis(), TileInteractionState::Undiscovered);
            assert_eq!(tile.is_mine(), *is_mine);
            assert!(!tile.is_discovered());
            assert!(!tile.is_flagged());

            tile.click();
            assert_eq!(tile.get_tis(), TileInteractionState::Discovered);
            assert_eq!(tile.is_mine(), *is_mine);
            assert!(tile.is_discovered());
            assert!(!tile.is_flagged());

            tile = Tile::new(*is_mine, TileInteractionState::Flagged);
            tile.click();
            assert_eq!(tile.get_tis(), TileInteractionState::Discovered);
            assert_eq!(tile.is_mine(), *is_mine);
            assert!(tile.is_discovered());
            assert!(!tile.is_flagged());

            tile.toggle_flag();
            assert_eq!(tile.get_tis(), TileInteractionState::Discovered);
            assert_eq!(tile.is_mine(), *is_mine);
            assert!(tile.is_discovered());
            assert!(!tile.is_flagged());
        }


    }
}