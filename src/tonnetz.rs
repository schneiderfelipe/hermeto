use crate::{KeyCard, KeyboardLayout, Note};
use core::iter::{once, repeat_n};
use either::Either;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType},
};
use std::collections::HashSet;

pub(super) struct Tonnetz<'a, const N: u8, const K: u8> {
    base_note: Note,
    keyboard_layout: KeyboardLayout,
    pressed: HashSet<char>,
    style: Style,
    block: Option<Block<'a>>,
}

impl<const N: u8, const K: u8> Tonnetz<'_, N, K> {
    pub(super) fn new(base_note: Note) -> Self {
        Self {
            base_note,
            keyboard_layout: KeyboardLayout::default(),
            pressed: HashSet::default(),
            style: Style::default(),
            block: None,
        }
    }

    pub(super) const fn keyboard_layout(&self) -> KeyboardLayout {
        self.keyboard_layout
    }

    pub(super) fn note(&self, key: char) -> Option<Note> {
        self.keyboard_layout.find(key).map(|(n, k)| {
            let k = k - n / 2; // adjust for the tilt
            Note(
                u8::from(self.base_note)
                    + N * u8::try_from(n).unwrap()
                    + K * u8::try_from(k).unwrap(),
            )
        })
    }

    pub(super) fn press(&mut self, key: char) -> bool {
        cli_log::info!("pressed {key:?}");
        self.pressed.insert(key)
    }

    pub(super) fn release(&mut self, key: char) -> bool {
        cli_log::info!("released {key:?}");
        self.pressed.remove(&key)
    }

    fn is_pressed(&self, key: char) -> bool {
        self.pressed.contains(&key)
    }
}

impl<const N: u8, const K: u8> Widget for &Tonnetz<'_, N, K> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        buf.set_style(area, self.style);
        self.block.render(area, buf);

        let area = self.block.inner_if_some(area);
        let (n_rows, max_n_keys) = self.keyboard_layout.size();
        let rows_layout = Layout::vertical(Constraint::from_fills(repeat_n(1, n_rows))).split(area);
        let tilted_rows_layout = if self
            .keyboard_layout
            .rows()
            .into_iter()
            .enumerate()
            .filter(|(n, _)| n % 2 == 1)
            .all(|(_, row_layout)| row_layout.into_iter().last().is_some_and(|c| c.is_none()))
        {
            Either::Left(rows_layout.iter().enumerate().map(move |(n, row_layout)| {
                Layout::horizontal(match n % 2 {
                    0 => Constraint::from_fills(repeat_n(2, max_n_keys)),
                    1 => Constraint::from_fills(
                        once(1).chain(repeat_n(2, max_n_keys - 1)).chain(once(1)),
                    ),
                    _ => unreachable!(),
                })
                .split(*row_layout)
            }))
        } else {
            Either::Right(rows_layout.iter().enumerate().map(move |(n, row_layout)| {
                Layout::horizontal(match n % 2 {
                    0 => Constraint::from_fills(repeat_n(2, max_n_keys).chain(once(1))),
                    1 => Constraint::from_fills(once(1).chain(repeat_n(2, max_n_keys))),
                    _ => unreachable!(),
                })
                .split(*row_layout)
            }))
        };

        self.keyboard_layout
            .rows()
            .into_iter()
            .zip(tilted_rows_layout)
            .enumerate()
            .for_each(|(n, (row, keys_layout))| {
                row.into_iter()
                    .zip(keys_layout.iter().skip(n % 2))
                    .filter_map(|(key, key_layout)| {
                        key.and_then(|key| self.note(key).map(|note| (key, key_layout, note)))
                    })
                    .for_each(|(key, key_layout, note)| {
                        buf.set_style(area, self.style);
                        KeyCard::new(key, note)
                            .style(self.style)
                            .bg(if self.is_pressed(key) {
                                Color::Black
                            } else {
                                Color::Reset
                            })
                            .block(self.block.clone().unwrap_or_else(|| {
                                Block::bordered()
                                    .border_type(BorderType::Rounded)
                                    .style(self.style)
                            }))
                            .render(*key_layout, buf);
                    });
            });
    }
}
