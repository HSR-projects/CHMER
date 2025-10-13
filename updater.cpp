#include "updater.h"
#include <iostream>
#include <fstream>
#include <filesystem>
#include <cstdlib>
#include <curl/curl.h>
#include <nlohmann/json.hpp>

namespace fs = std::filesystem;

const std::string GITHUB_API_LATEST = "https://api.github.com/repos/ChessDNA/CHMER-chess-programing-language/releases/latest";
const std::string GITHUB_API_BETA   = "https://api.github.com/repos/ChessDNA/CHMER-chess-programing-language/releases";

size_t write_callback(void* contents, size_t size, size_t nmemb, void* userp) {
    ((std::string*)userp)->append((char*)contents, size*nmemb);
    return size*nmemb;
}

void CHMERUpdater::fetch_latest_release(bool beta) {
    CURL* curl = curl_easy_init();
    if(!curl) throw std::runtime_error("Failed to init curl");

    std::string readBuffer;
    std::string url = beta ? GITHUB_API_BETA : GITHUB_API_LATEST;

    curl_easy_setopt(curl, CURLOPT_URL, url.c_str());
    curl_easy_setopt(curl, CURLOPT_USERAGENT, "CHMER-Updater");
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, write_callback);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &readBuffer);

    CURLcode res = curl_easy_perform(curl);
    curl_easy_cleanup(curl);
    if(res != CURLE_OK) throw std::runtime_error("Failed to fetch GitHub release");

    auto json = nlohmann::json::parse(readBuffer);
    if(beta) {
        for(auto& rel : json) {
            if(rel["prerelease"].get<bool>()) {
                latest_tag = rel["tag_name"].get<std::string>();
                latest_url = rel["assets"][0]["browser_download_url"].get<std::string>();
                return;
            }
        }
    } else {
        latest_tag = json["tag_name"].get<std::string>();
        latest_url = json["assets"][0]["browser_download_url"].get<std::string>();
    }
}

void CHMERUpdater::download_release(const std::string& url) {
    std::cout << "[Updater] Downloading: " << url << "\n";
    std::string cmd = "curl -L -o /tmp/chmer_update.tar.gz " + url;
    std::system(cmd.c_str());
}

void CHMERUpdater::extract_release() {
    std::cout << "[Updater] Extracting...\n";
    std::system("mkdir -p /tmp/chmer_update && tar -xzf /tmp/chmer_update.tar.gz -C /tmp/chmer_update");
}

void CHMERUpdater::run_setup() {
    std::cout << "[Updater] Running setup...\n";
    std::system("bash /tmp/chmer_update/setup.sh");
}

void CHMERUpdater::mark_installed() {
    std::ofstream marker(fs::path(getenv("HOME")) / ".chmer_installed_version");
    marker << latest_tag;
}

bool CHMERUpdater::is_installed() {
    std::ifstream marker(fs::path(getenv("HOME")) / ".chmer_installed_version");
    if(!marker.is_open()) return false;
    std::string installed_tag;
    marker >> installed_tag;
    return installed_tag == latest_tag;
}

void CHMERUpdater::check_for_updates(bool beta, bool force) {
    try {
        fetch_latest_release(beta);
        if(!force && is_installed()) {
            std::cout << "[Updater] Already up-to-date (" << latest_tag << ")\n";
            return;
        }
        download_release(latest_url);
        extract_release();
        run_setup();
        mark_installed();
        std::cout << "[Updater] Update completed!\n";
    } catch(std::exception& e) {
        std::cerr << "[Updater] Error: " << e.what() << "\n";
    }
}
