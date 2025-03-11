use std::collections::{HashSet, VecDeque};
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::time::Duration;
use crate::data::Data;

const DURATION_SEPARATOR: char = '-';

#[derive(Debug)]
pub enum DurationSerError {
    CantFindSeparator,
    EmptySeconds,
    EmptyNanos,
    ErrorParsingSeconds(ParseIntError),
    ErrorParsingNanos(ParseIntError)
}

impl Display for DurationSerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CantFindSeparator => write!(f, "Unable to find separator {DURATION_SEPARATOR:?}"),
            Self::EmptySeconds => write!(f, "Seconds part was empty"),
            Self::EmptyNanos => write!(f, "Sub-second nanoseconds part was empty"),
            Self::ErrorParsingSeconds(e) => write!(f, "Error parsing seconds: {e}"),
            Self::ErrorParsingNanos(e) => write!(f, "Error parsing sub-second nanoseconds: {e}"),
        }
    }
}

impl std::error::Error for DurationSerError {
    fn source (&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::ErrorParsingNanos(e) | Self::ErrorParsingSeconds(e) = &self {
            Some(e)
        } else {
            None
        }
    }
}

pub fn serialise_extra_time (extra_time: Duration) -> String {
    let secs = extra_time.as_secs();
    let nanos = extra_time.subsec_nanos();

    format!("{secs}{DURATION_SEPARATOR}{nanos}")
}

pub fn deserialise_extra_time (sered: impl AsRef<str>) -> Result<Duration, DurationSerError> {
    let sered = sered.as_ref();
    let (secs, nanos) = {
        let sep_index = sered.find(DURATION_SEPARATOR).ok_or(DurationSerError::CantFindSeparator)?;
        (&sered[..sep_index], &sered[(sep_index + 1)..])
    };

    if secs.is_empty() {
        return Err(DurationSerError::EmptySeconds);
    }
    let secs = secs.parse().map_err(DurationSerError::ErrorParsingSeconds)?;

    if nanos.is_empty() {
        return Err(DurationSerError::EmptyNanos);
    }
    let nanos = nanos.parse().map_err(DurationSerError::ErrorParsingNanos)?;

    Ok(Duration::new(secs, nanos))
}


#[derive(Debug)]
pub enum DataReadError {
    UnableToParseInteger(ParseIntError),
    UnableToConvertVec(Vec<usize>),
    NotEnoughData,
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
            Self::NotEnoughData => {
                write!(f, "Not enough data present in the string to fully parse")
            }
            Self::UnableToConvertVec(v) => write!(f, "Unable to convert stats vec to array: {v:?}"),
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
    BadCharacter,
}

impl Display for InvalidDataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroMines => write!(f, "Found data with zero mines"),
            Self::TooSmallWidth => write!(f, "Found data 1 or less width"),
            Self::TooSmallHeight => write!(f, "Found data 1 or less height"),
            Self::TooManyMines => write!(f, "Found data with more mines than allowed mine spaces"),
            Self::BadCharacter => write!(f, "Found data with unknown character in serialised form"),
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

        let should_contain_data = match chars.next() {
            Some('y') => true,
            Some('n') => false,
            None => return Err(DataReadError::NotEnoughData),
            _ => {
                return Err(DataReadError::InvalidDataFound(
                    InvalidDataError::TooSmallHeight,
                ));
            }
        };

        let mut get_numbers = |n| {
            let mut numbers: Vec<usize> = Vec::with_capacity(n);

            for ch in &mut chars {
                if ch.is_ascii_digit() {
                    accum.push(ch);
                } else {
                    numbers.push(accum.parse()?);
                    accum.clear();

                    if numbers.len() == n {
                        break;
                    }
                }
            }

            Ok::<_, DataReadError>(numbers)
        };

        let [width, height, n_flagged, n_clicked, n_mines] = get_numbers(5)?
            .try_into()
            .map_err(DataReadError::UnableToConvertVec)?;

        if width <= 1 {
            return Err(DataReadError::InvalidDataFound(
                InvalidDataError::TooSmallWidth,
            ));
        } else if n_mines > (width * height - 1) {
            return Err(DataReadError::InvalidDataFound(
                InvalidDataError::TooManyMines,
            ));
        }

        let (flagged, clicked, mines) = if should_contain_data {
            //according to the docs, this conversion is guaranteed not to re-allocate and will take O(1)
            let mut numbers: VecDeque<_> = get_numbers(n_flagged + n_clicked + n_mines)?.into();
            debug_assert_eq!(numbers.len(), n_flagged + n_clicked + n_mines);

            let mut get_hashset = |count| {
                (0..count)
                    .map(|_i| {
                        numbers
                            .pop_front()
                            .map(|index| Self::index_to_coords(index, width))
                            .ok_or(DataReadError::NotEnoughData)
                    })
                    .collect::<Result<_, _>>()
            };

            let flagged = get_hashset(n_flagged)?;
            let clicked = get_hashset(n_clicked)?;
            let mines = get_hashset(n_mines)?;

            debug_assert!(numbers.is_empty());

            (flagged, clicked, mines)
        } else {
            (HashSet::new(), HashSet::new(), HashSet::new())
        };

        let res = Self {
            width,
            height,
            number_of_mines: n_mines,
            flagged,
            clicked,
            mines,
        };

        println!("desered shitty hash: {}", res.shitty_hash());

        Ok(res)
    }
}

impl From<Data> for String {
    fn from(data: Data) -> Self {
        println!("serialising shitty hash: {}", data.shitty_hash());

        let Data {
            width,
            height,
            number_of_mines,
            flagged,
            clicked,
            mines,
        } = data;

        let (mut output, n_mines) = if mines.is_empty() {
            debug_assert!(mines.is_empty());
            debug_assert!(flagged.is_empty());
            debug_assert!(clicked.is_empty());

            ('n'.to_string(), number_of_mines)
        } else {
            ('y'.to_string(), mines.len())
        };

        for n in [width, height, flagged.len(), clicked.len(), n_mines]
            .into_iter()
            .chain(
                flagged
                    .into_iter()
                    .chain(clicked)
                    .chain(mines)
                    .map(|pos| Data::coords_to_index(pos, width)),
            )
        {
            output.push_str(&format!("{n},")); //make sure to add a trailing comma so the last number gets parsed!
            //i originally didn't do this, but it made the deser way easier lol so why not
        }

        output
    }
}