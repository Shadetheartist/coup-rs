use rand::seq::SliceRandom;
use rand::thread_rng;
use Character::Assassin;
use crate::Character::{Ambassador, Captain, Contessa, Duke};
use crate::CharacterAction::{Assassinate, Exchange, Steal, Tax};
use crate::CoupError::InvalidAction;

/*
this is all dogshit. i need a tree or something for the weirdly complicated

propose  -> block       -> lose
                        -> challenge -> lose
                                     -> reveal -> lose

         -> challenge   -> lose
                        -> reveal -> lose
*/


enum Phase {
    Propose(usize, Character, CharacterAction),
    Block(Box<Phase>, usize, Character),
    Challenge(Box<Phase>, usize),
    Reveal(Box<Phase>, usize, usize),
    Lose(Box<Phase>, usize, usize),
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
struct StackFrame {
    action: Action,
    remaining_passers: usize,
    actor: usize,
    succeeded: bool,
}

#[derive(Clone)]
struct Coup {
    turn: usize,

    current_player_idx: usize,

    priority_player_idx: Option<usize>,
    stack: Vec<StackFrame>,

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
            stack: vec![],
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

        // clear the stack
        self.stack.clear();

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
        let game = &self;

        if game.stack.len() == 0 { // initial proposal - nothing on stack
            // forced coup
            if game.players[self.current_player_idx].money >= 10 {
                for opponent_idx in self.other_player_indexes(self.current_player_idx) {
                    actions.push(Action::Coup(opponent_idx));
                }
                return actions;
            }

            actions.push(Action::Income);
            actions.push(Action::ForeignAid);
            actions.push(Action::CharacterAction(Duke, Tax));

            for card_idx in 0..self.players[self.current_player_idx].influence_cards.len() {
                actions.push(Action::CharacterAction(Ambassador, Exchange(card_idx)));
            }

            for opponent_idx in self.other_player_indexes(self.current_player_idx) {
                if game.players[self.current_player_idx].money >= 7 {
                    actions.push(Action::Coup(opponent_idx));
                } else if game.players[self.current_player_idx].money >= 3 {
                    actions.push(Action::CharacterAction(Assassin, Assassinate(opponent_idx)));
                }
                if game.players[opponent_idx].money > 0 {
                    actions.push(Action::CharacterAction(Captain, Steal(opponent_idx)));
                }
            }
        } else if game.stack.len() == 1 { // challenge character or block of proposal - 1 on stack
            let proposed_action = &game.stack[0].action;

            match proposed_action {
                Action::ForeignAid => {
                    // anyone can block foreign aid
                    actions.push(Action::Block(Duke));
                }
                Action::Coup(target_player) => {
                    // coup is forced
                    for card_idx in 0..game.players[*target_player].influence_cards.len() {
                        actions.clear();
                        actions.push(Action::Lose(card_idx));
                        return actions;
                    }
                }
                Action::CharacterAction(character, character_action) => {
                    match character_action {
                        Assassinate(target_player) => {
                            // assassination already succeeded - target just loses a card
                            if game.stack[0].succeeded {
                                for card_idx in 0..game.players[*target_player].influence_cards.len() {
                                    actions.clear();
                                    actions.push(Action::Lose(card_idx));
                                    return actions;
                                }
                            }

                            // can only block assassination of yourself
                            if *target_player == self.priority_player_idx.expect("this should be a priority player") {
                                actions.push(Action::Block(Contessa));
                            }
                        }
                        Steal(target_player) => {
                            // can only block stealing of yourself
                            if *target_player == self.priority_player_idx.expect("this should be a priority player") {
                                actions.push(Action::Block(Captain));
                                actions.push(Action::Block(Ambassador));
                            }
                        }
                        _ => {}
                    }

                    actions.push(Action::Challenge(*character));
                }
                _ => {}
            }

            actions.push(Action::Pass);
        } else if game.stack.len() == 2 {
            // proposal challenged or blocked - 2 on stack
            let counter_action = game.stack.last().unwrap().action;
            match counter_action {
                Action::Challenge(_) => {
                    // if the player has the card
                    let required_character = {
                        if let Action::CharacterAction(character, _) = game.stack[0].action {
                            character
                        } else {
                            panic!("weird action");
                        }
                    };

                    let required_character_index = {
                        game
                            .players[game.current_player_idx]
                            .influence_cards
                            .iter()
                            .position(|c| *c == required_character)
                    };

                    // the player can reveal the card to win the challenge
                    if let Some(required_character_index) = required_character_index {
                        actions.push(Action::Reveal(required_character_index))
                    }

                    // optionally, the player can lose the challenge and lose an influence card
                    for card_idx in 0..game.players[game.current_player_idx].influence_cards.len() {
                        actions.push(Action::Lose(card_idx));
                    }
                }
                Action::Block(claimed_blocking_character) => {
                    // can allow the block without trouble
                    actions.push(Action::Pass);

                    //or can challenge the block
                    actions.push(Action::Challenge(claimed_blocking_character))
                }
                _ => {}
            }
        } else if game.stack.len() == 3 {

            // take a look at if we challenged or blocked
            match game.stack[1].action {
                Action::Challenge(_) => {
                    // challenge failed, they must have revealed - 3 on stack
                    for card_idx in 0..game.players[game.priority_player_idx.unwrap()].influence_cards.len() {
                        actions.push(Action::Lose(card_idx));
                    }
                }
                Action::Block(required_character) => {
                    // block challenged - 3 on stack
                    let required_character_index = {
                        game
                            .players[game.priority_player_idx.unwrap()]
                            .influence_cards
                            .iter()
                            .position(|c| *c == required_character)
                    };

                    // the player can reveal the card to win the challenge
                    if let Some(required_character_index) = required_character_index {
                        actions.push(Action::Reveal(required_character_index))
                    }

                    // otherwise, the player can lose the challenge and lose an influence card
                    for card_idx in 0..game.players[game.priority_player_idx.unwrap()].influence_cards.len() {
                        actions.push(Action::Lose(card_idx));
                    }
                }
                _ => {
                    panic!("should have stack[1] as a block or challenge action");
                }
            }
        } else if game.stack.len() == 4 {
            // block revealed and the current player loses
            for card_idx in 0..game.players[game.current_player_idx].influence_cards.len() {
                actions.push(Action::Lose(card_idx));
            }
        }

