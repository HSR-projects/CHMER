#pragma once
#include <string>
#include <fstream>
#include <sstream>

namespace CHMERUtils {
    inline std::string read_file(const std::string& path) {
        std::ifstream file(path);
        std::stringstream buffer;
        buffer << file.rdbuf();
        return buffer.str();
    }
}
