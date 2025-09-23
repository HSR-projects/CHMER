#!/usr/bin/env python3
"""
CHMER — a tiny programming language for chess (.chess)
Interpreter + CLI + GTK GUI

Usage:
    chmer --run sample.chess         # CLI run
    chmer --gui sample.chess         # GTK GUI run
    chmer --run sample.chess --debug # CLI run with debug
    chmer --run sample.chess --stockfish /path/to/stockfish
"""

import sys
import shlex
import argparse
import io
from pathlib import Path
import threading

try:
    import chess
    import chess.pgn
    import chess.engine
except ImportError:
    print("Missing dependency: python-chess. Install with: pip install python-chess")
    raise

DEFAULT_STOCKFISH = "stockfish"

# ----------------------------
# Script parser
# ----------------------------
class CHMERScript:
    def __init__(self, text: str):
        self.text = text
        self.commands = []
        self.pgn_blocks = []
        self.py_blocks = []
        self.parse()

    def parse(self):
        lines = self.text.splitlines()
        i, n = 0, len(lines)
        while i < n:
            line = lines[i].strip()
            if not line or line.startswith('#'):
                i += 1
                continue
            if line.startswith('<<PGN>>'):
                buf = []
                i += 1
                while i < n and not lines[i].strip().startswith('<</PGN>>'):
                    buf.append(lines[i])
                    i += 1
                self.pgn_blocks.append('\n'.join(buf))
                i += 1
                continue
            if line.startswith('<<PY>>'):
                buf = []
                i += 1
                while i < n and not lines[i].strip().startswith('<</PY>>'):
                    buf.append(lines[i])
                    i += 1
                self.py_blocks.append('\n'.join(buf))
                i += 1
                continue
            parts = shlex.split(line)
            if parts:
                cmd, kwargs = parts[0], {}
                for p in parts[1:]:
                    if '=' in p:
                        k, v = p.split('=', 1)
                        kwargs[k] = v
                    else:
                        kwargs[p] = True
                self.commands.append((cmd, kwargs))
            i += 1

# ----------------------------
# Runner
# ----------------------------
class CHMERRunner:
    def __init__(self, stockfish_path=DEFAULT_STOCKFISH, gui=None, debug=False):
        self.stockfish_path = stockfish_path
        self.engine = None
        self.gui = gui
        self.debug = debug

    def log(self, *args):
        if self.debug:
            print('[DEBUG]', *args)

    def start_engine(self):
        if self.engine:
            return
        try:
            self.engine = chess.engine.SimpleEngine.popen_uci(self.stockfish_path)
        except FileNotFoundError:
            raise RuntimeError(f"Stockfish binary not found at '{self.stockfish_path}'.")

    def stop_engine(self):
        if self.engine:
            try:
                self.engine.quit()
            except Exception:
                pass
            self.engine = None

    def run_script(self, script: CHMERScript):
        board = chess.Board()
        games = []

        # Load PGN blocks
        for pgn_text in script.pgn_blocks:
            pgn_io = io.StringIO(pgn_text)
            game = chess.pgn.read_game(pgn_io)
            if game:
                node = game
                while node.variations:
                    node = node.variations[0]
                board = node.board()
                games.append(game)
        self.log('Loaded board:', board.fen())

        # Run commands
        for cmd, kwargs in script.commands:
            if cmd == 'analyze':
                depth = int(kwargs.get('depth', 12))
                side = kwargs.get('side', 'both')
                output = kwargs.get('output', 'console')
                self.start_engine()
                self.do_analyze(board, depth, side, output)
            elif cmd == 'play':
                side = kwargs.get('side', 'white')
                think_time = float(kwargs.get('time', 0.1))
                self.start_engine()
                mv = self.do_play(board, side, think_time)
                if mv:
                    print(f"Played move: {mv}")
            elif cmd == 'export':
                fname = kwargs.get('filename', 'export.pgn')
                self.export(board, games, fname)
            else:
                print("Unknown command:", cmd)

        # Run Python blocks
        for pycode in script.py_blocks:
            ctx = {'board': board, 'chess': chess, 'engine': self.engine}
            try:
                exec(pycode, ctx)
            except Exception as e:
                print("Error in PY block:", e)

        self.stop_engine()

    def do_analyze(self, board, depth, side, output):
        if not self.engine:
            self.start_engine()
        if board.is_game_over():
            self.log("Game over detected:", board.result())
            return
        limit = chess.engine.Limit(depth=depth)
        info = self.engine.analyse(board, limit)
        score = info.get('score')
        pv = info.get('pv')
        print(f"Analysis (depth {depth}): {score}")
        if pv:
            print('PV:', ' '.join(str(m) for m in pv))
        if self.gui:
            try:
                self.gui.show_analysis(score, pv)
            except Exception:
                pass

    def do_play(self, board, side, think_time):
        if board.is_game_over():
            print("Game over detected:", board.result())
            return None
        if not self.engine:
            self.start_engine()
        limit = chess.engine.Limit(time=think_time)
        if side.lower() == 'both':
            result = self.engine.play(board, limit)
            if result.move is None:
                print("No legal move available. Game over.")
                return None
            board.push(result.move)
            return result.move
        desired_white = side.lower() == 'white'
        if board.turn != desired_white:
            print(f"Not {side}'s turn. Skipping move.")
            return None
        result = self.engine.play(board, limit)
        if result.move is None:
            print("No legal move available. Game over.")
            return None
        board.push(result.move)
        return result.move

    def export(self, board, games, fname):
        if games:
            with open(fname, 'w') as f:
                for g in games:
                    exporter = chess.pgn.FileExporter(f)
                    g.accept(exporter)
            print('Exported', fname)
        else:
            game = chess.pgn.Game()
            node = game
            for mv in board.move_stack:
                node = node.add_variation(mv)
            with open(fname, 'w') as f:
                exporter = chess.pgn.FileExporter(f)
                game.accept(exporter)
            print('Exported', fname)

