use std::time::{Duration, Instant};

use chess::{BitBoard, Board, BoardStatus, ChessMove, Color, MoveGen, Piece};
use rapidhash::RapidHashMap;

const MAX_PLY: u64 = u64::MAX;

const TIME_TO_THINK: Duration = Duration::from_millis(100);
const TIME_CHECK_FREQUENCY: u8 = 50;

const ATTACK_PIECES: [Piece; 5] = [
    Piece::Pawn,
    Piece::Knight,
    Piece::Bishop,
    Piece::Rook,
    Piece::Queen,
];

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
        BoardStatus::Stalemate => Some((board, ChessMove::default(), f64::NEG_INFINITY)),
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

fn evaluate(board: Board) -> f64 {
    let evaluate = |color: Color| -> f64 {
        let mut color_score: f64 = 0.0;

        let color_board = board.color_combined(color);
        color_score += ATTACK_PIECES
            .into_iter()
            .map(|piece| piece_weight(&piece) * (board.pieces(piece) & color_board).popcnt() as f64)
            .sum::<f64>();

        let color_board = match board.side_to_move() {
            side if side == color => board,
            _ => match board.null_move() {
                Some(opponent_board) => opponent_board,
                None => {
                    // FIXME: evaluation heuristics like pinned opponent's pieces will not work when
                    // we are in check
                    return color_score - 50.0;
                }
            },
        };

        color_score -= bitboard_piece_value(*color_board.pinned(), &board) / 10.0;
        color_score -= 90.0 * color_board.checkers().popcnt() as f64;

        color_score
    };

    let score = evaluate(Color::White) - evaluate(Color::Black);

    match board.side_to_move() {
        Color::White => score,
        Color::Black => -score,
    }
}

#[inline]
fn piece_weight(piece: &Piece) -> f64 {
    match *piece {
        Piece::Pawn => 100.0,
        Piece::Knight => 300.0,
        Piece::Bishop => 300.0,
        Piece::Rook => 500.0,
        Piece::Queen => 900.0,
        Piece::King => f64::INFINITY,
    }
}

#[inline]
fn bitboard_piece_value(bitboard: BitBoard, board: &Board) -> f64 {
    bitboard
        .map(|sq| board.piece_on(sq).map(|p| piece_weight(&p)).unwrap_or(0.0))
        .sum()
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use chess::{Board, ChessMove, Square};

    use crate::evaluation::{choose_move, evaluate};

    #[test]
    fn mate_in_one() {
        let board = Board::from_str("8/k7/8/5K2/8/8/5R2/1R6 w - - 0 1").unwrap();
        assert_eq!(
            choose_move(board),
            ChessMove::new(Square::F2, Square::A2, None)
        );
    }

    #[test]
    fn evaluate_board_pieces() {
        let board = Board::from_str("4kp2/8/8/8/8/8/PPPPPPPP/RNBQKBNR b KQ - 0 1").unwrap();
        assert_eq!(
            evaluate(board),
            100.0 * (1 - (8 + 2 * 3 + 2 * 3 + 2 * 5 + 9)) as f64
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
}
