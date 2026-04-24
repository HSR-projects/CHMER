(#import) inet;
(#import) chess;

func page(req) {
    b = chess.board();
    b.load("startpos");
    return chess.renderBoardHtml(b);
}

server = inet.server(3000);
server.route("/", page);
server.start();
