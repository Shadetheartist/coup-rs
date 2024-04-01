use ai::{Mcts, Outcome};
use rand::Rng;
use crate::{Action, Coup, CoupError};

impl Mcts<usize, Action> for Coup {
    type Error = CoupError;

    fn actions(&self) -> Vec<Action> {
        self.actions()
    }

    fn apply_action<R: Rng + Sized>(&self, action: Action, rng: &mut R) -> Result<Self, Self::Error> where Self: Sized {
        self.apply_action(action, rng)
    }

    fn outcome(&self) -> Option<Outcome<usize>> {
        if let Some(winner_player_idx) = self.winner() {
            Some(Outcome::Winner(winner_player_idx))
        } else {
            if self.turn > 100 {
                Some(Outcome::Escape("Game has taken too many turns".to_string()))
            } else {
                None
            }
        }
    }

    fn current_player(&self) -> usize {
        self.current_player_idx
    }

    fn players(&self) -> Vec<usize> {
        self.players.iter().enumerate().map(|(idx, _)| idx).collect()
    }
}

impl ai::Determinable<usize, Action, Coup> for Coup {
    fn determine<R: Rng>(&self, rng: &mut R, perspective_player: usize) -> Coup {
        self.determine(rng, perspective_player)
    }
}

impl ai::Initializer<usize, Action, Coup> for Coup {
    fn initialize<R: Rng + Sized>(rng: &mut R) -> Coup {
        Coup::new(4, rng)
    }
}


#[cfg(test)]
mod tests {
    use ai::{Mcts, Outcome};
    use rand::{SeedableRng};
    use crate::Coup;

    #[test]
    fn run_test_simulation() {
        let mut rng = rand_pcg::Pcg32::seed_from_u64(0);

        let mut coup = Coup::new(4, &mut rng);

        while coup.outcome().is_none() {
            let a = ai::mcts(&coup, &mut rng, 100);
            coup = coup.apply_action(a, &mut rng).unwrap();
        }

        match coup.outcome().unwrap() {
            Outcome::Winner(player_idx) => println!("Simulation complete - winner {player_idx}"),
            Outcome::Winners(_) => {}
            Outcome::Escape(reason) => println!("Simulation complete - escaped: {reason}")
        }
    }
}