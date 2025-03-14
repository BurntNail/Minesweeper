use crate::data::Data;
use crate::ser::InvalidDataError;
use egui::ahash::HashMap;
use egui::{ColorImage, Context, Rect, TextureHandle, TextureOptions, pos2, Color32};
use fastrand::Rng;
use image::{ImageFormat, ImageReader};
use std::collections::hash_map::Entry;
use std::default::Default;
use std::io::Cursor;

pub struct Board {
    ///Has the player chosen to give up?
    pub has_given_up: bool,
    ///The current board data
    data: Data,
    ///The RNG used for random number generation
    rng: Rng,
}

impl TryFrom<Data> for Board {
    type Error = InvalidDataError;

    fn try_from(data: Data) -> Result<Self, Self::Error> {
        //check various invariants for creating from previous data
        if data.width <= 1 {
            return Err(InvalidDataError::TooSmallWidth);
        } else if data.height <= 1 {
            return Err(InvalidDataError::TooSmallHeight);
        } else if data.number_of_mines == 0 {
            return Err(InvalidDataError::ZeroMines);
        } else if data.number_of_mines > (data.width * data.width - 1) {
            return Err(InvalidDataError::TooManyMines);
        }

        Ok(Self {
            has_given_up: false,
            data,
            rng: Rng::default(),
        })
    }
}

impl Board {
    pub fn new(
        width: usize,
        height: usize,
        number_of_mines: usize,
    ) -> Result<Self, InvalidDataError> {
        Self::try_from(Data::new_blank(width, height, number_of_mines))
    }

    pub fn from_previous_data(data: Data) -> Result<Self, InvalidDataError> {
        Self::try_from(data)
    }

    pub fn reset(&mut self, new_width_height_mines: Option<(usize, usize, usize)>) {
        self.has_given_up = false;

        let (new_width, new_height, new_mines) = new_width_height_mines.unwrap_or((
            self.data.width,
            self.data.height,
            self.data.number_of_mines,
        ));
        self.data = Data::new_blank(new_width, new_height, new_mines);
    }

    pub const fn get_width(&self) -> usize {
        self.data.width
    }
    pub const fn get_height(&self) -> usize {
        self.data.height
    }

    pub const fn total_mines(&self) -> usize {
        self.data.number_of_mines
    }

    pub fn total_uninteracted(&self) -> usize {
        self.data.total_uninteracted::<true>()
    }

    pub fn flags_placed(&self) -> usize {
        self.data.flagged.len()
    }

    pub fn successfully_flagged(&self) -> usize {
        self.data.flagged.intersection(&self.data.mines).count()
    }

    pub const fn get_data(&self) -> &Data {
        &self.data
    }

    ///returns whether the game is over
    pub fn toggle_flag(&mut self, pos: (usize, usize)) -> bool {
        if self.game_is_over() {
            //ensure can't flag when game over
            return true;
        }
        self.data.toggle_flag(pos)
    }

    ///returns whether the game is over
    pub fn click(&mut self, pos: (usize, usize)) -> bool {
        if self.game_is_over() {
            //ensure can't click when game over
            return true;
        }

        self.data.click(pos, &mut self.rng)
    }

    pub fn undo_mistake (&mut self) {
        if self.game_has_been_won() || !self.game_is_over() {
            return;
        }

        for mine in &self.data.mines {
            self.data.clicked.remove(mine);
        }
    }

