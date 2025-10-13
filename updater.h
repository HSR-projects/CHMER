#pragma once
#include <string>

class CHMERUpdater {
    std::string latest_tag;
    std::string latest_url;

public:
    void fetch_latest_release(bool beta=false);
    void download_release(const std::string& url);
    void extract_release();
    void run_setup();
    void mark_installed();
    bool is_installed();
    void check_for_updates(bool beta=false, bool force=false);
};
