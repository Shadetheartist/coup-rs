use crate::{Character, CharacterAction};

#[derive(Clone, Debug)]
pub enum ProposableAction {
    ForeignAid,
    CharacterAction(Character, CharacterAction),
}

#[derive(Clone, Debug)]
pub struct ProposedAction {
    pub proposer_player_idx: usize,
    pub action: ProposableAction,
}

#[derive(Clone, Debug)]
pub struct BlockAction {
    pub blocked_player_idx: usize,
    pub blocker_player_idx: usize,
    pub blocker_claimed_character: Character,
}

#[derive(Clone, Debug)]
pub struct ChallengeAction {
    pub challenger_player_idx: usize,
    pub challenged_player_idx: usize,
    pub challenged_claimed_character: Character,
}

#[derive(Clone, Debug)]
pub struct ProveAction {
    pub prover_player_idx: usize,
    pub challenger_player_idx: usize,
    pub action: Prove,
    pub challenge: ChallengeAction,
}

#[derive(Clone, Debug)]
pub enum Prove {
    Win(usize), // card idx prover reveals & wins with
    Lose(usize), // card idx prover loses
}

#[derive(Clone, Debug)]
pub struct CoupAction {
    pub coup_player_idx: usize,
    pub couped_player_idx: usize,
}

#[derive(Clone, Debug)]
pub struct LoseAction {
    pub loser_player_idx: usize,
    pub discarded_influence_card_idx: usize,
}

#[derive(Clone, Debug)]
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
