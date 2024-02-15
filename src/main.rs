mod action;

use std::iter::Enumerate;
use rand::seq::SliceRandom;
use rand::{Rng, thread_rng};
use Character::Assassin;
use crate::action::{Action, BlockAction, ChallengeAction, CoupAction, LoseAction, ProposableAction, ProposedAction, Prove, ProveAction};
use crate::action::ProposableAction::{ForeignAid};
use crate::Character::{Ambassador, Captain, Contessa, Duke};
use crate::CharacterAction::{Assassinate, Exchange, Steal, Tax};

#[derive(Clone)]
struct Proposal {
    proposed_action: ProposedAction,
    response: Option<ProposalResponses>,
}

#[derive(Clone)]
struct Block {
    block_action: BlockAction,
    response: Option<BlockResponses>,
}

#[derive(Clone)]
struct Challenge {
    challenge_action: ChallengeAction,
    response: Option<ChallengeResponses>,
}

#[derive(Clone)]
struct Reveal {
    revealer_player_idx: usize,
    response: RevealResponses,
}

#[derive(Clone)]
struct Lose {
    loser_player_idx: usize,
}

#[derive(Clone)]
enum ProposalResponses {
    Block(Block),
    Challenge(Challenge),
}

#[derive(Clone)]
enum BlockResponses {
    Challenge(Challenge),
}

#[derive(Clone)]
enum ChallengeResponses {
    Reveal(Reveal),
    Lose(Lose),
}

#[derive(Clone)]
enum RevealResponses {
    Lose(Lose),
}


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Character {
    Duke,
    Assassin,
    Captain,
    Ambassador,
    Contessa,
}

static CHARACTER_VARIANTS: [Character; 5] = [
    Character::Duke,
    Assassin,
    Character::Captain,
    Character::Ambassador,
    Character::Contessa,
];


#[derive(Clone)]
struct Player {
    money: u8,
    influence_cards: Vec<(Character, bool)>, // (character, revealed)
}

#[derive(Debug)]
enum CoupError {
    InvalidAction(Action),
}

#[derive(Clone)]
struct Loser {
    loser_player_idx: usize,
    loser_lost_challenge: Option<ChallengeAction>,
}

#[derive(Clone)]
struct Coup {
    turn: usize,

    current_player_idx: usize,

    priority_player_idx: Option<usize>,
    proposal: Option<Proposal>,
    num_remaining_passers: Option<usize>,

    loser: Option<Loser>,

    deck: Vec<Character>,
    players: Vec<Player>,

}


impl Coup {
    fn new(num_players: u8) -> Self {
        let mut deck: Vec<Character> = CHARACTER_VARIANTS.iter()
            .flat_map(|&card| std::iter::repeat(card).take(3))
            .collect();

        let mut rng = thread_rng();
        deck.shuffle(&mut rng);

        let mut players = (0..num_players).map(|_| Player {
            money: 2,
            influence_cards: vec![(deck.remove(0), false), (deck.remove(0), false)],
        }).collect();

        Self {
            turn: 0,
            current_player_idx: 0,
            priority_player_idx: None,
            proposal: None,
            num_remaining_passers: None,
            loser: None,
            deck,
            players,
        }
    }

    fn active_player(&self) -> &Player {
        let idx = if let Some(priority_player_idx) = self.priority_player_idx {
            priority_player_idx
        } else {
            self.current_player_idx
        };

        &self.players[idx]
    }

    fn other_player_indexes(&self, exclude_idx: usize) -> Vec<usize> {
        (1..self.players.len())
            .map(|n| (exclude_idx + n) % self.players.len())
            .collect()
    }

    fn go_next_turn(&mut self) {
        // player's turn is over
        self.turn += 1;

        // clear priority
        self.priority_player_idx = None;

        // clear passer counter
        self.num_remaining_passers = None;

        // reset proposal
        self.proposal = None;

        // go to next player
        self.current_player_idx = self.next_living_player();
    }

    fn go_next_prio(&mut self) {
        self.priority_player_idx = Some(self.next_prio_player_idx());
    }

