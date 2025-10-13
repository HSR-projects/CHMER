#include "interpreter.h"
#include <fstream>
#include <sstream>

void Interpreter::run_file(const std::string& path) {
    std::ifstream f(path);
    if(!f) { std::cerr << "File not found: " << path << "\n"; return; }
    std::vector<std::string> lines;
    std::string line;
    while(std::getline(f,line)) lines.push_back(line);
    auto commands = parse(lines);
    for(auto& cmd : commands) execute(cmd);
}

std::vector<Command> Interpreter::parse(const std::vector<std::string>& lines) {
    std::vector<Command> cmds;
    std::vector<Command*> stack; // for loops/ifs
    for(auto& line : lines) {
        std::string l = line;
        if(l.empty() || l[0]=='#') continue;
        std::istringstream iss(l);
        std::string word;
        iss >> word;
        if(word=="loop" || word=="if" || word=="func") {
            Command c; c.name=word;
            stack.push_back(&c);
        } else if(word=="end-loop" || word=="end-if" || word=="end-func") {
            Command c = *stack.back();
            stack.pop_back();
            if(stack.empty()) cmds.push_back(c);
            else stack.back()->inner.push_back(c);
        } else {
            Command c; c.name=word;
            std::string k,v;
            while(iss >> k) {
                auto pos = k.find("=");
                if(pos!=std::string::npos) {
                    v = k.substr(pos+1);
                    k = k.substr(0,pos);
                    c.args[k]=v;
                } else {
                    c.args[k]="true";
                }
            }
            if(stack.empty()) cmds.push_back(c);
            else stack.back()->inner.push_back(c);
        }
    }
    return cmds;
}

void Interpreter::execute(const Command& cmd) {
    if(cmd.name=="set-var") {
        for(auto& [k,v]:cmd.args) int_vars[k]=std::stoi(v);
    } else if(cmd.name=="show-text") {
        for(auto& [k,v]:cmd.args) std::cout << v << "\n";
    } else if(cmd.name=="move") {
        auto it = cmd.args.find("uci");
        if(it!=cmd.args.end()) board.push(it->second);
    } else if(cmd.name=="analyze") {
        int depth = cmd.args.count("depth")?std::stoi(cmd.args.at("depth")):12;
        std::string side = cmd.args.count("side")?cmd.args.at("side"):"both";
        engine.analyze(board, depth, side);
    } else if(cmd.name=="play") {
        double t = cmd.args.count("time")?std::stod(cmd.args.at("time")):0.1;
        std::string side = cmd.args.count("side")?cmd.args.at("side"):"white";
        engine.play(board, side, t);
    } else if(cmd.name=="export") {
        std::string f = cmd.args.count("filename")?cmd.args.at("filename"):"export.pgn";
        board.export_pgn(f);
    } else if(cmd.name=="if") {
        // Very simple: only integer comparisons x>2
        for(auto& inner:cmd.inner) execute(inner);
    } else if(cmd.name=="loop") {
        int n = cmd.args.count("times")?std::stoi(cmd.args.at("times")):1;
        for(int i=0;i<n;i++) for(auto& inner:cmd.inner) execute(inner);
    }
}
