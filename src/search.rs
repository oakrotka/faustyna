use std::time::{Duration, Instant};

use chess::{Board, BoardStatus, ChessMove, MoveGen};
use rapidhash::RapidHashMap;

use crate::evaluation::evaluate;

const MAX_PLY: u64 = u64::MAX;

const TIME_TO_THINK: Duration = Duration::from_millis(100);
const TIME_CHECK_FREQUENCY: u8 = 50;

pub fn choose_move(board: Board) -> ChessMove {
    let start_time = Instant::now();

    let mut previous_state = board;

    let mut choice = ChessMove::default();
    let mut score = f64::NEG_INFINITY;

    for depth in 1..MAX_PLY {
        println!("info depth {depth}");

        let mut transposition_table = RapidHashMap::<u64, f64>::default();
        let mut iterations = TIME_CHECK_FREQUENCY;

        // searching the previously found best move first for better alpha pruning
        if depth > 1 {
            (previous_state, _, score) = match search_moves(
                previous_state,
                f64::INFINITY,
                f64::NEG_INFINITY,
                1,
                &mut transposition_table,
                &mut iterations,
                start_time,
            ) {
                Some(res) => res,
                None => return choice,
            };

            if score == f64::INFINITY {
                break;
            }
        }

        // search the tree thoroughly this time
        let (found_state, found_move, found_score) = match search_moves(
            board,
            score,
            f64::INFINITY,
            depth,
            &mut transposition_table,
            &mut iterations,
            start_time,
        ) {
            Some(res) => res,
            None => return choice,
        };

        if found_score > score || depth == 1 {
            (previous_state, choice, score) = (found_state, found_move, found_score);
        }

        if previous_state.status() != BoardStatus::Ongoing || start_time.elapsed() >= TIME_TO_THINK
        {
            break;
        }
    }

    println!("info score cp {score}");
    choice
}

/// Returns the best move found from the given position, its score and the state of the board at the
/// deepest searched moment. May return None if time runs out.
fn search_moves(
    board: Board,
    alpha: f64,
    beta: f64,
    depth: u64,
    transposition_table: &mut RapidHashMap<u64, f64>,
    iterations: &mut u8,
    start_time: Instant,
) -> Option<(Board, ChessMove, f64)> {
    if *iterations == 0 {
        if start_time.elapsed() >= TIME_TO_THINK {
            return None;
        }
        *iterations = TIME_CHECK_FREQUENCY;
    }
    *iterations -= 1;

    if let Some(score) = transposition_table.get(&board.get_hash()) {
        return Some((Board::default(), ChessMove::default(), *score));
    }

    let res = alpha_beta(
        board,
        alpha,
        beta,
        depth,
        transposition_table,
        iterations,
        start_time,
    );

    if let Some((_, _, score)) = res {
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
    start_time: Instant,
) -> Option<(Board, ChessMove, f64)> {
    if depth == 0 {
        let score = evaluate(board);
        transposition_table.insert(board.get_hash(), score);
        return Some((board, ChessMove::default(), score));
    }

    match board.status() {
        BoardStatus::Checkmate => Some((board, ChessMove::default(), f64::NEG_INFINITY)),
        BoardStatus::Stalemate => Some((board, ChessMove::default(), 0.0)),
        BoardStatus::Ongoing => {
            let mut best_state = Board::default();
            let mut best_move = ChessMove::default();

            for chess_move in MoveGen::new_legal(&board) {
                let (found_state, _, neg_score) = search_moves(
                    board.make_move_new(chess_move),
                    -beta,
                    -alpha,
                    depth - 1,
                    transposition_table,
                    iterations,
                    start_time,
                )?;
                let score = -neg_score;

                if score >= beta {
                    return Some((found_state, chess_move, score));
                }
                if score > alpha {
                    best_state = found_state;
                    best_move = chess_move;
                    alpha = score;
                }
            }

            Some((best_state, best_move, alpha))
        }
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
        /*
        println!("{}", evaluate(board.make_move_new(ChessMove::new(Square::F8, Square::G7, None))));
        println!("{}", evaluate(board.make_move_new(ChessMove::new(Square::B5, Square::B4, None))));
        */
        assert_eq!(
            choose_move(board),
            ChessMove::new(Square::F8, Square::G7, None)
        );
    }

    #[test]
    fn faustyna_please() {
        let position = "rn1k1b1r/pp5p/2p1bp1p/4N3/4P3/1PP5/P3BPPP/RN2K2R w KQ - 0 12";
        let board = Board::from_str(position).unwrap();
        let mv = choose_move(board);
        println!("{mv}");
        assert_ne!(
            mv,
            ChessMove::new(Square::H1, Square::F1, None)
        );
    }

    #[test]
    fn im_losing_my_mind() {
        let position = "8/ppk4p/3R4/7p/8/1PP1K3/P5r1/8 w - - 1 29";
        let board = Board::from_str(position).unwrap();
        let mv = choose_move(board);
        assert_ne!(mv, ChessMove::new(Square::D6, Square::D7, None));
    }
}
