#include "kernel/yosys.h"

USING_YOSYS_NAMESPACE
PRIVATE_NAMESPACE_BEGIN

static void prefix_module(Module *module, const std::string &prefix, const std::unordered_set<std::string> &ignore)
{

  if (!ignore.count(module->name.str()))
  {
    module->name = prefix + module->name.str();
  }

  for (auto &it : module->cells_)
  {
    Cell *cell = it.second;
    if (ignore.count(cell->name.str()))
      continue;
    cell->name = prefix + cell->name.str();
  }
}

struct PrefixPass : public Pass
{
  PrefixPass() : Pass("prefix", "add prefix to all modules") {}

  void help() override
  {
    log("\n");
    log("    prefix <prefix> <names to ignore separated by spaces>\n");
    log("\n");
    log("Add the given prefix to modules in the selected objects.\n");
    log("\n");
  }

  void execute(std::vector<std::string> args, Design *design) override
  {
    std::string prefix;

    log_header(design, "Executing PREFIX pass (add prefix to all identifiers).\n");

    if (args.size() < 2)
      log_cmd_error("Invalid number of arguments.\n");

    prefix = args[1];

    std::unordered_set<std::string> ignore;

    for (int i = 2; i < args.size(); i++)
    {
      std::string arg = args[i];
      ignore.insert(arg);
    }

    for (auto mod : design->modules_)
      prefix_module(mod.second, prefix, ignore);
  }
} PrefixPass;

PRIVATE_NAMESPACE_END