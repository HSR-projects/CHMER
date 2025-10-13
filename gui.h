#pragma once
#include <gtkmm.h>
#include <mutex>
#include <string>

class CHMERGui {
    Gtk::Window window;
    Gtk::TextView textview;
    Gtk::ScrolledWindow scrolled;
    std::mutex mtx;

public:
    CHMERGui();
    ~CHMERGui();

    void setup_widgets();
    void launch_gui(int argc, char** argv);
    void append_text(const std::string& text);
    void clear_text();
};
