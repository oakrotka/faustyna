use std::time::{Duration, Instant};

use chess::{Board, BoardStatus, ChessMove, MoveGen};
use rapidhash::RapidHashMap;

use crate::evaluation::evaluate;

const MAX_PLY: u64 = u64::MAX;

const TIME_TO_THINK: Duration = Duration::from_millis(2000);
const TIME_CHECK_FREQUENCY: u8 = 50;

pub fn choose_move(board: Board) -> ChessMove {
    let start_time = Instant::now();

    let mut best_choice: Option<ChessMove> = None;
    let mut best_score = f64::NEG_INFINITY;
    let mut previous_state = board;

    for depth in 1..MAX_PLY {
        println!("info depth {depth}");

        let mut transposition_table = RapidHashMap::<u64, f64>::default();
        let mut iterations = TIME_CHECK_FREQUENCY;

        // searching the previously found best move first for better alpha pruning
        if depth > 1 {
            // updating move stats for a greater depth
            (best_score, previous_state) = match search_moves(
                previous_state,
                f64::INFINITY,
                f64::NEG_INFINITY,
                1,
                &mut transposition_table,
                &mut iterations,
                start_time
            ) {
                SearchResult::BestMove(_, score, state) if depth % 2 == 0 => (-score, state),
                SearchResult::BestMove(_, score, state) => (score, state),
                SearchResult::OutOfTime | SearchResult::OutOfTimeWithResult(..) => break,
                SearchResult::CacheHit(_) | SearchResult::Leaf(..) => {
                    unreachable!("impossible with an empty transposition table and depth 1")
                }
            };

            // checkmate guaranteed
            if best_score == f64::INFINITY {
                break;
            }
        }

        // search the tree thoroughly this time
        match search_moves(
            board,
            best_score,
            f64::INFINITY,
            depth,
            &mut transposition_table,
            &mut iterations,
            start_time
        ) {
            SearchResult::BestMove(choice, score, state) if score > best_score => {
                best_choice = Some(choice);
                best_score = score;
                previous_state = state;
            }
            SearchResult::OutOfTimeWithResult(choice, score) if score > best_score => {
                best_choice = Some(choice);
                best_score = score;
                break;
            }
            SearchResult::OutOfTimeWithResult(..) | SearchResult::OutOfTime => break,
            SearchResult::BestMove(..) | SearchResult::CacheHit(_) => (),
            SearchResult::Leaf(..) => unreachable!("impossible with depth >=1"),
        };

        if previous_state.status() != BoardStatus::Ongoing || start_time.elapsed() >= TIME_TO_THINK
        {
            break;
        }
    }

    println!("info score cp {best_score}");
    best_choice.expect("no move was found")
}

/// Returns the best move found from the given position, its score and the state of the board at the
/// deepest searched moment.
fn search_moves(
    board: Board,
    alpha: f64,
    beta: f64,
    depth: u64,
    transposition_table: &mut RapidHashMap<u64, f64>,
    iterations: &mut u8,
    start_time: Instant
) -> SearchResult {
    // check the time every fixed amount of searches to avoid having to do a syscall every time
    // (TODO: is this really faster?)
    if *iterations == 0 {
        if start_time.elapsed() >= TIME_TO_THINK {
            return SearchResult::OutOfTime;
        }
        *iterations = TIME_CHECK_FREQUENCY;
    }
    *iterations -= 1;

    // look for position in cache
    if let Some(&score) = transposition_table.get(&board.get_hash()) {
        return SearchResult::CacheHit(score);
    }

    // descend deeper into the move tree
    let res = alpha_beta(
        board,
        alpha,
        beta,
        depth,
        transposition_table,
        iterations,
        start_time
    );

    // update the cache
    if let Some(score) = res.score() {
        transposition_table.insert(board.get_hash(), score);
    }

    res
}

