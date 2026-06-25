#include <stdio.h>

#define MUSE_IMPLEMENTATION
#include "muse.h"

int main(void) {
  muContext ctx = {0};

  muNode root = muse_node_create(&ctx);
  muNode child = muse_node_create(&ctx);

  muse_node_destroy(&ctx, child);

  child = muse_node_create(&ctx);

  if (muse_node_append(&ctx, root, child))
    printf("Appended\n");

  muse_root_attach(&ctx, root);

  printf("@(%lu|%lu)\n", root.numeral, root.generation);
  printf("@(%lu|%lu)\n", child.numeral, child.generation);

  muse_compute_layout(&ctx, 800, 600);

  return 0;
}