    pub fn render(&self) -> Vec<RenderedGridElement> {
        let game_is_over = self.game_is_over();

        //for each column
        (0..self.data.height)
            //and each row
            .flat_map(|y| (0..self.data.width).map(move |x| (x, y)))
            .map(|pos| {
                //work out the type, based off of various factors
                let ty = if self.data.mines.contains(&pos) && self.data.clicked.contains(&pos) {
                    GridElementType::Exploded
                } else if self.data.mines.contains(&pos) && game_is_over {
                    GridElementType::Mine
                } else if self.data.clicked.contains(&pos) {
                    GridElementType::Discovered {
                        should_display_count: self
                            .data
                            .get_neighbours(pos, true)
                            .any(|neighbour| !self.data.clicked.contains(&neighbour)),
                    }
                } else {
                    GridElementType::Undiscovered
                };

                //and return the rendered grid element
                RenderedGridElement {
                    ty,
                    flagged: self.data.flagged.contains(&pos),
                }
            })
            .collect()
    }

    pub fn game_has_been_won(&self) -> bool {
        self.data.game_has_been_won()
    }

    pub fn game_has_been_lost(&self) -> bool {
        self.has_given_up || self.data.game_has_been_lost()
    }

    pub fn game_is_over(&self) -> bool {
        self.has_given_up || self.data.game_is_over()
    }

    pub fn generate_counts(&self) -> Option<Vec<u8>> {
        self.data.generate_counts()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default)]
pub enum SpriteAtlas {
    //https://github.com/Minesweeper-World/MS-Texture/blob/main/png/cells/WinmineXP.png
    #[default]
    WinMine,
    //https://www.spriters-resource.com/fullview/180218/
    RTXOn,
    //https://kia.itch.io/16x16-tileset-for-minesweeper
    DarkMode,
}

impl SpriteAtlas {
    //currently controlled by the RTXOn variant
    pub const MAX_TEXTURE_SIDE: usize = 3072;
    pub const ALL_VARIANTS: [Self; 3] = [Self::WinMine, Self::RTXOn, Self::DarkMode];

    pub const fn get_png_bytes(self) -> &'static [u8] {
        match self {
            Self::WinMine => include_bytes!("../WinmineXP.png"),
            Self::RTXOn => include_bytes!("../RTXOn.png"),
            Self::DarkMode => include_bytes!("../NightMode.png")
        }
    }

    pub const fn get_texture_options(self) -> TextureOptions {
        match self {
            Self::WinMine | Self::DarkMode => TextureOptions::NEAREST,
            Self::RTXOn => TextureOptions::LINEAR,
        }
    }

    pub const fn as_static_str(self) -> &'static str {
        match self {
            Self::WinMine => "WinMine XP",
            Self::RTXOn => "RTX On",
            Self::DarkMode => "Dark Mode",
        }
    }

    pub const fn background_colour (self) -> Option<Color32> {
        match self {
            Self::DarkMode => Some(Color32::from_rgb(0x2d, 0x17, 0x10)),
            _ => None
        }
    }
}

#[derive(Default, Clone)]
pub struct TextureCache(HashMap<SpriteAtlas, TextureHandle>);

impl TextureCache {
    pub fn get(&mut self, atlas: SpriteAtlas, ctx: &Context) -> TextureHandle {
        match self.0.entry(atlas) {
            Entry::Occupied(occ) => occ.get().clone(),
            Entry::Vacant(vac) => {
                let image =
                    ImageReader::with_format(Cursor::new(atlas.get_png_bytes()), ImageFormat::Png)
                        .decode()
                        .expect("unable to decode image") //panic because fatal error in init
                        .to_rgba8(); //convert to rgba8 so when we get the flat samples it's easy to give it to the egui image
                let (w, h) = image.dimensions();
                let pixels = image.as_flat_samples();
                let img =
                    ColorImage::from_rgba_unmultiplied([w as usize, h as usize], pixels.as_slice());

                let handle =
                    ctx.load_texture(atlas.as_static_str(), img, atlas.get_texture_options());

                vac.insert(handle).clone()
            }
        }
    }
}

