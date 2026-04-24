(#import) chess;

board = chess.board();
board.load("startpos");
moves = board.legalmoves();
print(moves);
