(#import) chess;

b = chess.board();
b.load("startpos");
html = chess.renderBoardHtml(b);
print(html);
