use chess::{BitBoard, Board, Color, Piece};

const ATTACK_PIECES: [Piece; 5] = [
    Piece::Pawn,
    Piece::Knight,
    Piece::Bishop,
    Piece::Rook,
    Piece::Queen,
];

pub fn evaluate(board: Board) -> f64 {
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

    use chess::Board;

    use crate::evaluation::evaluate;

    #[test]
    fn evaluate_board_pieces() {
        let board = Board::from_str("4kp2/8/8/8/8/8/PPPPPPPP/RNBQKBNR b KQ - 0 1").unwrap();
        assert_eq!(
            evaluate(board),
            100.0 * (1 - (8 + 2 * 3 + 2 * 3 + 2 * 5 + 9)) as f64
        );
    }
}
