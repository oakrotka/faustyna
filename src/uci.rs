use std::{
    io::{self, BufRead},
    process,
    str::FromStr,
};

use chess::Board;
use vampirc_uci::{self as uci, Serializable, UciMessage};

use crate::{ENGINE_NAME, search};

pub fn event_loop() -> ! {
    let stdin = io::stdin();
    let mut command = String::new();

    let mut state = Board::default();

    loop {
        command.clear();
        {
            let n = stdin
                .lock()
                .read_line(&mut command)
                .expect("could not read from stdin");
            if n == 0 {
                panic!("sudden EOF on stdin");
            }
        }
        match uci::parse_one(&command) {
            UciMessage::Uci => {
                println!("id name {}", ENGINE_NAME);
                println!("uciok");
            }
            UciMessage::IsReady => {
                println!("readyok");
            }
            UciMessage::Position {
                startpos,
                fen,
                moves,
            } => {
                let mut game = match fen {
                    Some(fen_str) => Board::from_str(fen_str.as_str()).expect("got invalid FEN"),
                    None => {
                        debug_assert!(startpos);
                        Board::default()
                    }
                };
                for chess_move in moves.iter() {
                    game = game.make_move_new(*chess_move);
                }
                state = game;
            }
            UciMessage::Go { .. } => {
                let choice = search::choose_move(state);
                let response = UciMessage::BestMove {
                    best_move: choice,
                    ponder: None,
                }
                .serialize();
                println!("{response}");
            }
            UciMessage::Quit => {
                process::exit(0);
            }
            _ => {
                eprintln!("unrecognized or unimplemented command: {}", command.trim());
            }
        }
    }
}
