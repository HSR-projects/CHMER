(#import) gui;
(#import) chess;

board = chess.board();
board.load("startpos");

func draw(ui) {
    ui.text("CHMER Desktop Chess - click piece then target");
    ui.chessboard(board);
}

gui.run("CHMER Desktop Chess", 920, 680, draw);