    fn replace_influence_card(&mut self, player_idx: usize, card_idx: usize) {
        let card = self.players[player_idx].influence_cards.remove(card_idx);
        if card.1 == true {
            panic!("shouldn't be able to lose/replace a revealed/lost influence card");
        }

        self.deck.push(card.0);

        let mut rng = thread_rng();
        self.deck.shuffle(&mut rng);

        self.players[player_idx].influence_cards.push((self.deck.remove(0), false));
    }

    fn lose_influence_card(&mut self, player_idx: usize, card_idx: usize) {
        // 'losing' an influence means your card is flipped up and revealed and doesn't count
        self.players[player_idx].influence_cards[card_idx].1 = true;
    }

    fn is_player_dead(&self, player_idx: usize) -> bool {
        self.players[player_idx].influence_cards.iter().filter(|x| x.1 == false).count() == 0
    }

    fn player_active_influence_cards(&self, player_idx: usize) -> impl Iterator<Item=(usize, &(Character, bool))> {
        self.players[player_idx].influence_cards.iter().filter(|e| e.1 == false).enumerate()
    }

    fn gen_actions(&self) -> Vec<Action> {
        let mut actions = vec![];

        // a player is tasked with choosing a card to discard
        if let Some(loser) = &self.loser {
            match &loser.loser_lost_challenge {
                None => {
                    add_lose_actions(self, &mut actions, loser);
                }
                Some(challenge_action) => {
                    for (card_idx, _) in self.player_active_influence_cards(loser.loser_player_idx) {
                        actions.push(Action::Prove(ProveAction {
                            prover_player_idx: challenge_action.challenged_player_idx,
                            challenger_player_idx: challenge_action.challenger_player_idx,
                            action: Prove::Lose(card_idx),
                            challenge: challenge_action.clone(),
                        }));
                    }

                }
            }
            return actions;
        }

        fn add_lose_actions(game: &Coup, actions: &mut Vec<Action>, loser: &Loser) {
            for (card_idx, _) in game.player_active_influence_cards(loser.loser_player_idx) {
                actions.push(Action::Lose(LoseAction {
                    loser_player_idx: loser.loser_player_idx,
                    discarded_influence_card_idx: card_idx,
                }));
            }
        }

        fn add_proposal_actions(game: &Coup, actions: &mut Vec<Action>) {
            // forced coup
            if game.players[game.current_player_idx].money >= 10 {
                for opponent_idx in game.other_player_indexes(game.current_player_idx) {
                    actions.push(
                        Action::Coup(CoupAction {
                            coup_player_idx: game.current_player_idx,
                            couped_player_idx: opponent_idx,
                        })
                    );
                }
            } else {
                actions.push(Action::Income(game.current_player_idx));
                actions.push(Action::Propose(ProposedAction { proposer_player_idx: game.current_player_idx, action: ForeignAid }));
                actions.push(Action::Propose(ProposedAction { proposer_player_idx: game.current_player_idx, action: ProposableAction::CharacterAction(Duke, Tax) }));

                for card_idx in 0..game.players[game.current_player_idx].influence_cards.len() {
                    actions.push(Action::Propose(ProposedAction { proposer_player_idx: game.current_player_idx, action: ProposableAction::CharacterAction(Ambassador, Exchange(card_idx)) }));
                }

                for opponent_idx in game.other_player_indexes(game.current_player_idx) {
                    if game.players[game.current_player_idx].money >= 7 {
                        actions.push(
                            Action::Coup(CoupAction {
                                coup_player_idx: game.current_player_idx,
                                couped_player_idx: opponent_idx,
                            })
                        );
                    } else if game.players[game.current_player_idx].money >= 3 {
                        actions.push(Action::Propose(ProposedAction { proposer_player_idx: game.current_player_idx, action: ProposableAction::CharacterAction(Assassin, Assassinate(opponent_idx)) }));
                    }

                    if game.players[opponent_idx].money > 0 {
                        actions.push(Action::Propose(ProposedAction { proposer_player_idx: game.current_player_idx, action: ProposableAction::CharacterAction(Captain, Steal(opponent_idx)) }));
                    }
                }
            }
        }

        fn add_proposal_reactions(game: &Coup, actions: &mut Vec<Action>, proposal: &Proposal) {
            let priority_player_idx = game.priority_player_idx.expect("this should be a priority player");
            actions.push(Action::Pass(priority_player_idx));

            match proposal.proposed_action.action {
                ForeignAid => {
                    actions.push(
                        Action::Block(BlockAction {
                            blocked_player_idx: proposal.proposed_action.proposer_player_idx,
                            blocker_player_idx: priority_player_idx,
                            blocker_claimed_character: Duke,
                        })
                    );
                }
                ProposableAction::CharacterAction(character, _) => {}
            }

            if let ProposableAction::CharacterAction(character, _) = proposal.proposed_action.action {
                actions.push(
                    Action::Challenge(ChallengeAction {
                        challenger_player_idx: priority_player_idx,
                        challenged_player_idx: proposal.proposed_action.proposer_player_idx,
                        challenged_claimed_character: character,
                    })
                );
            }
        }

        fn handle_challenge(game: &Coup, actions: &mut Vec<Action>, challenge: &Challenge) {
            if let Some(challenge_response) = &challenge.response {
                match challenge_response {
                    ChallengeResponses::Reveal(reveal) => {
                        match &reveal.response {
                            RevealResponses::Lose(lose) => {}
                        }
                    }
                    ChallengeResponses::Lose(lose) => {}
                }
            } else {
                // challenged player needs to prove themselves or not

                // player may lose one of their cards as a result of not having proof (or if they
                // just want to.
                for (card_idx, _) in game.player_active_influence_cards(challenge.challenge_action.challenger_player_idx) {
                    actions.push(Action::Prove(ProveAction {
                        prover_player_idx: challenge.challenge_action.challenged_player_idx,
                        challenger_player_idx: challenge.challenge_action.challenger_player_idx,
                        action: Prove::Lose(card_idx),
                        challenge: challenge.challenge_action.clone(),
                    }));
                }

                let card_idx = {
                    game
                        .players[challenge.challenge_action.challenged_player_idx]
                        .influence_cards
                        .iter()
                        // not revealed and is the claimed character
                        .position(|e| e.1 == false && e.0 == challenge.challenge_action.challenged_claimed_character)
                };

                // player has the nuts, they can prove and win
                if let Some(card_idx) = card_idx {
                    actions.push(Action::Prove(ProveAction {
                        prover_player_idx: challenge.challenge_action.challenged_player_idx,
                        challenger_player_idx: challenge.challenge_action.challenger_player_idx,
                        action: Prove::Win(card_idx),
                        challenge: challenge.challenge_action.clone(),
                    }));
                }
            }
        }

        // structure of a turn
        match &self.proposal {
            None => {
                // no proposal set, player must take a proposal action
                add_proposal_actions(&self, &mut actions);
            }
            Some(proposal) => {
                match &proposal.response {
                    None => {
                        // a proposal has been made, that can be responded to, all players have a
                        // chance to respond, they each get priority in turn
                        add_proposal_reactions(&self, &mut actions, proposal);
                    }
                    Some(proposal_response) => {
                        // a proposal has been met with a response (block or challenge)
                        // the proposing player must respond
                        match proposal_response {
                            ProposalResponses::Block(block_response) => {
                                // if the proposal was blocked, the player may relent and lose their
                                // benefits, or they can challenge the blocker's validity
                                match &block_response.response {
                                    None => {
                                        // relent and get nothing
                                        actions.push(Action::Relent(proposal.proposed_action.proposer_player_idx));
                                    }
                                    Some(block_response) => {
                                        // challenge the blocker
                                        match block_response {
                                            BlockResponses::Challenge(challenge) => {
                                                match &challenge.response {
                                                    None => {
                                                        // block has not been responded to
                                                        handle_challenge(&self, &mut actions, challenge);
                                                    }
                                                    Some(response) => {
                                                        // block has already been responded to
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            ProposalResponses::Challenge(challenge) => {
                                handle_challenge(&self, &mut actions, challenge)
                            }
                        }
                    }
                }
            }
        }

        actions
    }

    fn apply_action(&self, action: Action) -> Result<Coup, CoupError> {
        let mut game = self.clone();

        println!("{} | {:?} -> ${} {:?} | {:?}", self.current_player_idx, self.priority_player_idx, self.active_player().money, self.active_player().influence_cards, action);

        match action {
            Action::Relent(_) => {
                game.go_next_turn();
            }
            Action::Income(player_idx) => {
                game.players[player_idx].money += 1;
                game.go_next_turn();
            }
            Action::Pass(_) => {
                if let Some(num_remaining_passers) = game.num_remaining_passers {
                    game.num_remaining_passers = Some(num_remaining_passers - 1);
                } else {
                    panic!("num_remaining_passers should be Some()");
                }
                game.go_next_prio();
            }
            Action::Coup(coup_action) => {
                game.loser = Some(Loser {
                    loser_player_idx: coup_action.couped_player_idx,
                    loser_lost_challenge: None,
                });
            }
            Action::Propose(proposal) => {
                game.proposal = Some(Proposal {
                    proposed_action: proposal,
                    response: None,
                });

                // set up passer counter
                game.num_remaining_passers = Some(game.players.len() - 1);

                game.go_next_prio();
            }
            Action::Challenge(challenge_action) => {
                game.proposal.as_mut().unwrap().response = Some(
                    ProposalResponses::Challenge(Challenge {
                        challenge_action,
                        response: None,
                    })
                );

                // pass prio back to proposer
                game.priority_player_idx = Some(game.proposal.as_ref().unwrap().proposed_action.proposer_player_idx);
            }
            Action::Block(block_action) => {
                game.proposal.as_mut().unwrap().response = Some(
                    ProposalResponses::Block(Block {
                        block_action,
                        response: None,
                    })
                );

                // pass prio back to proposer
                game.priority_player_idx = Some(game.proposal.as_ref().unwrap().proposed_action.proposer_player_idx);
            }
            Action::Prove(prove_action) => {
                match prove_action.action {
                    Prove::Win(card_idx) => {
                        // winner replaces their card first
                        game.replace_influence_card(prove_action.prover_player_idx, card_idx);

                        // then the challenger must choose to lose an influence
                        game.loser = Some(Loser {
                            loser_player_idx: prove_action.challenger_player_idx,
                            loser_lost_challenge: Some(prove_action.challenge),
                        });

                        // send priority to loser
                        game.priority_player_idx = Some(prove_action.challenger_player_idx);
                    }
                    Prove::Lose(card_idx) => {
                        game.lose_influence_card(prove_action.challenger_player_idx, card_idx);

                        // send priority back to winner
                        game.priority_player_idx = Some(prove_action.prover_player_idx);
                    }
                }
            }
            Action::Lose(lose_action) => {

                // player lost this influence card
                game.lose_influence_card(lose_action.loser_player_idx, lose_action.discarded_influence_card_idx);

                if let Some(loser) = &game.loser {
                    match loser.loser_lost_challenge {
                        None => {
                            // loss was not a challenge loss, so it was assassinate or coup, and so turn should end
                            game.go_next_turn();
                        }
                        Some(_) => {}
                    }
                }

                game.loser = None;
            }
        }

        if let Some(num_remaining_passers) = game.num_remaining_passers {
            if num_remaining_passers == 0 {
                println!("proposal has passed");

                // proposal has passed
                if let Some(proposal) = &game.proposal {
                    let player_idx = proposal.proposed_action.proposer_player_idx;
                    match proposal.proposed_action.action {
                        ForeignAid => {
                            game.players[player_idx].money += 2;
                            println!("player {} gets ForeignAid money (2)", player_idx);
                        }
                        ProposableAction::CharacterAction(character, character_action) => {
                            match character_action {
                                Tax => {
                                    game.players[player_idx].money += 3;
                                    println!("player {} gets Tax money (3)", player_idx);
                                }
                                Assassinate(target_player_idx) => {
                                    game.loser = Some(Loser {
                                        loser_player_idx: target_player_idx,
                                        loser_lost_challenge: None,
                                    });
                                    println!("player {} assassinates player {}", player_idx, target_player_idx);
                                }
                                Steal(target_player_idx) => {
                                    let amount = {
                                        if game.players[target_player_idx].money >= 2 {
                                            2
                                        } else if game.players[target_player_idx].money == 1 {
                                            1
                                        } else {
                                            panic!("shouldn't be able to steal more than the target has - action gen bug")
                                        }
                                    };

                                    game.players[target_player_idx].money -= amount;
                                    game.players[player_idx].money += amount;

                                    println!("player {} steals {} money from player {}", player_idx, amount, target_player_idx);
                                }
                                Exchange(card_idx) => {
                                    game.replace_influence_card(player_idx, card_idx);
                                    println!("player {} exchanges card {}", player_idx, card_idx);
                                }
                            }
                        }
                    }
                } else {
                    panic!("proposal should be Some()")
                }

                game.num_remaining_passers = None;
                game.go_next_turn();
            }
        }

        Ok(game)
    }

    fn is_terminal(&self) -> bool {
        return self.players
            .iter()
            .filter(|player| {
                // if the player has at least one unrevealed card they're still in the game
                player.influence_cards.iter().any(|card| card.1 == false)
            }).count() == 1;
    }

    fn next_living_player(&self) -> usize {
        let mut idx = self.current_player_idx;

        idx = (idx + 1) % self.players.len();
        while self.is_player_dead(idx) {
            idx = (idx + 1) % self.players.len();
        }

        idx
    }

    fn next_prio_player_idx(&self) -> usize {
        let mut idx = match self.priority_player_idx {
            None => {
                self.current_player_idx
            }
            Some(idx) => {
                idx
            }
        };

        idx = (idx + 1) % self.players.len();
        while self.is_player_dead(idx) {
            idx = (idx + 1) % self.players.len();
        }

        idx
    }
}

struct InformationSetOpponent {
    money: u8,
    num_influence_cards: u8,
}

// the game of coup from the perspective of a player
struct CoupInformationSet {
    current_player_idx: usize,
    num_deck_cards: usize,
    num_player_influence_cards_and_money: Vec<InformationSetOpponent>,
    // indexed by player (cards, money)
    influence_cards: Vec<Character>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum CharacterAction {
    Tax,
    // Assassinate a player by idx
    Assassinate(usize),
    // Steal from a player by idx
    Steal(usize),

    // Exchange an influence card by idx (this is the idx of the card in the current player's hand)
    Exchange(usize),
}

fn main() {
    let mut coup = Coup::new(4);
    let mut rng = thread_rng();

    for i in 0..500 {
        let mut actions = coup.gen_actions();
        let random_index = rng.gen_range(0..actions.len());
        let random_action = actions.remove(random_index);

        coup = coup.apply_action(random_action).unwrap();

        if coup.is_terminal() {
            println!("game over");
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::action::{Action, ChallengeAction, CoupAction, LoseAction, ProposableAction, ProposedAction, Prove, ProveAction};
    use crate::action::Action::{Income, Lose, Pass};
    use crate::Character::{Assassin, Duke};
    use crate::CharacterAction::Assassinate;
    use crate::{Character, Coup};

    fn find_action(game: &Coup, f: Box<dyn Fn(&Action) -> bool>) -> Action {
        let actions = game.gen_actions();
        let action = actions.iter().find(|a| f(*a));
        match action {
            None => {
                panic!("action was not found")
            }
            Some(action) => {
                action.clone()
            }
        }
    }

    #[test]
    fn double_assassinate() {
        let mut coup = Coup::new(3);

        // give p0 an assassin
        coup.players[0].influence_cards[0] = (Assassin, false);

        // give p1 no contessa
        coup.players[1].influence_cards[0] = (Duke, false);
        coup.players[1].influence_cards[1] = (Duke, false);

        // income round
        coup = coup.apply_action(find_action(&coup, Box::new(|a| *a == Income(0)))).unwrap();
        coup = coup.apply_action(find_action(&coup, Box::new(|a| *a == Income(1)))).unwrap();
        coup = coup.apply_action(find_action(&coup, Box::new(|a| *a == Income(2)))).unwrap();

        // assassinate
        coup = coup.apply_action(find_action(&coup, Box::new(|a| *a == Action::Propose(
            ProposedAction {
                proposer_player_idx: 0,
                action: ProposableAction::CharacterAction(Assassin, Assassinate(1)),
            }
        )))).unwrap();

        let challenge = ChallengeAction {
            challenger_player_idx: 1,
            challenged_player_idx: 0,
            challenged_claimed_character: Character::Assassin,
        };

        // p1 challenges assassination
        {
            let challenge = challenge.clone();
            coup = coup.apply_action(find_action(&coup, Box::new(move |a| *a == Action::Challenge(challenge.clone())))).unwrap();
        }

        // p0 wins challenge via proof
        {
            let challenge = challenge.clone();
            coup = coup.apply_action(find_action(&coup, Box::new(move |a| *a == Action::Prove(ProveAction {
                prover_player_idx: 0,
                challenger_player_idx: 1,
                action: Prove::Win(0),
                challenge: challenge.clone(),
            })))).unwrap();
        }

        // p1 loses challenge
        {
            println!("{:?}", coup.gen_actions());
            let challenge = challenge.clone();
            coup = coup.apply_action(find_action(&coup, Box::new(move |a| *a == Action::Prove(ProveAction {
                prover_player_idx: 0,
                challenger_player_idx: 1,
                action: Prove::Lose(0),
                challenge: challenge.clone(),
            })))).unwrap();
        }
        println!("{:?}", coup.gen_actions());

        coup = coup.apply_action(find_action(&coup, Box::new(|a| *a == Lose(LoseAction {
            loser_player_idx: 1,
            discarded_influence_card_idx: 1,
        })))).unwrap();
    }

    #[test]
    fn normal_assassinate() {
        let mut coup = Coup::new(3);

        // give p0 an assassin
        coup.players[0].influence_cards[0] = (Assassin, false);

        // give p1 no contessa
        coup.players[1].influence_cards[0] = (Duke, false);
        coup.players[1].influence_cards[1] = (Duke, false);

        // income round
        coup = coup.apply_action(find_action(&coup, Box::new(|a| *a == Income(0)))).unwrap();
        coup = coup.apply_action(find_action(&coup, Box::new(|a| *a == Income(1)))).unwrap();
        coup = coup.apply_action(find_action(&coup, Box::new(|a| *a == Income(2)))).unwrap();

        // assassinate
        coup = coup.apply_action(find_action(&coup, Box::new(|a| *a == Action::Propose(
            ProposedAction {
                proposer_player_idx: 0,
                action: ProposableAction::CharacterAction(Assassin, Assassinate(1)),
            }
        )))).unwrap();

        coup = coup.apply_action(find_action(&coup, Box::new(|a| *a == Pass(1)))).unwrap();
        coup = coup.apply_action(find_action(&coup, Box::new(|a| *a == Pass(2)))).unwrap();

        coup = coup.apply_action(find_action(&coup, Box::new(|a| *a == Lose(LoseAction {
            loser_player_idx: 1,
            discarded_influence_card_idx: 0,
        })))).unwrap();

        // next action should be player 1 choice
        find_action(&coup, Box::new(|a| *a == Income(1)));
    }

    #[test]
    fn next_actor() {
        let mut coup = Coup::new(4);
        assert_eq!(coup.next_prio_player_idx(), 1);

        coup.current_player_idx = 1;
        assert_eq!(coup.next_prio_player_idx(), 2);

        coup.priority_player_idx = Some(2);
        assert_eq!(coup.next_prio_player_idx(), 3);

        coup.current_player_idx = 3;
        assert_eq!(coup.next_prio_player_idx(), 3);
    }

    #[test]
    fn other_players() {
        let coup = Coup::new(4);
        assert_eq!(coup.other_player_indexes(0)[0], 1);
        assert_eq!(coup.other_player_indexes(0)[1], 2);
        assert_eq!(coup.other_player_indexes(0)[2], 3);
        assert_eq!(coup.other_player_indexes(0).len(), 3);

        assert_eq!(coup.other_player_indexes(1)[0], 2);
        assert_eq!(coup.other_player_indexes(1)[1], 3);
        assert_eq!(coup.other_player_indexes(1)[2], 0);
        assert_eq!(coup.other_player_indexes(1).len(), 3);

        let coup = Coup::new(3);
        assert_eq!(coup.other_player_indexes(1)[0], 2);
        assert_eq!(coup.other_player_indexes(1)[1], 0);
        assert_eq!(coup.other_player_indexes(1).len(), 2);
    }
}
