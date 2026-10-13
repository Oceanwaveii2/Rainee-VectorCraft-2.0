// Independent inference oracle compiled ONLY in the isolated research folder.
// Links the author's pinned Apache-2.0 forest files, not the solver/geometry dependencies.
#include <iomanip>
#include <iostream>
#include <vector>
double clf(const std::vector<double>&);
double clf_sec(const std::vector<double>&);
int main() {
    int count = 0;
    while (std::cin >> count) {
        if (count != 27 && count != 33) return 2;
        std::vector<double> features(count);
        for (double& value : features) if (!(std::cin >> value)) return 3;
        std::cout << std::setprecision(17) << (count == 27 ? clf(features) : clf_sec(features)) << '\n';
    }
}
