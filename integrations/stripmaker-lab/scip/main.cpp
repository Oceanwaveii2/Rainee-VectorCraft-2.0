// Contain exceptions in the separate research process, never in the editor.
#include <exception>
#include <iostream>
int rainee_author_main(int argc, char* argv[]);
int main(int argc, char** argv) {
  try { return rainee_author_main(argc, argv); }
  catch (const std::exception& e) { std::cerr << "StripMaker SCIP job failed: " << e.what() << '\n'; return 1; }
  catch (...) { std::cerr << "StripMaker SCIP job failed with an unknown exception\n"; return 2; }
}
