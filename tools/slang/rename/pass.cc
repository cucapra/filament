// Test pass to do some basic things using SLANG
#include <slang/syntax/SyntaxTree.h>
#include <slang/syntax/SyntaxVisitor.h>
#include <slang/ast/ASTVisitor.h>
#include <slang/ast/SemanticModel.h>
#include <slang/ast/Compilation.h>
#include <slang/parsing/ParserMetadata.h>
#include <slang/syntax/SyntaxPrinter.h>
#include <cstdio>
#include <string>
#include <set>
using namespace slang::syntax;

typedef struct
{
  std::string input;
  std::string output;
  std::vector<std::string> preserve;
} Args;

class RenamePass : public slang::syntax::SyntaxRewriter<RenamePass>
{
private:
  // std::set<std::string_view> preserve;

public:
  // RenamePass(const std::set<std::string_view> &to_preserve) : preserve(to_preserve)
  // {
  // }

  void handle(const slang::syntax::ModuleDeclarationSyntax &decl)
  {
    // // If the module name is in the set of names to preserve, don't rename it
    // if (preserve.find(decl.header->name.valueText()) != preserve.end())
    // {
    //   return;
    // }

    // // Create a new ModuleHeaderSyntax with the new name

    // auto newDecl = deepClone(decl, alloc);
    // newDecl->header->name = makeId("new_name", SingleSpace);
    // printf("Renaming module %s to new_name\n", std::string(decl.header->name.valueText()).c_str());
    // replace(decl, *newDecl);
    if (decl.header->name.valueText() == "m")
    {
      auto newMod = clone(decl, alloc);
      newMod->header->name = makeId("FooBar", SingleSpace);
      replace(decl, *newMod);
    }
  }
};

int main(int argc, char **argv)
{
  auto tree = SyntaxTree::fromFileInMemory(R"(
module m;
  module n;
  endmodule
  reg tmp;
  n n ();
  if (1) begin
  end
endmodule

module top (
    input clk
);
  m fooBar ();

  reg [3:0] a;

  always @(posedge clk) begin
    case (a)
      1: begin
        a <= 1;
      end
      2: begin
        a <= 2;
      end
    endcase
  end
endmodule

)",
                                           SyntaxTree::getDefaultSourceManager());

  assert(tree->diagnostics().empty());

  class ModuleChanger : public SyntaxRewriter<ModuleChanger>
  {
  public:
    void handle(const ModuleDeclarationSyntax &syntax)
    {
      printf("Header: %s\n", syntax.toString().c_str());
      for (auto &item : syntax.members)
      {
        printf("Member: %s\n", item->toString().c_str());
        visit(*item);
      }
      if (syntax.header->name.valueText() == "m")
      {
        auto newMod = clone(syntax, alloc);
        newMod->header->name = makeId("FooBar", SingleSpace);
        replace(syntax, *newMod);
      }
    }

    void handle(const MemberSyntax &syntax)
    {
      printf("Node: %s\n", syntax.toString().c_str());
      printf("Kind: %s\n", syntax.kind);
      // auto newMod = clone(syntax, alloc);
      // newMod->name = makeId("Egg", SingleSpace);
      // replace(syntax, *newMod);
    }
  };

  tree = ModuleChanger().transform(tree);

  std::string outputTree = SyntaxPrinter::printFile(*tree);

  printf("%s", outputTree.c_str());

  return 0;
  // /*
  // Run the renaming pass on the given file.
  // Arguments:
  //   <input file>
  //   <output file>
  //   <module names to preserve separated by commas (no spaces)>
  // */

  // // Parse command line arguments
  // if (argc <= 1)
  // {
  //   fprintf(stderr, "No arguments given\n");
  //   return 1;
  // }

  // // get a set of all the module names to preserve
  // std::set<std::string_view> preserve;
  // std::string input = argv[1];
  // std::string output = argv[2];
  // if (argc > 3)
  // {
  //   std::string sspreserve = argv[3];
  //   std::stringstream ss(sspreserve);
  //   std::string item;
  //   while (std::getline(ss, item, ','))
  //   {
  //     preserve.insert(item);
  //   }
  // }

  // // Parse the input file
  // auto tree_or_error = slang::syntax::SyntaxTree::fromFile(input);

  // if (!tree_or_error)
  // {
  //   fprintf(stderr, "Failed to parse input file %s:\n %s\n", input.c_str(), tree_or_error.error());
  //   return 1;
  // }

  // // Write the output file
  // FILE *fp = fopen(output.c_str(), "w");

  // if (fp == NULL)
  // {
  //   fprintf(stderr, "Failed to open output file %s\n", output.c_str());
  //   return 1;
  // }

  // auto tree = tree_or_error.value();

  // // Run the renaming pass
  // tree = RenamePass().transform(tree);

  // std::string outputTree = slang::syntax::SyntaxPrinter::printFile(*tree);

  // printf("%s", outputTree.c_str());

  // // fprintf(fp, "%s", outputTree.c_str());

  // return 0;
}