mod action;

use std::ops::Deref;
use rand::seq::SliceRandom;
use rand::{Rng, thread_rng};
use crate::action::Action;
use crate::Character::{Ambassador, Assassin, Captain, Contessa, Duke};

#[derive(Clone)]
enum State {
    AwaitingProposal,
    // num passes remaining
    AwaitingProposalResponse(usize),
    // blocker
    AwaitingProposalBlockResponse(usize),
    // blocker, challenger
    AwaitingChallengedBlockResponse(usize, usize),
    // challenger
    AwaitingChallengedProposalResponse(usize),
    // who's going to lose influence, and if to end the turn after
    AwaitingLoseInfluence(usize, bool),

    ResolveProposal,
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

}


#[derive(Clone)]
struct Coup {
    turn: usize,
    current_player_idx: usize,
    deck: Vec<Character>,
    players: Vec<Player>,

    state: State,
    priority_player_idx: Option<usize>,
    proposal: Option<Action>,
    proposal_blocked_with: Option<Character>,
}


impl Coup {
    fn new(num_players: u8) -> Self {
        let mut deck: Vec<Character> = CHARACTER_VARIANTS.iter()
            .flat_map(|&card| std::iter::repeat(card).take(3))
            .collect();

        let mut rng = thread_rng();
        deck.shuffle(&mut rng);

        let players = (0..num_players).map(|_| Player {
            money: 2,
            influence_cards: vec![(deck.remove(0), false), (deck.remove(0), false)],
        }).collect();

        Self {
            turn: 0,
            state: State::AwaitingProposal,
            current_player_idx: 0,
            priority_player_idx: None,
            proposal: None,
            proposal_blocked_with: None,
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
            .filter(|player_idx| self.is_player_dead(*player_idx) == false)
            .collect()
    }

    fn go_next_turn(&mut self) {
        // reset state
        self.state = State::AwaitingProposal;
        self.proposal_blocked_with = None;
        self.priority_player_idx = None;
        self.proposal = None;

        // player's turn is over
        self.turn += 1;

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

    fn player_active_influence_cards(&self, player_idx: usize) -> Vec<usize> {
        self.players[player_idx].influence_cards
            .iter()
            .enumerate()
            .filter_map(|(idx, card)| {
                if card.1 == false {
                    Some(idx)
                } else {
                    None
                }
            }).collect()
    }

    fn find_player_active_character(&self, player_idx: usize, character: Character) -> Option<usize> {
        // not revealed and is the claimed character
        self
            .players[player_idx]
            .influence_cards
            .iter()
            .position(|e| e.1 == false && e.0 == character)
    }

    fn actions(&self) -> Vec<Action> {
        let mut actions = vec![];

        match self.state {
            State::AwaitingProposal => {
                if self.players[self.current_player_idx].money >= 10 {
                    // forced coup at $10+
                    for opponent_idx in self.other_player_indexes(self.current_player_idx) {
                        actions.push(Action::Coup(self.current_player_idx, opponent_idx));
                    }
                } else {
                    // income-ing is not a proposal - it just happens
                    actions.push(Action::Income(self.current_player_idx));
                    actions.push(Action::Propose(self.current_player_idx, Box::new(Action::ForeignAid(self.current_player_idx))));
                    actions.push(Action::Propose(self.current_player_idx, Box::new(Action::Tax(self.current_player_idx))));

                    for card_idx in self.player_active_influence_cards(self.current_player_idx) {
                        actions.push(Action::Propose(self.current_player_idx, Box::new(Action::Exchange(self.current_player_idx, card_idx))));
                    }

                    for opponent_idx in self.other_player_indexes(self.current_player_idx) {
                        if self.players[self.current_player_idx].money >= 7 {
                            // coup-ing is not a proposal - it just happens
                            actions.push(Action::Coup(self.current_player_idx, opponent_idx));
                        } else if self.players[self.current_player_idx].money >= 3 {
                            actions.push(Action::Propose(self.current_player_idx, Box::new(Action::Assassinate(self.current_player_idx, opponent_idx))));
                        }

                        if self.players[opponent_idx].money > 0 {
                            actions.push(Action::Propose(self.current_player_idx, Box::new(Action::Steal(self.current_player_idx, opponent_idx))));
                        }
                    }
                }
            }
            State::AwaitingProposalResponse(_) => {
                match self.priority_player_idx {
                    Some(priority_player_idx) => {
                        if self.current_player_idx != self.priority_player_idx.unwrap() {
                            actions.push(Action::Pass(priority_player_idx));
                            match &self.proposal {
                                Some(proposal) => {
                                    // everyone can block foreign aid
                                    if let Action::ForeignAid(_) = proposal {
                                        actions.push(Action::Block(priority_player_idx, Duke));
                                    }

                                    // any character proposal action can be challenged
                                    // notably not including blocks here since they're not possible at
                                    // this state of the game
                                    match proposal {
                                        Action::Tax(_) => {
                                            actions.push(Action::Challenge(priority_player_idx));
                                        }
                                        Action::Assassinate(_, target_player_idx) => {
                                            actions.push(Action::Challenge(priority_player_idx));
                                            if *target_player_idx == self.priority_player_idx.unwrap() {
                                                actions.push(Action::Block(priority_player_idx, Contessa));
                                            }
                                        }
                                        Action::Steal(_, target_player_idx) => {
                                            actions.push(Action::Challenge(priority_player_idx));
                                            if *target_player_idx == self.priority_player_idx.unwrap() {
                                                actions.push(Action::Block(priority_player_idx, Ambassador));
                                                actions.push(Action::Block(priority_player_idx, Captain));
                                            }
                                        }
                                        Action::Exchange(_, _) => {
                                            actions.push(Action::Challenge(priority_player_idx));

                                        }
                                        _ => {}
                                    }

                                }
                                None => unreachable!("proposal must be defined at this point")
                            }
                        }
                    }
                    None => unreachable!("priority_player_idx must be defined at this point")
                }
            }
            State::AwaitingProposalBlockResponse(_) => {
                actions.push(Action::Challenge(self.current_player_idx));
                actions.push(Action::Relent(self.current_player_idx));
            }
            State::AwaitingChallengedBlockResponse(_, _) => {
                match self.priority_player_idx {
                    None => unreachable!("proposal must be defined at this point"),
                    Some(priority_player_idx) => {
                        // can lose if forced, or on purpose
                        for card_idx in self.player_active_influence_cards(priority_player_idx) {
                            actions.push(Action::Lose(priority_player_idx, card_idx, false));
                        }

                        match self.proposal_blocked_with {
                            None => unreachable!("proposal_blocked_wth must be defined at this point"),
                            Some(proposal_blocked_with) => {
                                // blocking player has the nuts, they can prove by revealing and win
                                if let Some(card_idx) = self.find_player_active_character(priority_player_idx, proposal_blocked_with) {
                                    actions.push(Action::Reveal(priority_player_idx, card_idx));
                                }
                            }
                        }
                    }
                }
            }
            State::AwaitingChallengedProposalResponse(_) => {
                // can lose if forced, or on purpose
                for card_idx in self.player_active_influence_cards(self.current_player_idx) {
                    actions.push(Action::Lose(self.current_player_idx, card_idx, true));
                }

                match &self.proposal {
                    None => unreachable!("proposal must be defined at this point"),
                    Some(proposal) => {
                        let required_character = match proposal {
                            Action::Tax(_) => Duke,
                            Action::Assassinate(_, _) => Assassin,
                            Action::Steal(_, _) => Captain,
                            Action::Exchange(_, _) => Ambassador,
                            _ => panic!("{:?} is not a blockable action", proposal),
                        };

                        if let Some(card_idx) = self.find_player_active_character(self.current_player_idx, required_character) {
                            actions.push(Action::Reveal(self.current_player_idx, card_idx));
                        }
                    }
                }
            }
            State::AwaitingLoseInfluence(loser_player_idx, end_turn) => {
                for card_idx in self.player_active_influence_cards(loser_player_idx) {
                    actions.push(Action::Lose(loser_player_idx, card_idx, end_turn));
                }
            }
            State::ResolveProposal => {
                actions.push(Action::Resolve(self.current_player_idx));
            }
        }

        actions
    }

    fn apply_action(&self, action: Action) -> Result<Coup, CoupError> {
        let mut game = self.clone();
        println!("T{}: {} | {:?} -> ${} {:?} | {:?}", self.turn, self.current_player_idx, self.priority_player_idx, self.active_player().money, self.active_player().influence_cards, action);

        match action {
            Action::Propose(_, proposed_action) => {

                // pay for assassinate proposal
                match *proposed_action {
                    Action::Assassinate(_, _) => {
                        game.players[game.current_player_idx].money -= 3;
                    }

                    _ => {}
                }

                game.proposal = Some(proposed_action.deref().clone());
                game.state = State::AwaitingProposalResponse(game.other_player_indexes(self.current_player_idx).len());
                game.priority_player_idx = Some(game.next_prio_player_idx());
            }
            Action::Income(player_idx) => {
                game.players[player_idx].money += 1;
                game.go_next_turn();
            }
            Action::Coup(_, target_player_idx) => {
                game.players[game.current_player_idx].money -= 7;
                game.state = State::AwaitingLoseInfluence(target_player_idx, true);
            }
            Action::Block(_, character) => {
                let blocking_player_idx = game.priority_player_idx.expect("priority should exist and the acting player should have priority");
                game.state = State::AwaitingProposalBlockResponse(blocking_player_idx);
                game.priority_player_idx = Some(game.current_player_idx);
                game.proposal_blocked_with = Some(character)
            }
            Action::Relent(_) => {
                game.go_next_turn();
            }
            Action::Challenge(_) => {
                match game.state {
                    State::AwaitingProposalResponse(_) => {
                        game.state = State::AwaitingChallengedProposalResponse(game.priority_player_idx.unwrap());
                        game.priority_player_idx = Some(game.current_player_idx);
                    }
                    State::AwaitingProposalBlockResponse(blocker_player_idx) => {
                        game.state = State::AwaitingChallengedBlockResponse(blocker_player_idx, game.current_player_idx);
                        game.priority_player_idx = Some(blocker_player_idx);
                    }
                    _ => unreachable!("only the proposal and block actions can be challenged")
                }
            }
            Action::Lose(loser_player_idx, card_idx, end_turn) => {
                match game.state {
                    State::AwaitingChallengedProposalResponse(_) => {
                        game.lose_influence_card(loser_player_idx, card_idx);
                        game.go_next_turn();
                    }
                    State::AwaitingChallengedBlockResponse(_, _) => {
                        game.lose_influence_card(loser_player_idx, card_idx);
                        game.priority_player_idx = Some(game.current_player_idx);
                        game.state = State::ResolveProposal;

                        if game.is_player_dead(game.current_player_idx) {
                            game.go_next_turn();
                        }
                    }
                    State::AwaitingLoseInfluence(_, _) => {
                        game.lose_influence_card(loser_player_idx, card_idx);
                        game.priority_player_idx = Some(game.current_player_idx);
                        game.state = State::ResolveProposal;

                        // loss was not a challenge loss, so it was assassinate or coup, and so turn should end
                        if end_turn {
                            game.go_next_turn();
                        }

                        if game.is_player_dead(game.current_player_idx) {
                            game.go_next_turn();
                        }
                    }
                    _ => unreachable!("can only lose if current state is awaiting lose influence")
                }
            }
            Action::Reveal(player_idx, card_idx) => {
                game.replace_influence_card(player_idx, card_idx);
                match game.state {
                    State::AwaitingChallengedBlockResponse(_, challenger_player_idx) => {
                        game.state = State::AwaitingLoseInfluence(challenger_player_idx, true);
                        game.priority_player_idx = Some(challenger_player_idx);
                    }
                    State::AwaitingChallengedProposalResponse(challenger_player_idx) => {
                        game.state = State::AwaitingLoseInfluence(challenger_player_idx, false);
                        game.priority_player_idx = Some(challenger_player_idx);
                    }
                    _ => unreachable!("can only reveal if current state is awaiting block or challenge response")
                }
            }
            Action::Pass(_) => {
                if let State::AwaitingProposalResponse(ref mut num_remaining_passers) = game.state {
                    *num_remaining_passers -= 1;

                    if *num_remaining_passers == 0 {
                        game.state = State::ResolveProposal;
                        game.priority_player_idx = Some(game.current_player_idx);
                    } else {
                        game.go_next_prio();
                    }
                } else {
                    unreachable!("should be in the awaiting proposal response phase")
                }
            }
            Action::Resolve(_) => {
                match &game.proposal {
                    None => {}
                    Some(proposal) => {
                        match proposal {
                            Action::ForeignAid(_) => {
                                game.players[game.current_player_idx].money += 2;
                                game.go_next_turn();
                            }
                            Action::Tax(_) => {
                                game.players[game.current_player_idx].money += 3;
                                game.go_next_turn();
                            }
                            Action::Assassinate(_, target_player_idx) => {
                                game.state = State::AwaitingLoseInfluence(*target_player_idx, true);
                            }
                            Action::Steal(_, target_player_idx) => {
                                game.players[game.current_player_idx].money += 2;
                                game.players[*target_player_idx].money -= 2;
                                game.go_next_turn();
                            }
                            Action::Exchange(_, card_idx) => {
                                game.replace_influence_card(game.current_player_idx, *card_idx);
                                game.go_next_turn();
                            }
                            _ => unreachable!("proposal is not actionable")
                        }
                    }
                }
            }
            _ => unreachable!("invalid action")
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

fn main() {
    let mut coup = Coup::new(4);
    let mut rng = thread_rng();

    for _ in 0..100 {
        let mut actions = coup.actions();

        if actions.is_empty() {
            println!("no actions");
            coup.actions();
            break;
        }

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
    use crate::action::Action::{Income, Lose, Pass, Assassinate, Resolve, Challenge, Reveal, Steal, Block};
    use crate::Character::{Ambassador, Assassin, Captain, Duke};
    use crate::{Coup};

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

    fn try_action(game: Coup, f: Box<dyn Fn(&Action) -> bool>) -> Coup {
        match game.apply_action(find_action(&game, f)) {
            Ok(game) => {
                game
            }
            Err(err) => {
                panic!("failed to apply action: {:?}", err)
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
        coup = try_action(coup, Box::new(|a| *a == Income(0)));
        coup = try_action(coup, Box::new(|a| *a == Income(1)));
        coup = try_action(coup, Box::new(|a| *a == Income(2)));

        // assassinate
        let assassinate_proposal = Action::Propose(0, Box::new(Assassinate(0, 1)));
        coup = try_action(coup, Box::new(move |a| *a == assassinate_proposal));

        // p1 challenges
        coup = try_action(coup, Box::new(|a| *a == Challenge(1)));

        // p0 wins challenge
        coup = try_action(coup, Box::new(|a| *a == Reveal(0, 0)));

        // p1 loses card
        coup = try_action(coup, Box::new(|a| *a == Lose(1, 0, false)));

        // p0 resolves
        coup = try_action(coup, Box::new(|a| *a == Resolve(0)));

        // p1 loses another card
        coup = try_action(coup, Box::new(|a| *a == Lose(1, 1, true)));

        // next action should be player 2 choice, p1 is dead
        find_action(&coup, Box::new(|a| *a == Income(2)));
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
        coup = try_action(coup, Box::new(|a| *a == Income(0)));
        coup = try_action(coup, Box::new(|a| *a == Income(1)));
        coup = try_action(coup, Box::new(|a| *a == Income(2)));

        assert_eq!(coup.players[0].money, 3);

        // assassinate
        let assassinate_proposal = Action::Propose(0, Box::new(Assassinate(0, 1)));
        coup = try_action(coup, Box::new(move |a| *a == assassinate_proposal));

        // should instantly be out of money
        assert_eq!(coup.players[0].money, 0);

        coup = try_action(coup, Box::new(|a| *a == Pass(1)));
        coup = try_action(coup, Box::new(|a| *a == Pass(2)));

        coup = try_action(coup, Box::new(|a| *a == Resolve(0)));

        coup = try_action(coup, Box::new(|a| *a == Lose(1, 0, true)));

        // next action should be player 1 choice
        find_action(&coup, Box::new(|a| *a == Income(1)));
    }

    #[test]
    fn test_steal() {
        let mut coup = Coup::new(3);

        // give p0 a captain
        coup.players[0].influence_cards[0] = (Captain, false);
        coup.players[0].influence_cards[1] = (Duke, false);

        // give p2 an ambassador
        coup.players[2].influence_cards[0] = (Ambassador, false);
        coup.players[2].influence_cards[1] = (Duke, false);

        // steal from p2
        let proposal = Action::Propose(0, Box::new(Steal(0, 2)));
        coup = try_action(coup, Box::new(move |a| *a == proposal));

        // p1 can't block - it's not targeting them
        coup = try_action(coup, Box::new(|a| *a == Pass(1)));
        coup = try_action(coup, Box::new(|a| *a == Pass(2)));

        coup = try_action(coup, Box::new(|a| *a == Resolve(0)));

        // next action should be player 1 choice
        find_action(&coup, Box::new(|a| *a == Income(1)));

        // players should still have the same amount of money
        assert_eq!(coup.players[0].money, 4);
        assert_eq!(coup.players[2].money, 0);
    }

    #[test]
    fn test_steal_block() {
        let mut coup = Coup::new(3);

        // give p0 a captain
        coup.players[0].influence_cards[0] = (Captain, false);
        coup.players[0].influence_cards[1] = (Duke, false);

        // give p2 an ambassador
        coup.players[2].influence_cards[0] = (Ambassador, false);
        coup.players[2].influence_cards[1] = (Duke, false);

        // steal from p2
        let proposal = Action::Propose(0, Box::new(Steal(0, 2)));
        coup = try_action(coup, Box::new(move |a| *a == proposal));

        // p1 can't block - it's not targeting them
        coup = try_action(coup, Box::new(|a| *a == Pass(1)));

        // p2 blocks
        coup = try_action(coup, Box::new(|a| *a == Block(2, Ambassador)));

        // p0 challenges
        coup = try_action(coup, Box::new(|a| *a == Challenge(0)));

        // p2 reveals & wins
        coup = try_action(coup, Box::new(|a| *a == Reveal(2, 0)));

        // p0 loses a card, and the game ends
        coup = try_action(coup, Box::new(|a| *a == Lose(0, 0, true)));

        // next action should be player 1 choice
        find_action(&coup, Box::new(|a| *a == Income(1)));

        // players should still have the same amount of money
        assert_eq!(coup.players[0].money, 2);
        assert_eq!(coup.players[2].money, 2);
    }

    #[test]
    fn test_coup() {
        let mut coup = Coup::new(3);

        // give p0 $10
        coup.players[0].money = 10;

        // coup should be forced at $10+ (coup p1 and p2)
        assert_eq!(coup.actions().len(), 2);

        // coup from p1
        coup = try_action(coup, Box::new(|a| *a == Action::Coup(0, 1)));

        println!("{:?}", coup.actions());

        coup = try_action(coup, Box::new(|a| *a == Lose(1, 0, true)));

        // next action should be player 1 choice
        find_action(&coup, Box::new(|a| *a == Income(1)));

        // players should still have the same amount of money
        assert_eq!(coup.players[0].money, 3);
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
