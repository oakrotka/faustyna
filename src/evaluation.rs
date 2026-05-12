use std::cmp::Ordering;

use chess::{BitBoard, Board, BoardStatus, ChessMove, Color, MoveGen, Piece};
use rapidhash::RapidHashMap;

const DEPTH: u64 = 4;

const ATTACK_PIECES: [Piece; 5] = [
    Piece::Pawn,
    Piece::Knight,
    Piece::Bishop,
    Piece::Rook,
    Piece::Queen,
];

pub fn choose_move(board: &Board) -> ChessMove {
    let mut alpha = f64::NEG_INFINITY;
    let mut beta = f64::INFINITY;
    let mut transposition_table = RapidHashMap::<u64, f64>::default();

    let (choice, score) = MoveGen::new_legal(board)
        .map(|chess_move| {
            let score = -alpha_beta(
                board.make_move_new(chess_move),
                &mut alpha,
                &mut beta,
                DEPTH,
                &mut transposition_table,
            );
            (chess_move, score)
        })
        .max_by(|(_, val1), (_, val2)| val1.partial_cmp(val2).unwrap_or(Ordering::Less))
        .expect("no moves left");
    println!("score cp {score}");
    choice
}

fn alpha_beta(
    board: Board,
    alpha: &mut f64,
    beta: &mut f64,
    depth: u64,
    transposition_table: &mut RapidHashMap<u64, f64>,
) -> f64 {
    if let Some(score) = transposition_table.get(&board.get_hash()) {
        return *score;
    }

    if depth == 0 {
        let score = evaluate(board);
        transposition_table.insert(board.get_hash(), score);
        return score;
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
                    transposition_table,
                );

                if score >= *beta {
                    transposition_table.insert(board.get_hash(), *beta);
                    return *beta;
                }
                if score > *alpha {
                    *alpha = score;
                }
            }

            transposition_table.insert(board.get_hash(), *alpha);
            *alpha
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
                    return color_score;
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
    bitboard.map(|sq| board.piece_on(sq).map(|p| piece_weight(&p)).unwrap_or(0.0)).sum()
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
            100.0 * (1 - (8 + 2 * 3 + 2 * 3 + 2 * 5 + 9)) as f64
        );
    }

    #[test]
    fn dont_give_away_pieces() {
        let position = "r1bqkbnr/2pn2Pp/8/1p6/3PN3/p1P2N2/P4P1P/R1BQKB1R b KQkq - 0 13";
        let board = Board::from_str(position).unwrap();
        assert_eq!(
            choose_move(&board),
            ChessMove::new(Square::F8, Square::G7, None)
        );
    }
}
