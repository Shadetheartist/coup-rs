// information set monte carlo tree search

use std::sync::{Arc, Mutex};
use std::thread;
use petgraph::{Directed, Graph};
use petgraph::graph::NodeIndex;
use petgraph::prelude::StableGraph;
use rand::{SeedableRng, Rng};
use rand_pcg::Pcg64;
use crate::action::Action;
use crate::{Character, Coup};

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
        for _ in 0..num_determinizations {
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

#[derive(Clone, Eq, PartialEq)]
pub struct GraphNode {
    pub turn: usize,
    pub step: usize,
    pub player_id: usize,
    pub action: Action,
    pub state: Coup
}


pub fn generate_graph(num_sims: u32) -> StableGraph<GraphNode, i32, Directed> {
    let num_batches = 1;

    let mut graph = StableGraph::new();
    let mut nodes: Vec<(NodeIndex, GraphNode)> = Vec::new();

    for batch_count in 0..num_batches {
        for simulation_count in 0..num_sims {

            let seed: u64 = 3; // 12345 + batch_count + (simulation_count as u64) * batch_count;
            let mut rng = Pcg64::seed_from_u64(seed);

            let mut game = Coup::new(3);
            let mut step = 0;

            loop {
                let ai_selected_action = ismcts(&game, &mut rng, 6, 100);
                match ai_selected_action {
                    Action::Coup(_, _) |
                    Action::Income(_) |
                    Action::Propose(_, _) => {
                        println!("\nState: {:?}", game);
                    }
                    _ => {}
                }

                println!("{:?}", ai_selected_action);

                match ai_selected_action {
                    Action::Propose(player_id, _) |
                    Action::Income(player_id) |
                    Action::ForeignAid(player_id) |
                    Action::Tax(player_id) |
                    Action::Assassinate(player_id, _) |
                    Action::Coup(player_id, _) |
                    Action::Steal(player_id, _) |
                    Action::Exchange(player_id, _) |
                    Action::Block(player_id, _) |
                    Action::Relent(player_id) |
                    Action::Challenge(player_id) |
                    Action::Lose(player_id, _) |
                    Action::Reveal(player_id, _) |
                    Action::Resolve(player_id) => {
                        let node = GraphNode {
                            turn: game.turn,
                            step: step,
                            player_id: player_id,
                            action: ai_selected_action.clone(),
                            state: game.clone()
                        };

                        let node_index = {
                            let existing = nodes.iter().find(|n| n.1 == node);
                            if let Some(existing) = existing {
                                existing.0
                            } else {
                                graph.add_node(node.clone())
                            }
                        };

                        if step != 0 {
                            let last_node = nodes.last();
                            if let Some(last_node) = last_node {
                                let existing_edge = graph.find_edge(last_node.0, node_index);
                                if let Some(existing_edge) = existing_edge {
                                    let weight = graph.edge_weight(existing_edge).unwrap();
                                    graph.update_edge(last_node.0, node_index, weight + 1);
                                } else {
                                    graph.add_edge(last_node.0, node_index, 1);
                                }
                            }
                        }


                        nodes.push((node_index, node));
                    },
                    _ => {}

                }

                game = game.apply_action(ai_selected_action, &mut rng).unwrap();

                if let Some(winner) = game.winner() {
                    println!("game over, winner is player {winner}");
                    break;
                }

                step += 1;
            }
        }
    }

    graph
}



#[cfg(test)]
mod tests {
    use crate::ai::{generate_graph};

    #[test]
    fn run_test_simulation() {
        generate_graph(3);
    }
}