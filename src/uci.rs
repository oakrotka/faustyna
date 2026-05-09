use std::sync::LazyLock;

use regex::Regex;

fn parse_commands(commands: String) -> Vec<Result<UciCommand, ParseUciCommandError>> {
    let mut parsed_commands = Vec::new();

    for command_untrimmed in commands.lines() {
        let command = command_untrimmed.trim();
        let parsed_command = match command {
            "uci" => Ok(UciCommand::Uci),
            "isready" => Ok(UciCommand::IsReady),
            "ucinewgame" => Ok(UciCommand::UciNewGame),
            "stop" => Ok(UciCommand::Stop),
            "quit" => Ok(UciCommand::Quit),
            _ => Err(ParseUciCommandError::UnrecognizedCommand(
                command.to_string(),
            )),
        };
        parsed_commands.push(parsed_command);
    }

    parsed_commands
}

#[non_exhaustive]
#[derive(Clone)]
enum UciCommand {
    Uci,
    IsReady,
    UciNewGame,
    Position(GameState, Vec<Move>),
    Go(Vec<GoSubcommand>),
    Stop,
    Quit,
}

#[non_exhaustive]
#[derive(Clone)]
enum UciResponse {
    IdName(String),
    IdAuthor(String),
    UciOk,
    ReadyOk,
    BestMove(Move),
}

static POSITION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^ *position +(?:fen (.*?)|startpos) +moves(:? +([a-h][1-8][a-h][1-8][qrbn]?)) *$")
        .unwrap()
});

#[derive(Clone)]
enum GameState {
    StartPos,
    Fen(String),
}

#[non_exhaustive]
#[derive(Clone, Copy)]
enum Move {
    Standard(Position, Position),
    Promotion(Position, Position, PromotionPiece),
}

#[derive(Clone, Copy)]
struct Position {
    row: u8,
    column: u8,
}

#[derive(Clone, Copy)]
enum PromotionPiece {
    Queen,
    Rook,
    Bishop,
    Knight,
}

#[derive(Clone)]
enum GoSubcommand {
    SearchMoves(Vec<Move>),
    Ponder,
    WhiteTime(u64),
    BlackTime(u64),
    WhiteInc(u64),
    BlackInc(u64),
    MovesToGo(u64),
    Depth(u64),
    Nodes(u64),
    Mate(u64),
    MoveTime(u64),
    Infinite,
}

#[derive(Clone)]
enum ParseUciCommandError {
    UnrecognizedCommand(String),
}
