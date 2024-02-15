#[derive(Clone, Debug)]
enum Action {
    Income,
    ForeignTax,
    Assassinate(usize),
    //todo: other character actions
    Block,
    Challenge,
    Lose(usize), // index of card revealed & lost
    Reveal(usize), // index of card exchanged
    Pass,
}

#[derive(Clone)]
enum State {
    AwaitingProposal,
    AwaitingProposalResponse,
    AwaitingProposalBlockResponse,
    AwaitingBlockChallengeResponse,
    AwaitingChallengedProposalResponse,
    AwaitingLoseInfluence(usize),
}



#[derive(Clone)]
struct Coup {
    state: State,
}



#[cfg(test)]
mod tests {

    #[test]
    fn state() {
    }
}

/*
- proposal `a
    - (pass)
    - (block) blocked_proposal `b
        - (relent) relent to block `a
        - (challenge) challenged_block `a
            - (lose) lost challenge `b
            - (reveal & exchange) won challenge `b
                - (discard) must discard `a

    - (challenge) challenged_proposal `b
        - (lose) lost challenge `a
        - (reveal & exchange) won challenge `a
            - (discard) must discard `b
*/

/*
- proposal
    - pass
        - *resolve
            - *next turn
    - block
        - relent
            - *next turn
        - challenge
            - reveal
                - exchange revealed card
                    - choose card to lose
                        - *resolve
            - lose
                - choose card to lose
                    - *resolve
    - challenge
        - reveal
            - exchange revealed card
                - choose card to lose
                    - *resolve
        - lose
            - choose card to lose
                - *resolve

*/