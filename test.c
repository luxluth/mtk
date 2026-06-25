#include <raylib.h>

#define MUSE_IMPLEMENTATION
#include "muse.h"

static muTextComputedOutput raylib_text_sizing(muContext *ctx, muId text_node,
                                               float available_width,
                                               float available_height) {
  muText *text_comp = muse_sparse_get(&ctx->texts, text_node);
  if (!text_comp || !text_comp->data) {
    return (muTextComputedOutput){0};
  }

  Font default_font = GetFontDefault();
  float font_size = 20.0f;
  float spacing = 2.0f;

  Vector2 size =
      MeasureTextEx(default_font, text_comp->data, font_size, spacing);

  return (muTextComputedOutput){.computed_width = size.x,
                                .computed_height = size.y,
                                .baseline_offset = 0.0f};
}

static void draw_node_recursive(muContext *ctx, muNode node, int depth) {
  if (!muse_muid_is_valid(node))
    return;

  muComputed *comp = muse_sparse_get(&ctx->computed, node);
  if (comp) {
    Color bg = ColorFromHSV((float)(node.numeral * 45 % 360), 0.5f,
                            0.8f - (depth * 0.1f));
    DrawRectangleRec((Rectangle){comp->x, comp->y, comp->w, comp->h}, bg);
    DrawRectangleLinesEx((Rectangle){comp->x, comp->y, comp->w, comp->h}, 1.0f,
                         Fade(BLACK, 0.3f));

    muText *text_comp = muse_sparse_get(&ctx->texts, node);
    if (text_comp && text_comp->data) {
      DrawText(text_comp->data, (int)comp->x + 5, (int)comp->y + 5, 20, BLACK);
    }
  }

  muse_foreach_child(child, ctx, node) {
    draw_node_recursive(ctx, child, depth + 1);
  }
}

int main(void) {
  SetConfigFlags(FLAG_WINDOW_RESIZABLE);
  InitWindow(800, 600, "MUSE Engine - Layout Test");
  /* SetTargetFPS(60); */

  muContext ctx = {0};
  ctx.text_sizing_func = raylib_text_sizing;

  muNode root = muse_node_create(&ctx);
  muse_root_attach(&ctx, root);
  muse_constraints_set(&ctx, root,
                       (muConstraints){.dimension.width.kind = MU_PERCENT,
                                       .dimension.width.percent = 1.0f,
                                       .dimension.height.kind = MU_PERCENT,
                                       .dimension.height.percent = 1.0f,
                                       .gap = 10.0f,
                                       .flex_direction = MUSE_FLEX_ROW,
                                       .padding = 10.0f});

  muNode sidebar = muse_node_create(&ctx);
  muse_node_append(&ctx, root, sidebar);
  muse_constraints_set(&ctx, sidebar,
                       (muConstraints){.dimension.width.kind = MU_PERCENT,
                                       .dimension.width.percent = 0.2f,
                                       .dimension.height.kind = MU_FILL,
                                       .dimension.height.fill = true,
                                       .flex_direction = MUSE_FLEX_COLUMN,
                                       .padding = 10.0f});

  muNode main_content = muse_node_create(&ctx);
  muse_node_append(&ctx, root, main_content);
  muse_constraints_set(&ctx, main_content,
                       (muConstraints){.dimension.width.kind = MU_FILL,
                                       .dimension.width.fill = true,
                                       .dimension.height.kind = MU_FILL,
                                       .dimension.height.fill = true,
                                       .flex_direction = MUSE_FLEX_COLUMN,
                                       .padding = 20.0f});

  muNode text_block = muse_node_create(&ctx);
  muse_node_append(&ctx, main_content, text_block);
  muse_constraints_set(&ctx, text_block,
                       (muConstraints){.dimension.width.kind = MU_FIT,
                                       .dimension.width.fit = true,
                                       .dimension.height.kind = MU_FIT,
                                       .dimension.height.fit = true,
                                       .padding = 5.0f});

  muse_sparse_insert(&ctx.texts, text_block,
                     ((muText){.data = "Hello from MUSE"}));

  muse_compute_layout(&ctx, 800, 600);

  while (!WindowShouldClose()) {
    if (IsWindowResized()) {
      muse_node_set_dirty(&ctx, root);
    }

    muse_compute_layout(&ctx, (float)GetScreenWidth(),
                        (float)GetScreenHeight());

    BeginDrawing();
    ClearBackground(RAYWHITE);

    draw_node_recursive(&ctx, root, 0);

    EndDrawing();
  }

  muse_free_context(&ctx);
  CloseWindow();

  return 0;
}