# ----------------------------
# GTK GUI
# ----------------------------
class TinyGui:
    def __init__(self, runner, board):
        self.runner = runner
        self.board = board
        import gi
        gi.require_version('Gtk', '3.0')
        from gi.repository import Gtk, GLib
        self.Gtk = Gtk
        self.GLib = GLib

        self.win = Gtk.Window(title='CHMER')
        self.win.set_default_size(480, 480)
        self.box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.win.add(self.box)
        self.text = Gtk.Label(label=str(board))
        self.box.pack_start(self.text, True, True, 0)
        self.analysis = Gtk.Label(label='Analysis: -')
        self.box.pack_start(self.analysis, False, False, 0)
        self.win.connect('destroy', Gtk.main_quit)
        self.win.show_all()

    def show_analysis(self, score, pv):
        self.GLib.idle_add(self.analysis.set_text, f'Analysis: {score} | PV: {pv}')

    def run(self):
        self.Gtk.main()

# ----------------------------
# CLI
# ----------------------------
def run_file(path: Path, stockfish_path, gui_mode, debug):
    text = path.read_text(encoding='utf-8')
    script = CHMERScript(text)
    runner = CHMERRunner(stockfish_path, gui=None, debug=debug)
    if gui_mode:
        try:
            gui = TinyGui(runner, chess.Board())
            runner.gui = gui
            t = threading.Thread(target=lambda: runner.run_script(script))
            t.start()
            gui.run()
            t.join()
            return
        except Exception as e:
            print("GUI unavailable:", e)
            print("Falling back to CLI.")
    runner.run_script(script)

# ----------------------------
# Main
# ----------------------------
def main():
    parser = argparse.ArgumentParser(description='CHMER .chess runner')
    parser.add_argument('--run', metavar='FILE', help='Run .chess script in CLI')
    parser.add_argument('--gui', metavar='FILE', help='Run .chess script in GTK GUI')
    parser.add_argument('--debug', action='store_true', help='Enable debug output')
    parser.add_argument('--stockfish', default=DEFAULT_STOCKFISH, help='Path to Stockfish binary')
    args = parser.parse_args()

    if args.run:
        p = Path(args.run)
        if not p.exists():
            print("File not found:", p)
            sys.exit(1)
        run_file(p, args.stockfish, gui_mode=False, debug=args.debug)
    elif args.gui:
        p = Path(args.gui)
        if not p.exists():
            print("File not found:", p)
            sys.exit(1)
        run_file(p, args.stockfish, gui_mode=True, debug=args.debug)
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
