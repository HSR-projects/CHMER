#include "runner.h"
#include "gui.h"
#include <iostream>
#include <fstream>
#include <sstream>
#include <stdexcept>
#include <cstring>

// -------------------- Utility --------------------
std::vector<std::string> CHMERRunner::split(const std::string& str, char delim) {
    std::vector<std::string> result;
    std::string temp;
    for (char c : str) {
        if (c == delim) {
            if (!temp.empty()) result.push_back(temp);
            temp.clear();
        } else temp += c;
    }
    if (!temp.empty()) result.push_back(temp);
    return result;
}

std::string CHMERRunner::join_args(const std::vector<std::string>& args) {
    std::string out;
    for (auto& a : args) out += a + " ";
    if (!out.empty()) out.pop_back();
    return out;
}

std::string CHMERRunner::join_moves() {
    std::string out;
    for (auto& m : moves) out += m + " ";
    if (!out.empty()) out.pop_back();
    return out;
}

std::string CHMERRunner::extract_bestmove(const std::string& sf_output) {
    auto pos = sf_output.find("bestmove ");
    if (pos == std::string::npos) return "";
    auto end = sf_output.find(' ', pos+9);
    return sf_output.substr(pos+9, end - pos - 9);
}

// -------------------- Constructor / Destructor --------------------
CHMERRunner::CHMERRunner(const std::string& stockfish_path_, CHMERGui* gui_, bool debug_)
    : stockfish_path(stockfish_path_), gui(gui_), stockfish(nullptr), debug(debug_),
      current_script(nullptr), current_idx(0) {}

CHMERRunner::~CHMERRunner() {
    if (stockfish) pclose(stockfish);
}

// -------------------- Script Loader --------------------
bool Script::load(const std::string& filepath) {
    std::ifstream file(filepath);
    if (!file.is_open()) return false;
    std::string line;
    while (std::getline(file, line)) lines.push_back(line);
    return true;
}

// -------------------- Stockfish Communication --------------------
std::string CHMERRunner::send_stockfish(const std::string& cmd) {
    if (!stockfish) {
        stockfish = popen(stockfish_path.c_str(), "w+");
        if (!stockfish) {
            std::cerr << "Failed to start Stockfish at: " << stockfish_path << std::endl;
            throw std::runtime_error("Failed to start Stockfish");
        }
    }

    fprintf(stockfish, "%s\n", cmd.c_str());
    fflush(stockfish);

    char buffer[256];
    std::string output;
    while (fgets(buffer, sizeof(buffer), stockfish)) {
        output += buffer;
        if (strstr(buffer, "bestmove")) break;
    }
    return output;
}

// -------------------- Runner --------------------
void CHMERRunner::run(const std::string& filepath) {
    Script script;
    if (!script.load(filepath)) {
        std::cerr << "[Runner] Failed to load script: " << filepath << "\n";
        return;
    }

    current_script = &script;
    for (current_idx = 0; current_idx < script.lines.size(); ++current_idx) {
        line_buffer = script.lines[current_idx];
        execute_line(line_buffer);
    }
    current_script = nullptr;

    if (debug) std::cout << "[Runner] Execution complete.\n";
}

void CHMERRunner::execute_line(const std::string& line) {
    if (line.empty() || line[0] == '#') return; // comment
    auto tokens = split(line, ' ');
    if (tokens.empty()) return;

    std::string cmd = tokens[0];
    tokens.erase(tokens.begin());
    handle_command(cmd, tokens);
}

// -------------------- Command Handler --------------------
void CHMERRunner::handle_command(const std::string& cmd, const std::vector<std::string>& args) {
    if (cmd == "show-text") {
        std::string text = join_args(args);
        if (gui) gui->append_text(text);
        else std::cout << text << "\n";
    }
    else if (cmd == "set-var") {
        auto parts = split(args[0], '=');
        if (parts.size() == 2) variables[parts[0]] = parts[1];
    }
    else if (cmd == "analyze") {
        int depth = 12;
        for (auto& a : args) {
            if (a.find("depth=") == 0) depth = std::stoi(a.substr(6));
        }
        std::string sf_cmd = "position startpos\ngo depth " + std::to_string(depth);
        std::string result = send_stockfish(sf_cmd);
        if (debug) std::cout << "[Runner] Analysis (depth " << depth << "): " << result << "\n";
    }
    else if (cmd == "play") {
        std::string side = "white";
        double time = 1.0;
        for (auto& a : args) {
            if (a.find("side=") == 0) side = a.substr(5);
            else if (a.find("time=") == 0) time = std::stod(a.substr(5));
        }
        std::string sf_cmd = "position startpos\ngo movetime " + std::to_string(int(time*1000));
        std::string result = send_stockfish(sf_cmd);
        auto best = extract_bestmove(result);
        moves.push_back(best);
        pgn_moves.push_back(best);
        if (debug) std::cout << "[Runner] Played move: " << best << "\n";
    }
    else if (cmd == "move") {
        std::string mv = args[0];
        moves.push_back(mv);
        pgn_moves.push_back(mv);
        std::string sf_cmd = "position startpos moves " + join_moves() + "\n";
        send_stockfish(sf_cmd);
    }
    else if (cmd == "export") {
        std::string filename = args[0].substr(9); // filename="game.pgn"
        export_pgn(filename);
    }
    else std::cerr << "Unknown command: " << cmd << "\n";
}
void CHMERRunner::export_pgn(const std::string& filename) {
    // Simple placeholder: write moves to a PGN file
    std::ofstream out(filename);
    if (!out.is_open()) {
        std::cerr << "[Runner] Failed to open PGN file: " << filename << "\n";
        return;
    }

    out << "[Event \"CHMER Script\"]\n";
    out << "[Site \"Local\"]\n";
    out << "[Date \"2025.10.13\"]\n";
    out << "[Round \"-\"]\n";
    out << "[White \"CHMER\"]\n";
    out << "[Black \"Stockfish\"]\n";
    out << "[Result \"*\"]\n\n";

    int move_num = 1;
    for (size_t i = 0; i < pgn_moves.size(); i += 2) {
        out << move_num << ". " << pgn_moves[i];
        if (i + 1 < pgn_moves.size()) out << " " << pgn_moves[i + 1];
        out << " ";
        move_num++;
    }
    out << "*\n";

    out.close();
    if (debug) std::cout << "[Runner] PGN exported to " << filename << "\n";
}

