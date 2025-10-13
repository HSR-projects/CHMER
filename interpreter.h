#pragma once
#include <string>
#include <map>
#include <vector>
#include <functional>
#include <iostream>
#include "stockfish.h"

struct Command {
    std::string name;
    std::map<std::string,std::string> args;
    std::vector<Command> inner; // for loops/ifs
};

class Interpreter {
    std::map<std::string,int> int_vars;
    StockfishEngine engine;
    ChessBoard board;  // Your internal UCI chess board
public:
    void run_file(const std::string& path);
private:
    std::vector<Command> parse(const std::vector<std::string>& lines);
    void execute(const Command& cmd);
};