///A grid element that has been rendered - to display, use the [`RenderedGridElement::to_uv`] method
#[derive(Copy, Clone, Debug)]
pub struct RenderedGridElement {
    ty: GridElementType,
    flagged: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum GridElementType {
    Exploded,
    Discovered { should_display_count: bool },
    Undiscovered,
    Mine,
}

impl RenderedGridElement {
    #[allow(clippy::too_many_lines)]
    pub fn to_uv(self, count: u8, game_is_over: bool, sprite_atlas: SpriteAtlas) -> Rect {
        let rect_function_creator = |x_width: f32, y_width: f32, extra_y_sf: Option<f32>| {
            let x_divisor = 1.0 / x_width;
            let y_divisor = 1.0 / y_width * extra_y_sf.unwrap_or(1.0);

            move |x, y| {
                let (x, y) = (x as f32, y as f32);
                Rect {
                    min: pos2(x_divisor * x, y_divisor * y),
                    max: pos2(x_divisor * (x + 1.0), y_divisor * (y + 1.0)),
                }
            }
        };

        match sprite_atlas {
            SpriteAtlas::WinMine => {
                let rect = rect_function_creator(4.0, 4.0, None);

                if self.flagged {
                    return if game_is_over && self.ty != GridElementType::Mine {
                        rect(3, 2)
                    } else {
                        rect(2, 2)
                    };
                }

                match self.ty {
                    GridElementType::Exploded => rect(3, 3),
                    GridElementType::Undiscovered => rect(1, 2),
                    GridElementType::Discovered {
                        should_display_count,
                    } => {
                        if should_display_count {
                            match count {
                                1 => rect(0, 0),
                                2 => rect(1, 0),
                                3 => rect(2, 0),
                                4 => rect(3, 0),
                                5 => rect(0, 1),
                                6 => rect(1, 1),
                                7 => rect(2, 1),
                                8 => rect(3, 1),
                                _ => rect(0, 2),
                            }
                        } else {
                            rect(0, 2)
                        }
                    }
                    GridElementType::Mine => rect(2, 3),
                }
            }
            SpriteAtlas::RTXOn => {
                let rect = rect_function_creator(4.0, 5.0, Some((512.0 * 5.0) / 3072.0));

                if self.flagged {
                    return if game_is_over && self.ty != GridElementType::Mine {
                        rect(1, 2)
                    } else {
                        rect(0, 1)
                    };
                }

                match self.ty {
                    GridElementType::Exploded => rect(2, 2),
                    GridElementType::Undiscovered => rect(0, 0),
                    GridElementType::Discovered {
                        should_display_count,
                    } => {
                        if should_display_count {
                            match count {
                                1 => rect(0, 3),
                                2 => rect(1, 3),
                                3 => rect(2, 3),
                                4 => rect(3, 3),
                                5 => rect(0, 4),
                                6 => rect(1, 4),
                                7 => rect(2, 4),
                                8 => rect(3, 4),
                                _ => rect(3, 0),
                            }
                        } else {
                            rect(3, 0)
                        }
                    }
                    GridElementType::Mine => rect(0, 2),
                }
            }
            SpriteAtlas::DarkMode => {
                let rect = rect_function_creator(4.0, 4.0, None);

                if self.flagged {
                    return if game_is_over && self.ty != GridElementType::Mine {
                        rect(3, 3)
                    } else {
                        rect(2, 0)
                    };
                }

                match self.ty {
                    GridElementType::Exploded => rect(3, 0),
                    GridElementType::Undiscovered => rect(2, 3),
                    GridElementType::Discovered {
                        should_display_count,
                    } => {
                        if should_display_count {
                            match count {
                                1 => rect(0, 1),
                                2 => rect(1, 1),
                                3 => rect(2, 1),
                                4 => rect(3, 1),
                                5 => rect(0, 2),
                                6 => rect(1, 2),
                                7 => rect(2, 2),
                                8 => rect(3, 2),
                                _ => rect(0, 3),
                            }
                        } else {
                            rect(0, 3)
                        }
                    }
                    GridElementType::Mine => rect(0, 0),
                }
            }

        }
    }
}
