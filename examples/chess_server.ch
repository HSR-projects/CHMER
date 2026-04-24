(#import) inet;
(#import) chess;

board = chess.board();
board.load("startpos");

home = chess.renderBoardHtml(board);
moves = "Legal moves: " + board.legalmoves();

server = inet.server(8080);
server.routeText("/", home);
server.routeText("/health", "ok");
server.routeText("/bug" , "good boy");
server.routeText("/moves", moves);
server.start();
