#pragma once
#include <string>
#include "interpreter.h"

class StockfishEngine {
public:
    void analyze(ChessBoard& board, int depth, const std::string& side="both");
    void play(ChessBoard& board, const std::string& side="white", double seconds=0.1);
};
