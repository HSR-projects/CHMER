![Logo](ch.png) 

# CHMER — Chess Mini Programming Language
CHMER is a tiny, Python-based programming language for chess scripts (`.chess`).  
It includes a **CLI interpreter** and a **GTK GUI** for analyzing, playing, and exporting chess games.

---

## Features

- Run `.chess` scripts via **command line**:
  ```bash
  chmer --run sample.chess
  chmer --run sample.chess --debug       # CLI with debug
  chmer --run sample.chess --stockfish /path/to/stockfish

    Run .chess scripts with a GTK GUI:

    chmer --gui sample.chess

    Load and analyze PGN games

    Export games to PGN format

    Run custom Python blocks inside .chess scripts

    Integrated with Stockfish for move calculation and analysis

Installation
From Source

    Clone the repository:

git clone https://github.com/HSR-projects/CHMER-chess-programing-language.git
cd CHMER-chess-programing-language

Make the script executable:

chmod +x chmer.py

(Optional) Move it to your PATH:

    sudo mv chmer.py /usr/local/bin/chmer

Dependencies

    Python 3.x

    python-chess:

pip install python-chess

Stockfish engine (for analysis/play):

    sudo apt install stockfish

    GTK 3 (for GUI)

Usage Examples
CLI

chmer --run simple_analyze.chess
chmer --run play_and_export.chess --stockfish /usr/bin/stockfish

GTK GUI

chmer --gui web_annotate.chess

Python Blocks inside .chess files

<<PY>>
# Example Python code
print("Current board position:", board)
<<PY>>

File Structure

    .chess — Script files

    .pgn — Chess game files

    chmer.py — Main interpreter

    chmer-1.0/ — Packaging files for DEB/RPM/Flatpak

Contributing

CHMER is open source! Contributions are welcome via GitHub pull requests.
Follow the GPL license and include proper attribution.
License

This project is licensed under the GNU General Public License v3.0 (GPL-3.0).
See the LICENSE

file for full details.
Author

organisation: HSR-projects 
Owner: Hemesh Kasukurthy
GitHub: https://github.com/HSR-projects
