#include "parser.h"
#include <fstream>
#include <sstream>

bool Script::load_from_file(const std::string& filepath) {
    std::ifstream file(filepath);
    if (!file.is_open()) return false;

    std::string line;
    while (std::getline(file, line)) {
        if (line.empty() || line[0]=='#') continue;

        std::istringstream iss(line);
        std::string cmd;
        iss >> cmd;
        ScriptCommand sc;
        sc.name = cmd;

        std::string arg;
        while (iss >> arg) sc.args.push_back(arg);

        commands.push_back(sc);
    }
    return true;
}
