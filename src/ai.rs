use mcts::{Mcts, Outcome};
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
            None
        }
    }

    fn current_player(&self) -> usize {
        self.current_player_idx
    }

    fn players(&self) -> Vec<usize> {
        self.players.iter().enumerate().map(|(idx, _)| idx).collect()
    }
}

impl <R: Rng + Sized> mcts::Determinable<usize, Action, Coup, R> for Coup {
    fn determine(&self, rng: &mut R, perspective_player: usize) -> Coup {
        self.determine(rng, perspective_player)
    }
}



#[cfg(test)]
mod tests {
    use rand::{SeedableRng};
    use crate::Coup;

    #[test]
    fn run_test_simulation() {
        let mut rng = rand_pcg::Pcg32::seed_from_u64(0);
        let coup = Coup::new(4, &mut rng);

        let a = mcts::ismcts(&coup, &rng, 4, 4);
        print!("{:?}",a);
    }
}