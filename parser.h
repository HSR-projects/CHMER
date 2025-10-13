#pragma once
#include <string>
#include "parser.h"
#include "gui.h"

class CHMERRunner {
    FILE* stockfish_in = nullptr;
    FILE* stockfish_out = nullptr;
    CHMERGui* gui = nullptr;
    bool debug = false;

public:
    CHMERRunner(const std::string& stockfish_path, CHMERGui* gui_ptr=nullptr, bool dbg=false);
    ~CHMERRunner();

    void run_script(const Script& script);
};
