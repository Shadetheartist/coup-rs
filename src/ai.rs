// information set monte carlo tree search

use std::sync::{Arc, Mutex};
use std::thread;
use rand::{Rng, thread_rng};
use crate::action::Action;
use crate::Coup;

fn simulate<R: Rng + Sized>(game: &Coup, rng: &mut R) -> usize {
    let mut game = game.clone();

    loop {
        let mut actions = game.actions();

        if actions.len() == 0 {
            // not sure why this can occur
            if let Some(winner) = game.winner() {
                return winner;
            }
            let mut actions = game.actions();
        }

        let random_index = rng.gen_range(0..actions.len());
        let random_action = actions.remove(random_index);

        game = game.apply_action(random_action, rng).unwrap();

        if let Some(winner) = game.winner() {
            return winner;
        }

        if game.turn > 1000 {
            println!("failed to simulate in a reasonable amount of turns - default winner 0");
            return 0;
        }
    }

}

fn ismcts<R: Rng + Sized + Clone + std::marker::Send>(game: &Coup, rng: &mut R, num_determinizations: usize, num_simulations: usize) -> Action {

    // actions should be the same between the determinization and the current game
    let actions = game.actions();

    let mut determinization_scores: Arc<Mutex<Vec<Vec<Vec<f32>>>>> = Arc::new(Mutex::new(Vec::new()));

    thread::scope(|scope| {
        for determinization_count in 0..num_determinizations {
            {
                let mut rng = rng.clone();
                let actions = actions.clone();
                let determinization_scores_ref_clone = determinization_scores.clone();

                scope.spawn(move || {
                    let game = game.determine(&mut rng, game.current_player_idx);
                    let mut action_scores: Vec<Vec<f32>> = actions.iter().map(|_| vec![]).collect();

                    for (action_idx, action) in actions.iter().enumerate() {
                        let game_after_action = game.apply_action(action.clone(), &mut rng).unwrap();

                        let mut scores: Vec<f32> = game.players.iter().map(|_| 0f32).collect();
                        for simulation_count in 0..num_simulations {
                            let winner_player_idx = simulate(&game_after_action, &mut rng);
                            scores[winner_player_idx] += 1f32;
                        }

                        let max = scores.iter().fold(0f32, |sum, &val| if sum > val { sum } else { val });
                        let normalized: Vec<f32> = scores.iter().map(|&n| n / max).collect();

                        action_scores[action_idx] = normalized;
                    }

                    determinization_scores_ref_clone.lock().unwrap().push(action_scores);
                });

            }
        }
    });

    let avg_scores: Vec<Vec<f32>> = actions
        .iter()
        .enumerate()
        .map(|(action_idx, _)| {
            let player_scores: Vec<f32> = game.players.iter().map(|_| 0f32).collect();
            determinization_scores.lock().unwrap()
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

#[derive(Clone)]
struct Turn {
    turn: usize,
    player_idx: usize,
    action: Action,
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use csv::Writer;
    use rand::{Rng, SeedableRng};
    use rand_pcg::Pcg64;
    use crate::action::Action;

    use crate::ai::{ismcts, Turn};
    use crate::Coup;

    #[test]
    fn run_test_simulation() {

        let base_determinations = 1;
        let base_simulations = 1;
        let mut ai_strengths: Vec<(usize, usize)> = (0..6).map(|_|(base_determinations, base_simulations)).collect();
        let mut results: Vec<(Vec<(usize, usize)>, Vec<usize>)> = Vec::new();
        let mut behaviors: HashMap<Vec<(usize, usize)>, Vec<Vec<Turn>>> = HashMap::new();
        let num_batches = 5;
        let num_sims = 100;

        for batch_count in 0..num_batches {

            // boost all ai strengths
            //let ai_strengths: Vec<(usize, usize)> = ai_strengths.iter().map(|str| {
            //    (str.0, str.1 + (batch_count as usize) * 10)
            //}).collect();

            ai_strengths[0].0 += 5;
            ai_strengths[0].0 = 10;

            let mut sim_results: Vec<usize> = ai_strengths.iter().map(|_| 0).collect();

            for simulation_count in 0..num_sims {

                let mut turn_batch: Vec<Turn> = Vec::new();
                let seed: u64 = 12345 + batch_count + simulation_count * batch_count;
                let mut rng = Pcg64::seed_from_u64(seed);

                let mut game = Coup::new(ai_strengths.len() as u8);

                loop {
                    let strength = ai_strengths[game.current_player_idx];

                    let ai_selected_action = ismcts(&game, &mut rng, strength.0, strength.1);
                    println!("AI selected action {:?}", ai_selected_action);

                    match ai_selected_action {
                        Action::Propose(_, _) |
                        Action::Income(_) |
                        Action::Challenge(_) |
                        Action::Block(_, _) |
                        Action::Resolve(_) |
                        Action::Coup(_, _) => {
                            turn_batch.push(Turn {
                                turn: game.turn,
                                player_idx: game.current_player_idx,
                                action: ai_selected_action.clone(),
                            });
                        }
                        _ => {}
                    }

                    game = game.apply_action(ai_selected_action, &mut rng).unwrap();

                    if let Some(winner) = game.winner() {
                        println!("game over, winner is player {winner}");
                        sim_results[winner] += 1;

                        if behaviors.contains_key(&ai_strengths) {
                            behaviors.get_mut(&ai_strengths).unwrap().push(turn_batch.clone());
                        } else {
                            behaviors.insert(ai_strengths.clone(), vec![turn_batch.clone()]);
                        }

                        break
                    }
                }
            }



            results.push((ai_strengths.clone(), sim_results.clone()))
        }

        println!("results {:?}", results);

        let mut wtr = Writer::from_path("results.csv").unwrap();
        wtr.write_record(&[
            "p0 determinations", "p0 simulations",
            "p1 determinations", "p1 simulations",
            "p2 determinations", "p2 simulations",
            "p3 determinations", "p3 simulations",
            "p4 determinations", "p4 simulations",
            "p5 determinations", "p5 simulations",

            "sims",
            "p0 wins",
            "p1 wins",
            "p2 wins",
            "p3 wins",
            "p4 wins",
            "p5 wins",

        ]).unwrap();

        for r in results {
            wtr.write_record(&[
                format!("{}", r.0[0].0), format!("{}", r.0[0].1),
                format!("{}", r.0[1].0), format!("{}", r.0[1].1),
                format!("{}", r.0[2].0), format!("{}", r.0[2].1),
                format!("{}", r.0[3].0), format!("{}", r.0[3].1),
                format!("{}", r.0[4].0), format!("{}", r.0[4].1),
                format!("{}", r.0[5].0), format!("{}", r.0[5].1),

                format!("{}", num_sims),
                format!("{}", r.1[0]),
                format!("{}", r.1[1]),
                format!("{}", r.1[2]),
                format!("{}", r.1[3]),
                format!("{}", r.1[4]),
                format!("{}", r.1[5]),
            ]).unwrap();
        }

        std::fs::remove_dir_all("out").unwrap();

        for (ai_str, results) in behaviors.iter() {

            let dir_str = format!("out/{:?}", ai_str);
            std::fs::create_dir_all(&dir_str).unwrap();

            for (set_id, set) in results.iter().enumerate() {
                let mut wtr = Writer::from_path(format!("{dir_str}/set_{:?}.csv", set_id)).unwrap();

                wtr.write_record(&[
                    "turn",
                    "player",
                    "action",
                ]).unwrap();

                for turn in set.iter() {
                    wtr.write_record(&[
                        format!("{}", turn.turn),
                        format!("{}", turn.player_idx),
                        format!("{:?}", turn.action),
                    ]).unwrap();
                }
            }
        }

        wtr.flush().unwrap();
    }
}