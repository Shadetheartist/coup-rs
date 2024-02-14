use rand::seq::SliceRandom;
use rand::thread_rng;
use Character::Assassin;
use crate::Character::{Ambassador, Captain, Contessa, Duke};
use crate::CharacterAction::{Assassinate, Exchange, Steal, Tax};
use crate::CoupError::InvalidAction;

#[derive(Clone)]
struct Proposal {
    proposer_player_idx: usize,
    action: Action,
    response: Option<ProposalResponses>,
}

#[derive(Clone)]
struct Block {
    blocker_player_idx: usize,
    response: Option<BlockResponses>,
}

#[derive(Clone)]
struct Challenge {
    challenger_player_idx: usize,
    response: ChallengeResponses,
}

#[derive(Clone)]
struct Reveal {
    challenger_player_idx: usize,
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


#[derive(Copy, Clone, Debug, PartialEq)]
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
    influence_cards: Vec<Character>,
}

#[derive(Debug)]
enum CoupError {
    InvalidAction(Action),
}


#[derive(Clone)]
struct Coup {
    turn: usize,

    current_player_idx: usize,

    priority_player_idx: Option<usize>,
    proposal: Option<Proposal>,

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
            influence_cards: vec![deck.remove(0), deck.remove(0)],
        }).collect();

        Self {
            turn: 0,
            current_player_idx: 0,
            priority_player_idx: None,
            proposal: None,
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

        // go to next player
        self.current_player_idx = (self.current_player_idx + 1) % self.players.len();
    }

    fn replace_influence_card(&mut self, player_idx: usize, card_idx: usize) {
        let card = self.players[player_idx].influence_cards.remove(card_idx);
        self.deck.push(card);

        let mut rng = thread_rng();
        self.deck.shuffle(&mut rng);

        self.players[player_idx].influence_cards.push(self.deck.remove(0));
    }

    fn gen_actions(&self) -> Vec<Action> {
        let mut actions = vec![];

        fn add_proposal_actions(game: &Coup, actions: &mut Vec<Action>) {
            // forced coup
            if game.players[game.current_player_idx].money >= 10 {
                for opponent_idx in game.other_player_indexes(game.current_player_idx) {
                    actions.push(Action::Coup(opponent_idx));
                }
            } else {
                actions.push(Action::Income);
                actions.push(Action::ForeignAid);
                actions.push(Action::CharacterAction(Duke, Tax));

                for card_idx in 0..game.players[game.current_player_idx].influence_cards.len() {
                    actions.push(Action::CharacterAction(Ambassador, Exchange(card_idx)));
                }

                for opponent_idx in game.other_player_indexes(game.current_player_idx) {
                    if game.players[game.current_player_idx].money >= 7 {
                        actions.push(Action::Coup(opponent_idx));
                    } else if game.players[game.current_player_idx].money >= 3 {
                        actions.push(Action::CharacterAction(Assassin, Assassinate(opponent_idx)));
                    }
                    if game.players[opponent_idx].money > 0 {
                        actions.push(Action::CharacterAction(Captain, Steal(opponent_idx)));
                    }
                }
            }
        }

        fn add_proposal_reactions(game: &Coup, actions: &mut Vec<Action>, proposal: &Proposal){
            match proposal.action {
                Action::CharacterAction(character, character_action) => {
                    match character_action {
                        Assassinate(target_player) => {
                            // can only block assassination of yourself
                            if target_player == game.priority_player_idx.expect("this should be a priority player") {
                                actions.push(Action::Block(Contessa));
                            }
                        }
                        Steal(target_player) => {
                            // can only block stealing of yourself
                            if target_player == game.priority_player_idx.expect("this should be a priority player") {
                                actions.push(Action::Block(Captain));
                                actions.push(Action::Block(Ambassador));
                            }
                        }
                        _ => {}
                    }
                    actions.push(Action::Challenge(character));
                }
                _ => {}
            }
            actions.push(Action::Pass);
        }

        fn handle_lose(game: &Coup, actions: &mut Vec<Action>, lose: &Lose){
            // optionally, the player can lose the challenge and lose an influence card
            for card_idx in 0..game.players[lose.loser_player_idx].influence_cards.len() {
                actions.push(Action::Lose(card_idx));
            }
        }

        fn handle_challenge(game: &Coup, actions: &mut Vec<Action>, challenge: &Challenge){
            match &challenge.response {
                ChallengeResponses::Reveal(reveal) => {
                    match &reveal.response {
                        RevealResponses::Lose(lose) => handle_lose(&game, actions, lose)
                    }
                }
                ChallengeResponses::Lose(lose) => handle_lose(&game, actions, lose)
            }
        }


        match &self.proposal {
            None => {add_proposal_actions(&self, &mut actions);}
            Some(proposal) => {
                match &proposal.response {
                    None => {
                        add_proposal_reactions(&self, &mut actions, proposal);
                    }
                    Some(proposal_response) => {
                        match proposal_response {
                            ProposalResponses::Block(block_response) => {
                                match &block_response.response {
                                    None => {} // relent and get nothing
                                    Some(block_response) => {
                                        match block_response {
                                            BlockResponses::Challenge(challenge) => {
                                                handle_challenge(&self, &mut actions, challenge)
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

    fn apply_action(&self, action: &Action) -> Result<Coup, CoupError> {
        let mut game = self.clone();

        Ok(game)
    }

    fn next_actor(&self) -> usize {
        let idx = match self.priority_player_idx {
            None => {
                self.current_player_idx
            }
            Some(idx) => {
                idx
            }
        };

        (idx + 1) % self.players.len()
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

#[derive(Copy, Clone, Debug)]
enum CharacterAction {
    Tax,
    // Assassinate a player by idx
    Assassinate(usize),
    // Steal from a player by idx
    Steal(usize),

    // Exchange an influence card by idx (this is the idx of the card in the current player's hand)
    Exchange(usize),
}

#[derive(Copy, Clone, Debug)]
enum Action {
    Income,
    ForeignAid,
    Coup(usize), // coup a player by idx

    // a player claims to be a character and take a character action
    CharacterAction(Character, CharacterAction),

    // a player challenges that another player has this character
    Challenge(Character),

    // player must claim to be a character to block
    Block(Character),

    Pass,
    // player does not issue a challenge
    Lose(usize),
    // if the player loses a challenge, they remove this influence card
    Reveal(usize), // if the player reveals, they win the challenge
}

fn main() {
    let mut coup = Coup::new(4);
    let mut rng = thread_rng();

    for i in 0..50 {
        let actions = coup.gen_actions();
        let random_action = actions.choose(&mut rng);
        match random_action {
            None => {
                println!("No actions generated");
                break;
            }
            Some(action) => {
                println!("{} | {:?} -> ${} {:?} | {:?}", coup.current_player_idx, coup.priority_player_idx, coup.active_player().money, coup.active_player().influence_cards, random_action);
                coup = coup.apply_action(action).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Coup;

    #[test]
    fn next_actor() {
        let mut coup = Coup::new(4);
        assert_eq!(coup.next_actor(), 1);

        coup.current_player_idx = 1;
        assert_eq!(coup.next_actor(), 2);

        coup.priority_player_idx = Some(2);
        assert_eq!(coup.next_actor(), 3);

        coup.current_player_idx = 3;
        assert_eq!(coup.next_actor(), 3);
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
