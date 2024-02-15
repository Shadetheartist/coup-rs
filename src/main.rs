mod action;
mod coup;

use rand::seq::SliceRandom;
use rand::{Rng, thread_rng};
use crate::action::Action;
#[derive(Clone)]
enum State {
    AwaitingProposal,
    AwaitingProposalResponse,
    AwaitingProposalBlockResponse,
    AwaitingBlockChallengeResponse,
    AwaitingChallengedProposalResponse,
    AwaitingLoseInfluence(usize),
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
    Character::Assassin,
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
struct Coup {
    turn: usize,
    state: State,

    current_player_idx: usize,

    priority_player_idx: Option<usize>,
    proposal: Option<Action>,
    num_remaining_passers: Option<usize>,

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
            state: State::AwaitingProposal,
            current_player_idx: 0,
            priority_player_idx: None,
            proposal: None,
            num_remaining_passers: None,
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

    fn actions(&self) -> Vec<Action> {
        let mut actions = vec![];

        match self.state {
            State::AwaitingProposal => {}
            State::AwaitingProposalResponse => {}
            State::AwaitingProposalBlockResponse => {}
            State::AwaitingBlockChallengeResponse => {}
            State::AwaitingChallengedProposalResponse => {}
            State::AwaitingLoseInfluence(_) => {

            }
        }

        actions
    }

    fn apply_action(&self, action: Action) -> Result<Coup, CoupError> {
        let mut game = self.clone();

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
        let mut actions = coup.actions();
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
    use crate::action::{Action};
    use crate::action::Action::{Income, Lose, Pass};
    use crate::Character::{Assassin, Duke};
    use crate::CharacterAction::Assassinate;
    use crate::{Character, Coup};

    fn find_action(game: &Coup, f: Box<dyn Fn(&Action) -> bool>) -> Action {
        let actions = game.actions();
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
            println!("{:?}", coup.actions());
            let challenge = challenge.clone();
            coup = coup.apply_action(find_action(&coup, Box::new(move |a| *a == Action::Prove(ProveAction {
                prover_player_idx: 0,
                challenger_player_idx: 1,
                action: Prove::Lose(0),
                challenge: challenge.clone(),
            })))).unwrap();
        }
        println!("{:?}", coup.actions());

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
