use std::fmt::{Debug, Formatter};
use serde::{Deserialize, Serialize};
use crate::{Character};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Action {
    Propose(usize, Box<Action>),
    Income(usize),
    ForeignAid(usize),
    Tax(usize),
    Assassinate(usize, usize),
    Coup(usize, usize),
    Steal(usize, usize),
    Exchange(usize, usize),
    Block(usize, Character),
    Relent(usize),
    Challenge(usize),
    Lose(usize, usize), // index of card revealed & lost, and if to end the turn after the loss
    Reveal(usize, usize), // index of card exchanged
    Pass(usize),
    Resolve(usize)
}

impl Debug for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

        match self {
            Action::Propose(player_idx, proposal) => {
                f.write_fmt(format_args!("Player {player_idx} proposes \"{:?}\"", proposal))
            },
            Action::Income(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} takes Income"))
            }
            Action::ForeignAid(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} gets foreign aid"))
            }
            Action::Assassinate(player_idx, target_player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} assassinates {target_player_idx}"))
            }
            Action::Coup(player_idx, target_player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} coups {target_player_idx}"))
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
            Action::Block(player_idx, character) => {
                f.write_fmt(format_args!("Player {player_idx} blocks with {:?}", character))
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
            Action::Relent(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} relents to the block"))
            }
            Action::Resolve(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} resolves their proposal"))
            }
        }
    }
}