fn alpha_beta(
    board: Board,
    mut alpha: f64,
    beta: f64,
    depth: u64,
    transposition_table: &mut RapidHashMap<u64, f64>,
    iterations: &mut u8,
    start_time: Instant
) -> SearchResult {
    if depth == 0 {
        let score = evaluate(board);
        return SearchResult::Leaf(score, board);
    }

    match board.status() {
        BoardStatus::Checkmate => SearchResult::Leaf(f64::NEG_INFINITY, board),
        BoardStatus::Stalemate => SearchResult::Leaf(0.0, board),
        BoardStatus::Ongoing => {
            // if the game is ongoing, there has to be at least one move to be considered
            let mut best_value: Option<SearchResult> = None;
            let mut best_score = f64::NEG_INFINITY;

            for chess_move in MoveGen::new_legal(&board) {
                let (findings, score) = match search_moves(
                    board.make_move_new(chess_move),
                    -beta,
                    -alpha,
                    depth - 1,
                    transposition_table,
                    iterations,
                    start_time
                ) {
                    // found a notable move
                    SearchResult::BestMove(_, neg_score, state)
                    | SearchResult::Leaf(neg_score, state) => (
                        SearchResult::BestMove(chess_move, -neg_score, state),
                        -neg_score,
                    ),
                    SearchResult::CacheHit(neg_score) => {
                        (SearchResult::CacheHit(-neg_score), -neg_score)
                    }

                    // out of time
                    SearchResult::OutOfTime => {
                        return best_value.unwrap_or(SearchResult::OutOfTime);
                    }
                    SearchResult::OutOfTimeWithResult(_, neg_score) => {
                        let score = -neg_score;
                        let oot_res = SearchResult::OutOfTimeWithResult(chess_move, score);
                        return match score.gt(&best_score) {
                            false => best_value.unwrap_or(oot_res).timeout(),
                            true => oot_res,
                        };
                    }
                };

                if best_value.is_none() || score > best_score {
                    best_value = Some(findings);
                    best_score = score;

                    if score > alpha {
                        alpha = score;

                        if score >= beta {
                            return findings;
                        }
                    }
                }
            }

            best_value.expect("a value should always be found")
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SearchResult {
    BestMove(ChessMove, f64, Board),
    OutOfTimeWithResult(ChessMove, f64),
    OutOfTime,
    // unreachable when starting with empty transposition table
    CacheHit(f64),
    // unreachable if called with a depth of 1 or greater
    Leaf(f64, Board),
}

impl SearchResult {
    fn score(&self) -> Option<f64> {
        match *self {
            Self::OutOfTime => None,
            Self::BestMove(_, score, _)
            | Self::OutOfTimeWithResult(_, score)
            | Self::CacheHit(score)
            | Self::Leaf(score, _) => Some(score),
        }
    }

    fn timeout(&self) -> Self {
        match *self {
            Self::BestMove(choice, score, _) | Self::OutOfTimeWithResult(choice, score) => {
                Self::OutOfTimeWithResult(choice, score)
            }
            _ => Self::OutOfTime,
        }
    }
}

impl PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.score()
            .zip(other.score())
            .and_then(|(a, b)| a.partial_cmp(&b))
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use chess::{Board, ChessMove, Square};

    use crate::search::choose_move;

    #[test]
    fn mate_in_one() {
        let board = Board::from_str("8/k7/8/5K2/8/8/5R2/1R6 w - - 0 1").unwrap();
        assert_eq!(
            choose_move(board),
            ChessMove::new(Square::F2, Square::A2, None)
        );
    }

    #[test]
    fn dont_give_away_pieces() {
        let position = "r1bqkbnr/2pn2Pp/8/1p6/3PN3/p1P2N2/P4P1P/R1BQKB1R b KQkq - 0 13";
        let board = Board::from_str(position).unwrap();
        let choice = choose_move(board);
        /*
        println!("{}", evaluate(board.make_move_new(ChessMove::new(Square::F8, Square::G7, None))));
        println!("{}", evaluate(board.make_move_new(ChessMove::new(Square::B5, Square::B4, None))));
        */
        assert_eq!(
            choice,
            ChessMove::new(Square::F8, Square::G7, None)
        );
    }

    #[test]
    fn faustyna_please() {
        let position = "rn1k1b1r/pp5p/2p1bp1p/4N3/4P3/1PP5/P3BPPP/RN2K2R w KQ - 0 12";
        let board = Board::from_str(position).unwrap();
        let choice = choose_move(board);
        assert_ne!(choice, ChessMove::new(Square::H1, Square::F1, None));
    }

    #[test]
    fn im_losing_my_mind() {
        let position = "8/ppk4p/3R4/7p/8/1PP1K3/P5r1/8 w - - 1 29";
        let board = Board::from_str(position).unwrap();
        let choice = choose_move(board);
        assert_ne!(choice, ChessMove::new(Square::D6, Square::D7, None));
    }

    #[test]
    fn dont_make_illegal_moves_please() {
        let position = "rnbqk1nr/3p1ppp/1pp1p3/3P4/1p2P3/5N2/PP3PPP/RNBQKB1R b KQkq - 0 7";
        let board = Board::from_str(position).unwrap();
        let choice = choose_move(board);
        println!("{choice}");
        board.make_move_new(choice);
    }
}
