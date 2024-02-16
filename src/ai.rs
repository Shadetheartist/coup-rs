// information set monte carlo tree search

use std::thread;
use rand::{Rng, thread_rng};
use crate::action::Action;
use crate::Coup;

fn simulate<R: Rng + Sized>(game: &Coup, rng: &mut R) -> usize {
    let mut game = game.clone();

    loop {
        let mut actions = game.actions();

        let random_index = rng.gen_range(0..actions.len());
        let random_action = actions.remove(random_index);

        game = game.apply_action(random_action, rng).unwrap();

        if let Some(winner) = game.winner() {
            return winner;
        }
    }
}

fn ismcts<R: Rng + Sized>(game: &Coup, rng: &mut R, num_determinizations: usize, num_simulations: usize) -> Action {

    // actions should be the same between the determinization and the current game
    let actions = game.actions();

    let mut determinization_scores: Vec<Vec<Vec<f32>>> = Vec::new();


    for determinization_count in 0..num_determinizations {

        let game = game.determine(rng, game.current_player_idx);

        let mut action_scores: Vec<Vec<f32>> = actions.iter().map(|_| vec![]).collect();

        for (action_idx, action) in actions.iter().enumerate() {
            let game_after_action = game.apply_action(action.clone(), rng).unwrap();

            let mut scores: Vec<f32> = game.players.iter().map(|_| 0f32).collect();
            for simulation_count in 0..num_simulations {
                let winner_player_idx = simulate(&game_after_action, rng);
                scores[winner_player_idx] += 1f32;
            }

            let max = scores.iter().fold(0f32, |sum, &val| if sum > val { sum } else { val });
            let normalized: Vec<f32> = scores.iter().map(|&n| n / max).collect();

            action_scores[action_idx] = normalized;
        }

        determinization_scores.push(action_scores);
    }

    let avg_scores: Vec<Vec<f32>> = actions
        .iter()
        .enumerate()
        .map(|(action_idx, _)| {
            let player_scores: Vec<f32> = game.players.iter().map(|_| 0f32).collect();
            determinization_scores
                .iter()
                .fold(player_scores, |sum, val| {
                    sum.iter().zip(&val[action_idx]).map(|(a, b)| a + (b / num_determinizations as f32)).collect()
                })
        }).collect();

    let mut sorted = avg_scores.iter().enumerate().collect::<Vec<(usize, &Vec<f32>)>>();

    sorted.sort_by(|a, b| {
        b.1[game.current_player_idx].partial_cmp(&a.1[game.current_player_idx]).unwrap()
    });

    actions[sorted[0].0].clone()
}


#[cfg(test)]
mod tests {
    use rand::thread_rng;
    use crate::ai::ismcts;
    use crate::Coup;

    #[test]
    fn eeeeeeeeee() {
        let mut game = Coup::new(4);

        let mut rng = thread_rng();

        loop {
            let ai_selected_action = ismcts(&game, &mut rng, 100, 100);
            println!("AI selected action {:?}", ai_selected_action);

            game = game.apply_action(ai_selected_action, &mut rng).unwrap();

            if let Some(winner) = game.winner() {
                println!("game over, winner is player {winner}");
                break
            }
        }

    }
}