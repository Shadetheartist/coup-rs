use std::fmt::{Debug, Formatter};
use crate::{Character, CharacterAction};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProposableAction {
    ForeignAid,
    CharacterAction(Character, CharacterAction),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposedAction {
    pub proposer_player_idx: usize,
    pub action: ProposableAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockAction {
    pub blocked_player_idx: usize,
    pub blocker_player_idx: usize,
    pub blocker_claimed_character: Character,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChallengeAction {
    pub challenger_player_idx: usize,
    pub challenged_player_idx: usize,
    pub challenged_claimed_character: Character,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProveAction {
    pub prover_player_idx: usize,
    pub challenger_player_idx: usize,
    pub action: Prove,
    pub challenge: ChallengeAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prove {
    Win(usize), // card idx prover reveals & wins with
    Lose(usize), // card idx prover loses
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoupAction {
    pub coup_player_idx: usize,
    pub couped_player_idx: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoseAction {
    pub loser_player_idx: usize,
    pub discarded_influence_card_idx: usize,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Action {
    Income(usize),
    Pass(usize),
    Relent(usize),
    Coup(CoupAction),
    Propose(ProposedAction),
    Challenge(ChallengeAction),
    Block(BlockAction),
    Prove(ProveAction),
    Lose(LoseAction),
}

impl Debug for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Income(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} takes Income"))
            }
            Action::Pass(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} passes priority"))
            }
            Action::Relent(player_idx) => {
                f.write_fmt(format_args!("Player {player_idx} relents to the block"))
            }
            Action::Coup(args) => {
                f.write_fmt(format_args!("Player {} coups player {}", args.coup_player_idx, args.couped_player_idx))
            }
            Action::Propose(args) => {
                f.write_fmt(format_args!("Player {} proposes {:?}", args.proposer_player_idx, args.action))
            }
            Action::Challenge(args) => {
                f.write_fmt(format_args!("Player {} challenges player {}'s ownership of {:?}", args.challenger_player_idx, args.challenged_player_idx, args.challenged_claimed_character))
            }
            Action::Block(args) => {
                f.write_fmt(format_args!("Player {} blocks player {}'s proposal", args.blocker_player_idx, args.blocked_player_idx))
            }
            Action::Prove(args) => {
                match args.action {
                    Prove::Win(card_idx) => {
                        f.write_fmt(format_args!("Player {} wins player {}'s challenge by revealing an influence card [{}]", args.prover_player_idx, args.challenger_player_idx, card_idx))
                    }
                    Prove::Lose(card_idx) => {
                        f.write_fmt(format_args!("Player {} loses player {}'s challenge and reveals/loses an influence card [{}]", args.challenger_player_idx, args.prover_player_idx, card_idx))
                    }
                }

            }
            Action::Lose(args) => {
                f.write_fmt(format_args!("Player {} loses an influence and reveals influence card [{}]", args.loser_player_idx, args.discarded_influence_card_idx))
            }
        }
    }
}
