#pragma once
#include <string>
#include <vector>
#include <map>
#include <cstdio>

class CHMERGui; // forward declaration

struct Script {
    std::vector<std::string> lines;
    bool load(const std::string& filepath);
};

class CHMERRunner {
public:
    // Constructor: gui_ can be nullptr for CLI mode
    CHMERRunner(const std::string& stockfish_path_, CHMERGui* gui_=nullptr, bool debug_=false);
    ~CHMERRunner();

    void run(const std::string& filepath);
    void execute_line(const std::string& line);

private:
    CHMERGui* gui;
    std::string stockfish_path;
    FILE* stockfish;
    bool debug;

    Script* current_script;
    size_t current_idx;
    std::string line_buffer;
    std::vector<std::string> moves;
    std::vector<std::string> pgn_moves;
    std::map<std::string,std::string> variables;

    // Utility
    std::vector<std::string> split(const std::string& str, char delim);
    std::string join_args(const std::vector<std::string>& args);
    std::string join_moves();
    std::string extract_bestmove(const std::string& sf_output);

    // Core
    std::string send_stockfish(const std::string& cmd);
    void handle_command(const std::string& cmd, const std::vector<std::string>& args);
    void export_pgn(const std::string& filename); // implement as needed
};
