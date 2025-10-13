#pragma once
#include <fstream>
#include <string>
#include <sstream>

namespace CHMERUtils {
    inline std::string read_file(const std::string& path) {
        std::ifstream file(path);
        if (!file.is_open()) return "";
        std::ostringstream buffer;
        buffer << file.rdbuf();
        return buffer.str();
    }
}
