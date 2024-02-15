use std::fmt::{Debug, Formatter};
use crate::{Character, CharacterAction};

#[derive(Clone, PartialEq, Eq)]
pub enum Action {
    Income(usize),
    ForeignTax(usize),
    Tax(usize),
    Assassinate(usize, usize),
    Steal(usize, usize),
    Exchange(usize, usize),
    Block(usize),
    Challenge(usize),
    Lose(usize, usize), // index of card revealed & lost
    Reveal(usize, usize), // index of card exchanged
    Pass(usize),
}

impl Debug for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

        match self {
            Action::Income(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} takes Income"))
            }
            Action::ForeignTax(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} passes priority"))

            }
            Action::Assassinate(player_idx, target_player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} assassinates {target_player_idx}"))
            }
            Action::Tax(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} gets Taxes"))
            }
            Action::Steal(player_idx, target_player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} steals from {target_player_idx}"))
            }
            Action::Exchange(player_idx, card_idx) => {
                f.write_fmt(format_args!("Player {player_idx} exchanges card {card_idx}"))
            }
            Action::Block(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} blocks"))
            }
            Action::Challenge(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} challenges"))
            }
            Action::Lose(player_idx, card_idx) => {
                f.write_fmt(format_args!("Player {player_idx} loses card {card_idx}"))
            }
            Action::Reveal(player_idx, card_idx) => {
                f.write_fmt(format_args!("Player {player_idx} reveals & exchanges card {card_idx}"))
            }
            Action::Pass(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} passes priority"))
            }
        }

    }
}
