//! Tic-tac-toe — the canonical Elm-architecture game, with *zero* effects.
//!
//! Exercises a structured `Model` (a board, a turn, no phase field — the
//! phase is *derived* from the board on every render), pure game logic
//! (win detection), conditional attributes (cells lock once the game ends),
//! and a reset that is just another message.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

/// The eight ways to line up three in a row.
const LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];

enum Msg {
    /// Place the current player's mark in cell `i`.
    Place(usize),
    Reset,
}

#[derive(Default)]
struct Model {
    board: [Option<Player>; 9],
    turn: Player,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum Player {
    #[default]
    X,
    O,
}

impl Player {
    fn mark(self) -> &'static str {
        match self {
            Player::X => "X",
            Player::O => "O",
        }
    }

    fn other(self) -> Self {
        match self {
            Player::X => Player::O,
            Player::O => Player::X,
        }
    }
}

/// The player with three in a line, if any.
fn winner(board: &[Option<Player>; 9]) -> Option<Player> {
    for &[a, b, c] in &LINES {
        if let Some(p) = board[a]
            && board[b] == Some(p)
            && board[c] == Some(p)
        {
            return Some(p);
        }
    }
    None
}

#[derive(Default)]
struct TicTacToe;

impl App for TicTacToe {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            Msg::Place(i) => {
                // Locked once somebody has won; empty cells only.
                if winner(&model.board).is_none()
                    && model.board.get(i).is_some_and(|cell| cell.is_none())
                {
                    model.board[i] = Some(model.turn);
                    model.turn = model.turn.other();
                }
            }
            Msg::Reset => *model = Model::default(),
        }
        Command::none()
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        let game_over = winner(&model.board).is_some() || model.board.iter().all(Option::is_some);

        let status = match winner(&model.board) {
            Some(p) => format!("{} wins!", p.mark()),
            None if game_over => "Draw.".to_string(),
            None => format!("{} to move", model.turn.mark()),
        };

        let cells = model.board.iter().enumerate().map(|(i, cell)| {
            let mark = cell.map(Player::mark).unwrap_or("");
            let class = match cell {
                Some(Player::X) => "cell x",
                Some(Player::O) => "cell o",
                None => "cell",
            };
            Html::button()
                .class(class)
                .text(mark)
                // Taken cells and finished games take no further clicks.
                .disabled(cell.is_some() || game_over)
                .on_click(move || Msg::Place(i))
        });

        Html::div()
            .class("game")
            .children([
                Html::h1().text("Tic-tac-toe"),
                Html::p().class("status").text(status),
                Html::div().class("board").children(cells),
                Html::button().text("New game").on_click(|| Msg::Reset),
            ])
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<TicTacToe>();
}
