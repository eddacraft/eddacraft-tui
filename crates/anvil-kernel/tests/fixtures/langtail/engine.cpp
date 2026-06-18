// LANGTAIL T1 fixture — representative C++ source (modern-ish).
#include <string>
#include <vector>
#include <memory>

namespace engine {

class Renderer {
public:
    explicit Renderer(std::string name);
    std::string render(const std::vector<int>& items);
    void reset();

private:
    std::string name_;
};

struct Config {
    int width;
    int height;
};

enum class Backend {
    Software,
    Hardware
};

template <typename T>
T clampValue(T value, T lo, T hi) {
    return value < lo ? lo : (value > hi ? hi : value);
}

int globalSeed() {
    return 42;
}

}  // namespace engine
