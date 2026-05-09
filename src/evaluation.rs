use std::cmp::Ordering;

use chess::{Board, BoardStatus, ChessMove, Color, MoveGen, Piece};

const DEPTH: u64 = 3;

pub fn choose_move(board: &Board) -> ChessMove {
    let mut alpha = f64::NEG_INFINITY;
    let mut beta = f64::INFINITY;
    let (choice, score) = MoveGen::new_legal(board)
        .map(|chess_move| {
            let score = -alpha_beta(board.make_move_new(chess_move), &mut alpha, &mut beta, DEPTH);
            (chess_move, score)
        })
        .max_by(|(_, val1), (_, val2)| val1.partial_cmp(val2).unwrap_or(Ordering::Less))
        .expect("no moves left");
    println!("score cp {score}");
    choice
}

fn alpha_beta(board: Board, alpha: &mut f64, beta: &mut f64, depth: u64) -> f64 {
    if depth == 0 {
        return evaluate(board);
    }

    match board.status() {
        BoardStatus::Checkmate => f64::NEG_INFINITY,
        BoardStatus::Stalemate => 0.0,
        BoardStatus::Ongoing => {
            for chess_move in MoveGen::new_legal(&board) {
                let score = -alpha_beta(
                    board.make_move_new(chess_move),
                    &mut -*beta,
                    &mut -*alpha,
                    depth - 1,
                );

                if score >= *beta {
                    return *beta;
                }
                if score > *alpha {
                    *alpha = score;
                }
            }

            *alpha
        }
    }
}

fn evaluate(board: Board) -> f64 {
    let evaluate = |color: Color| -> u32 {
        let color_board = board.color_combined(color);
        [
            (Piece::Pawn, 1),
            (Piece::Knight, 3),
            (Piece::Bishop, 3),
            (Piece::Rook, 5),
            (Piece::Queen, 9),
        ]
        .into_iter()
        .map(|(piece, weight)| weight * (board.pieces(piece) & color_board).popcnt())
        .sum()
    };

    let score = (evaluate(Color::White) - evaluate(Color::Black)) as f64;

    match board.side_to_move() {
        Color::White => score,
        Color::Black => -score,
    }
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
            choose_move(&board),
            ChessMove::new(Square::F2, Square::A2, None)
        );
    }

    #[test]
    fn evaluate_board_pieces() {
        let board = Board::from_str("4kp2/8/8/8/8/8/PPPPPPPP/RNBQKBNR b KQ - 0 1").unwrap();
        assert_eq!(
            evaluate(board),
            (1 - (8 + 2 * 3 + 2 * 3 + 2 * 5 + 9)) as f64
        );
    }
}
