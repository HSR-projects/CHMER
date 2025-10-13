#include "runner.h"
#include "updater.h"
#include "gui.h"
#include <iostream>
#include <thread>
#include <string>

int main(int argc, char** argv) {
    std::string run_file;
    bool gui_flag = false;
    bool debug_flag = false;
    bool update_flag = false;
    bool beta_flag = false;
    bool force_flag = false;

    // Command-line arguments
    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];
        if (arg == "--run" && i + 1 < argc) {
            run_file = argv[++i];
        } else if (arg == "--gui") {
            gui_flag = true;
        } else if (arg == "--debug") {
            debug_flag = true;
        } else if (arg == "--update") {
            update_flag = true;
        } else if (arg == "--beta-update") {
            beta_flag = true;
        } else if (arg == "--force-update") {
            force_flag = true;
        } else if (arg == "--help") {
            std::cout << "CHMER v4 Options:\n"
                      << "  --run <file>         Run CHMER script\n"
                      << "  --gui                Launch GUI\n"
                      << "  --debug              Enable debug output\n"
                      << "  --update             Auto-update to latest release\n"
                      << "  --beta-update        Update to latest pre-release\n"
                      << "  --force-update       Force update even if up-to-date\n"
                      << "  --help               Show this message\n";
            return 0;
        }
    }

    // Update if requested
    if (update_flag || beta_flag) {
        CHMERUpdater updater;
        updater.check_for_updates(beta_flag, force_flag);
    }

    if (gui_flag) {
        // GUI mode: create gui first
        CHMERGui gui;
        std::thread gui_thread([&]() { gui.launch_gui(argc, argv); });
        std::this_thread::sleep_for(std::chrono::milliseconds(500)); // Give GUI time to start

        // If a script is also provided, run it with GUI reference
        if (!run_file.empty()) {
            CHMERRunner runner("/usr/games/stockfish", &gui, debug_flag);
            runner.run(run_file);
        }

        if (gui_thread.joinable()) gui_thread.join();

    } else {
        // CLI mode: no GUI object created
        if (!run_file.empty()) {
            CHMERRunner runner("/usr/games/stockfish", nullptr, debug_flag);
            runner.run(run_file);
        }
    }

    return 0;
}
