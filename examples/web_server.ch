(#import) inet;

func home(req) {
    return "<h1>CHMER Online</h1><p>It works.</p>";
}

server = inet.server(8080);
server.route("/", home);
server.start();
