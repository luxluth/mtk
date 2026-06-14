#include <stdio.h>

#define MUSE_IMPLEMENTATION
#include "muse.h"

int main(void) {
  muContext ctx = {0};
  muNode root = muse_node_create(&ctx);
  muNode child = muse_node_create(&ctx);

  printf("@(%lu|%lu)\n", root.numeral, root.generation);
  printf("@(%lu|%lu)\n", child.numeral, child.generation);

  return 0;
}
