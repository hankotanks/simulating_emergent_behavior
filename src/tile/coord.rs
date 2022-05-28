use std::cell::Cell;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) struct Coord {
    x: usize,
    y: usize
}

impl Coord {
    pub(crate) fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    pub(crate) fn apply_offset(&mut self, offset: Offset, dimensions: &iced::Size<usize>) {
        use Offset::*;
        match offset {
            X(d) => {
                let x = self.x as isize + d.get();
                self.x = if x >= 0 {
                    x as usize % dimensions.width
                } else {
                    dimensions.width - x as usize
                }
            },
            Y(d) => {
                let y = self.y as isize + d.get();
                self.y = if y >= 0 {
                    y as usize % dimensions.width
                } else {
                    dimensions.width - y as usize
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum Offset {
    X(Cell<isize>),
    Y(Cell<isize>)
}

impl Offset {
    pub(crate) fn new_x(distance: isize) -> Self {
        Self::X(Cell::new(distance))
    }

    pub(crate) fn new_y(distance: isize) -> Self {
        Self::Y(Cell::new(distance))
    }

    pub(crate) fn from_direction(direction: crate::agent::Direction) -> Self {
        use crate::agent::Direction::*;
        match direction {
            Up => Self::new_y(-1),
            Down => Self::new_y(1),
            Left => Self::new_x(-1),
            Right => Self::new_x(1)
        }
    }

    // unsure if this method is needed
    pub(crate) fn distance(&self) -> isize {
        use Offset::*;
        match self {
            X(d) | Y(d) => d.get()
        }
    }

    pub(crate) fn blank(&self) -> bool {
        use Offset::*;
        match self {
            X(d) | Y(d) => d.get() == 0
        }
    }

    pub(crate) fn signum(&mut self) -> Self {
        use Offset::*;
        match self {
            X(d) | Y(d) => {
                let s = d.get().signum();

                // reduce the distance of the Offset
                d.set(d.get() - 1isize * s);

                match self {
                    X(..) => X(Cell::new(s)),
                    Y(..) => Y(Cell::new(s))
                }
            }
        }
    }
}











