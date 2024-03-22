// information set monte carlo tree search

use std::sync::{Arc, Mutex};
use std::thread;
use petgraph::{Directed};
use petgraph::graph::NodeIndex;
use petgraph::prelude::StableGraph;
use rand::{SeedableRng, Rng};
use rand_pcg::Pcg64;
use crate::action::Action;
use crate::{Coup};

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

        if game.turn > 100 {
            println!("failed to simulate in a reasonable amount of turns - default winner 0");
            return 0;
        }
    }
}

fn ismcts<R: Rng + Sized + Clone + std::marker::Send>(game: &Coup, rng: &mut R, num_determinizations: usize, num_simulations: usize) -> Action {

    // actions should be the same between the determinization and the current game
    let actions = game.actions();

    let determinization_scores: Arc<Mutex<Vec<Vec<Vec<f32>>>>> = Arc::new(Mutex::new(Vec::new()));

    thread::scope(|scope| {
        for determinization_idx in 0..num_determinizations {
            {
                let mut rng = rng.clone();

                // each determination gets its own new seed by running the random generator so many times
                for _ in 0..determinization_idx {
                    rng.next_u32();
                }

                let actions = actions.clone();
                let determinization_scores_ref_clone = determinization_scores.clone();

                scope.spawn(move || {
                    let game = game.determine(&mut rng, game.current_player_idx);
                    let mut action_scores: Vec<Vec<f32>> = actions.iter().map(|_| vec![]).collect();

                    for (action_idx, action) in actions.iter().enumerate() {
                        let game_after_action = game.apply_action(action.clone(), &mut rng).unwrap();

                        let mut scores: Vec<f32> = game.players.iter().map(|_| 0f32).collect();
                        for _simulation_count in 0..num_simulations {
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
                    sum.iter().zip(&val[action_idx]).map(|(a, b)| {
                        *a + (*b / num_determinizations as f32)
                    }).collect()
                })
        }).collect();

    let mut diff: Vec<(usize, f32)> = avg_scores.iter().enumerate().map(|scores| {
        let num_opps = (game.players.len() - 1) as f32;
        let sum_opps_score = scores.1.iter().enumerate().filter(|(idx, _)| *idx != game.current_player_idx).map(|(_, e)| e).sum::<f32>();
        let avg_opps_score = sum_opps_score / num_opps;
        (scores.0, scores.1[game.current_player_idx] - avg_opps_score)
    }).collect();

    diff.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap()
    });

    actions[diff[0].0].clone()
}

#[derive(Clone, Eq, PartialEq)]
pub struct GraphNode {
    pub weight: u32,
    pub sim: u32,
    pub turn: usize,
    pub step: usize,
    pub player_id: usize,
    pub action: Action,
    pub state: Coup,
}


pub fn generate_graph(num_sims: u32) -> StableGraph<GraphNode, i32, Directed> {
    let mut graph: StableGraph<GraphNode, i32, Directed> = StableGraph::new();
    let mut nodes: Vec<(NodeIndex, GraphNode)> = Vec::new();

    for _simulation_count in 0..num_sims {
        let seed: u64 = 12345 + (_simulation_count as u64);
        let mut rng = Pcg64::seed_from_u64(seed);

        let mut not_rng = Pcg64::seed_from_u64(0);

        let mut game = Coup::new(3, &mut not_rng);
        let mut step = 0;

        loop {
            let ai_selected_action = ismcts(&game, &mut rng, 1, 100);
            match ai_selected_action {
                Action::Coup(_, _) |
                Action::Income(_) |
                Action::Propose(_, _) => {
                    //println!("\nState: {:?}", game);
                }
                _ => {}
            }

            //println!("{:?}", ai_selected_action);

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
                        weight: 1,
                        sim: _simulation_count,
                        turn: game.turn,
                        step,
                        player_id,
                        action: ai_selected_action.clone(),
                        state: game.clone(),
                    };

                    let node_index = {
                        let existing = nodes.iter().find(|n| n.1.state == node.state);
                        if let Some(existing) = existing {
                            graph.node_weight_mut(existing.0).unwrap().weight += 1;
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
                }
                _ => {}
            }

            game = game.apply_action(ai_selected_action, &mut rng).unwrap();

            if let Some(_winner) = game.winner() {
                //println!("game over, winner is player {winner}");
                break;
            }

            step += 1;
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