        actions
    }

    fn apply_action(&self, action: &Action) -> Result<Coup, CoupError> {
        let mut game = self.clone();

        if game.stack.len() == 0 {
            // current player proposes action
            let player = &mut game.players[game.current_player_idx];

            // pay for all actions up-front
            match action {
                // these actions are proposed, and then a round of responses occur
                Action::CharacterAction(_, Assassinate(_)) => {
                    assert!(player.money >= 3);
                    player.money -= 3;
                }
                Action::Coup(_) => {
                    assert!(player.money >= 7);
                    player.money -= 7;
                }
                _ => {}
            }

            // propose the action
            match action {
                Action::Income => {
                    // income cannot be responded to
                    player.money += 1;

                    // insta turn over
                    game.go_next_turn();
                }

                // these actions are proposed, and then a round of responses occur
                Action::CharacterAction(_, _) |
                Action::ForeignAid |
                Action::Coup(_) => {
                    game.stack.push(StackFrame {
                        action: action.clone(),
                        remaining_passers: game.players.len() - 1,
                        actor: self.current_player_idx,
                        succeeded: false
                    });
                    game.priority_player_idx = Some(self.next_actor());
                }
                _ => {
                    return Err(InvalidAction(action.clone()));
                }
            }
        } else if game.stack.len() == 1 {
            // player has made a proposal, other players may be challenging or blocking

            // any responding players may issue challenge or block or pass

            match action {
                Action::Pass => {
                    let mut proposal_frame = &mut game.stack[0];
                    proposal_frame.remaining_passers -= 1;

                    let mut prio = game.priority_player_idx.as_mut().expect("someone should have prio");
                    *prio = self.next_actor();
                }
                Action::Challenge(_) |
                Action::Block(_) => {
                    game.stack.push(StackFrame {
                        action: action.clone(),
                        remaining_passers: game.players.len() - 1,
                        actor: game.priority_player_idx.unwrap(),
                        succeeded: false
                    });
                    game.priority_player_idx = Some(self.current_player_idx);
                }
                _ => {
                    return Err(InvalidAction(action.clone()));
                }
            }
        } else if game.stack.len() == 2 {
            // proposal challenged
            // - player can lose an influence or reveal (and the other player loses influence)

            // or player was blocked
            // - player can challenge the block or accept the fail

            match game.stack[1].action {
                Action::Challenge(_) => {
                    // if challenged, the current player either loses or reveals
                    match action {
                        Action::Lose(idx) => {
                            game.players[game.current_player_idx].influence_cards.remove(*idx);
                            game.go_next_turn();
                        }
                        Action::Reveal(card_idx) => {
                            // reveal-er gets to replace their card
                            game.replace_influence_card(game.current_player_idx, *card_idx);
                            game.stack.push(StackFrame {
                                action: action.clone(),
                                remaining_passers: game.players.len() - 1,
                                actor: self.current_player_idx,
                                succeeded: false
                            });
                            // give prio back to the challenger, so they can lose a card
                            game.priority_player_idx = Some(game.stack[1].actor);
                        }
                        _ => {
                            return Err(InvalidAction(action.clone()));
                        }
                    }
                }
                Action::Block(_) => {
                    // if blocked, the current player either passes or challenges the block
                    match action {
                        Action::Pass => {
                            game.go_next_turn();
                        }
                        Action::Challenge(_) => {
                            game.stack.push(StackFrame {
                                action: action.clone(),
                                remaining_passers: game.players.len() - 1,
                                actor: self.current_player_idx,
                                succeeded: false
                            });

                            // give prio back to the challenger, so they can respond
                            game.priority_player_idx = Some(game.stack[1].actor);
                        }
                        _ => {
                            return Err(InvalidAction(action.clone()));
                        }
                    }
                }
                _ => {
                    panic!("weird state")
                }
            }
        } else if game.stack.len() == 3 {
            // block challenged - 3 on stack

            match action {
                Action::Lose(idx) => {
                    game.players[game.priority_player_idx.unwrap()].influence_cards.remove(*idx);
                    if game.players[game.priority_player_idx.unwrap()].influence_cards.len() == 0 {
                        game.players.remove(game.priority_player_idx.unwrap());
                    }

                    game.stack.drain(1..);
                    game.stack[0].remaining_passers = 0;

                }
                Action::Reveal(card_idx) => {
                    // reveal-er gets to replace their card
                    game.replace_influence_card(game.current_player_idx, *card_idx);

                    game.stack.push(StackFrame {
                        action: action.clone(),
                        remaining_passers: game.players.len() - 1,
                        actor: game.priority_player_idx.unwrap(),
                        succeeded: false
                    });
                    game.priority_player_idx = Some(self.current_player_idx);
                }
                _ => {
                    return Err(InvalidAction(action.clone()));
                }
            }
        } else if game.stack.len() == 4 {
            match action {
                Action::Lose(idx) => {
                    game.players[game.priority_player_idx.unwrap()].influence_cards.remove(*idx);
                    if game.players[game.priority_player_idx.unwrap()].influence_cards.len() == 0 {
                        game.players.remove(game.priority_player_idx.unwrap());
                    }

                    game.go_next_turn();
                }
                _ => {
                    return Err(InvalidAction(action.clone()));
                }
            }
        }

        // if remaining passers of the top-frame is 0
        // then we should be ready to proceed with the proposed action
        // player has already paid
        if game.stack.len() > 0 && game.stack[game.stack.len() - 1].remaining_passers == 0 {
            let player = &mut game.players[game.current_player_idx];
            match game.stack[0].action {
                Action::ForeignAid => {
                    player.money += 2;
                }
                Action::Coup(_) => {
                    game.stack.push(StackFrame {
                        action: action.clone(),
                        remaining_passers: game.players.len() - 1,
                        actor: game.current_player_idx,
                        succeeded: true
                    });
                }
                Action::CharacterAction(_, character_action) => {
                    match character_action {
                        Tax => {
                            player.money += 3;
                        }
                        Assassinate(target_player_idx) => {
                            game.stack.push(StackFrame {
                                action: action.clone(),
                                remaining_passers: game.players.len() - 1,
                                actor: game.current_player_idx,
                                succeeded: true
                            });
                            game.priority_player_idx = Some(target_player_idx);
                        }
                        Steal(other_player_idx) => {
                            player.money += 2;
                            if game.players[other_player_idx].money == 1 {
                                game.players[other_player_idx].money -= 1;
                            } else {
                                game.players[other_player_idx].money -= 2;
                            }
                        }
                        Exchange(card_idx) => {
                            game.replace_influence_card(game.current_player_idx, card_idx);
                        }
                    }
                }
                _ => {
                    panic!("strange stack frame action")
                }
            }

            game.go_next_turn();
        }

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
                println!("{} | [{}] {:?} -> ${} {:?} | {:?}", coup.current_player_idx, coup.stack.len(), coup.priority_player_idx, coup.active_player().money, coup.active_player().influence_cards, random_action);
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
