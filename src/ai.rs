// information set monte carlo tree search

use std::sync::{Arc, Mutex};
use std::thread;
use petgraph::{Directed};
use petgraph::graph::{NodeIndex};
use petgraph::prelude::StableGraph;
use rand::{SeedableRng, Rng};
use rand_pcg::Pcg64;
use crate::action::Action;
use crate::{Coup};

fn simulate<R: Rng + Sized>(game: &Coup, rng: &mut R) -> usize {
    if let Some(winner) = game.winner() {
        return winner;
    }

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
    pub sim: usize,
    pub step: usize,
    pub state: Coup,
}

#[derive(Clone, Eq, PartialEq)]
pub struct GraphEdge {
    pub count: usize,
    pub action: Action,
}

#[derive(Clone)]
pub struct SimPlayerParams {
    pub num_determinations: usize,
    pub num_simulations_per_action: usize,
}

pub struct SimParams {
    pub seed: u64,
    pub num_sims: usize,
    pub sim_players: Vec<SimPlayerParams>,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            seed: 0,
            num_sims: 1,
            sim_players: vec![
                SimPlayerParams {
                    num_determinations: 12,
                    num_simulations_per_action: 100,
                },
                SimPlayerParams {
                    num_determinations: 12,
                    num_simulations_per_action: 100,
                },
                SimPlayerParams {
                    num_determinations: 12,
                    num_simulations_per_action: 100,
                },
            ],
        }
    }
}

fn add_state_to_graph(
    graph: &mut StableGraph<GraphNode, GraphEdge, Directed>,
    nodes: &mut Vec<(NodeIndex, GraphNode)>,
    game: &Coup,
    sim_n: usize,
    step: usize,
) -> NodeIndex {
    let node = GraphNode {
        sim: sim_n,
        step: step,
        state: game.clone(),
    };

    let node_index = {
        let existing = nodes.iter().find(|n| n.1.state == node.state);
        if let Some(existing) = existing {
            existing.0
        } else {
            graph.add_node(node.clone())
        }
    };

    nodes.push((node_index, node));

    node_index
}

fn add_action_to_graph(
    graph: &mut StableGraph<GraphNode, GraphEdge, Directed>,
    action: Action,
    prev_state_idx: NodeIndex,
    new_state_idx: NodeIndex,
) {
    let existing_edge = graph.find_edge(prev_state_idx, new_state_idx);
    if let Some(existing_edge) = existing_edge {
        let edge = graph.edge_weight(existing_edge).unwrap();
        graph.update_edge(prev_state_idx, new_state_idx, GraphEdge { action: action, count: edge.count + 1 });
    } else {
        graph.add_edge(prev_state_idx, new_state_idx, GraphEdge { action: action, count: 1 });
    }
}

pub fn generate_graph(sim_params: SimParams) -> StableGraph<GraphNode, GraphEdge, Directed> {
    let mut graph: StableGraph<GraphNode, GraphEdge, Directed> = StableGraph::new();
    let mut nodes: Vec<(NodeIndex, GraphNode)> = Vec::new();

    for sim_n in 0..sim_params.num_sims {
        let mut not_rng = Pcg64::seed_from_u64(sim_params.seed);
        let mut per_sim_rng = Pcg64::seed_from_u64(sim_params.seed + (sim_n as u64));

        let mut game = Coup::new(sim_params.sim_players.len() as u8, &mut not_rng);
        let mut step = 0usize;

        add_state_to_graph(&mut graph, &mut nodes, &mut game, sim_n, step);

        step += 1;

        loop {
            let sim_player = &sim_params.sim_players[game.current_player_idx];
            let ai_selected_action = ismcts(&game, &mut per_sim_rng, sim_player.num_determinations, sim_player.num_simulations_per_action);

            let prev_node_idx = nodes.last().unwrap().0;

            game = game.apply_action(ai_selected_action.clone(), &mut per_sim_rng).unwrap();

            match ai_selected_action {
                Action::Propose(_player_id, _) |
                Action::Income(_player_id) |
                Action::ForeignAid(_player_id) |
                Action::Tax(_player_id) |
                Action::Assassinate(_player_id, _) |
                Action::Coup(_player_id, _) |
                Action::Steal(_player_id, _) |
                Action::Exchange(_player_id, _) |
                Action::Block(_player_id, _) |
                Action::Relent(_player_id) |
                Action::Challenge(_player_id) |
                Action::Lose(_player_id, _) |
                Action::Reveal(_player_id, _) |
                Action::Resolve(_player_id) => {
                    let new_node_idx = add_state_to_graph(&mut graph, &mut nodes, &game, sim_n, step);
                    add_action_to_graph(&mut graph, ai_selected_action.clone(), prev_node_idx, new_node_idx);
                }
                _ => {}
            }

            step += 1;

            if let Some(_winner) = game.winner() {
                break;
            }
        }
    }

    graph
}


#[cfg(test)]
mod tests {
    use crate::ai::{generate_graph, SimParams};

    #[test]
    fn run_test_simulation() {
        generate_graph(SimParams::default());
    }
}