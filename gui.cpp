#include "gui.h"
#include <iostream>

CHMERGui::CHMERGui() {
    setup_widgets();
}

CHMERGui::~CHMERGui() {}

void CHMERGui::setup_widgets() {
    scrolled.add(textview);
    scrolled.set_policy(Gtk::POLICY_AUTOMATIC, Gtk::POLICY_AUTOMATIC);
    window.add(scrolled);
    window.set_default_size(600, 400);
    window.set_title("CHMER GUI");
    window.show_all_children();
}

void CHMERGui::launch_gui(int argc, char** argv) {
    auto app = Gtk::Application::create(argc, argv, "chmer.gui");
    app->run(window);
}

void CHMERGui::append_text(const std::string& text) {
    std::lock_guard<std::mutex> lock(mtx);
    auto buffer = textview.get_buffer();
    buffer->insert(buffer->end(), text + "\n");
    auto iter = buffer->end();  // store iterator in a variable
textview.scroll_to(iter);
}

void CHMERGui::clear_text() {
    std::lock_guard<std::mutex> lock(mtx);
    auto buffer = textview.get_buffer();
    buffer->set_text("");
}
