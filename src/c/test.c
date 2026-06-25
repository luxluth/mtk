#include <math.h>
#include <raylib.h>
#include <stdio.h>
#include <stdlib.h>

#define MUSE_IMPLEMENTATION
#include "muse.h"

static muTextComputedOutput raylib_text_sizing(muContext *ctx, muId text_node,
                                               float available_width,
                                               float available_height) {
  (void)available_width;
  (void)available_height;
  muText *text_comp = muse_text_get(ctx, text_node);
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

int main(void) {
  SetConfigFlags(FLAG_WINDOW_RESIZABLE | FLAG_VSYNC_HINT);
  InitWindow(800, 600, "MUSE Engine - Layout Test");
  SetTargetFPS(60);

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
                                       .padding = mu_edges_all(10.0f)});

  muNode sidebar = muse_node_create(&ctx);
  muse_node_append(&ctx, root, sidebar);
  muse_constraints_set(&ctx, sidebar,
                       (muConstraints){.dimension.width.kind = MU_PERCENT,
                                       .dimension.width.percent = 0.2f,
                                       .dimension.height.kind = MU_FILL,
                                       .dimension.height.fill = true,
                                       .overflow = MU_OVERFLOW_HIDDEN,
                                       .scroll = {0.0f, 0.0f},
                                       .flex_direction = MUSE_FLEX_COLUMN,
                                       .padding = mu_edges_all(10.0f)});

  char *texts = malloc(10000 * 32);
  muNode last_sidebar_item = {0};
  for (int i = 0; i < 10000; i++) {
    muNode item = muse_node_create(&ctx);
    muse_node_append(&ctx, sidebar, item);
    muse_constraints_set(
        &ctx, item,
        (muConstraints){.dimension.width.kind = MU_FILL,
                        .dimension.width.fill = true,
                        .dimension.height.kind = MU_FIXED,
                        .dimension.height.px = GetRandomValue(30, 100),
                        .padding = mu_edges_all(5.0f)});
    char *t = texts + i * 32;
    sprintf(t, "Track %d", i + 1);
    muse_text_set(&ctx, item, (muText){.data = t});
    last_sidebar_item = item;
  }

  muNode main_content = muse_node_create(&ctx);
  muse_node_append(&ctx, root, main_content);
  muse_constraints_set(&ctx, main_content,
                       (muConstraints){.dimension.width.kind = MU_FILL,
                                       .dimension.width.fill = true,
                                       .dimension.height.kind = MU_FILL,
                                       .dimension.height.fill = true,
                                       .flex_direction = MUSE_FLEX_COLUMN,
                                       .padding = mu_edges_all(20.0f)});

  muNode text_block = muse_node_create(&ctx);
  muse_node_append(&ctx, main_content, text_block);
  muse_constraints_set(&ctx, text_block,
                       (muConstraints){.dimension.width.kind = MU_FIT,
                                       .dimension.width.fit = true,
                                       .dimension.height.kind = MU_FIT,
                                       .dimension.height.fit = true,
                                       .padding = mu_edges_all(5.0f)});

  muse_text_set(&ctx, text_block, (muText){.data = "Hello from MUSE"});

  muse_compute_layout(&ctx, 800, 600);

  float target_scroll_y = 0.0f;
  float current_scroll_y = 0.0f;
  float scroll_active_timer = 0.0f;

  bool is_dragging_scrollbar = false;
  float drag_offset_y = 0.0f;

  while (!WindowShouldClose()) {
    if (IsWindowResized()) {
      muse_node_set_dirty(&ctx, root);
    }

    float max_scroll = 0.0f;
    muComputed *sidebar_comp = muse_computed_get(&ctx, sidebar);
    muComputed *last_item_comp = muse_computed_get(&ctx, last_sidebar_item);

    if (sidebar_comp && last_item_comp) {
      float content_height = (last_item_comp->y + last_item_comp->h) -
                             sidebar_comp->y + current_scroll_y;
      max_scroll =
          content_height - sidebar_comp->h + 10.0f; // +10 for bottom padding
      if (max_scroll < 0.0f)
        max_scroll = 0.0f;
    }

    float dt = GetFrameTime();
    float wheel = GetMouseWheelMove();
    Vector2 mouse_pos = GetMousePosition();

    // Scrollbar logic
    if (sidebar_comp && max_scroll > 0.0f) {
      float content_height = max_scroll + sidebar_comp->h;
      float scrollbar_h = (sidebar_comp->h / content_height) * sidebar_comp->h;
      if (scrollbar_h < 20.0f)
        scrollbar_h = 20.0f;
      float scrollable_track = sidebar_comp->h - scrollbar_h;

      float scroll_ratio = current_scroll_y / max_scroll;
      if (scroll_ratio < 0.0f)
        scroll_ratio = 0.0f;
      if (scroll_ratio > 1.0f)
        scroll_ratio = 1.0f;

      float scrollbar_y = sidebar_comp->y + scroll_ratio * scrollable_track;
      float scrollbar_x = sidebar_comp->x + sidebar_comp->w - 15.0f;
      float scrollbar_w = 10.0f;

      Rectangle sb_rect = {scrollbar_x, scrollbar_y, scrollbar_w, scrollbar_h};

      if (IsMouseButtonPressed(MOUSE_LEFT_BUTTON)) {
        if (CheckCollisionPointRec(mouse_pos, sb_rect)) {
          is_dragging_scrollbar = true;
          drag_offset_y = mouse_pos.y - scrollbar_y;
        }
      }

      if (IsMouseButtonReleased(MOUSE_LEFT_BUTTON)) {
        is_dragging_scrollbar = false;
      }

      if (is_dragging_scrollbar) {
        float new_scrollbar_y = mouse_pos.y - drag_offset_y;
        if (scrollable_track > 0.0f) {
          float new_ratio = (new_scrollbar_y - sidebar_comp->y) / scrollable_track;
          if (new_ratio < 0.0f)
            new_ratio = 0.0f;
          if (new_ratio > 1.0f)
            new_ratio = 1.0f;

          target_scroll_y = new_ratio * max_scroll;
          current_scroll_y = target_scroll_y; // Instant snap while dragging
        }
        wheel = 0.0f; // Disable wheel while dragging
      }
    }

    if (wheel != 0.0f) {
      float delta = -wheel * 150.0f;
      // Rubber-band resistance (stretching) when scrolling out of bounds
      if (target_scroll_y < 0.0f) {
        float overscroll = -target_scroll_y;
        float friction =
            1.0f /
            (1.0f + overscroll * 0.03f); // Stiffens rapidly to reduce travel
        delta *= friction;
      } else if (target_scroll_y > max_scroll) {
        float overscroll = target_scroll_y - max_scroll;
        float friction = 1.0f / (1.0f + overscroll * 0.03f);
        delta *= friction;
      }
      target_scroll_y += delta;

      // User is actively scrolling (or system is generating inertia events)
      scroll_active_timer = 0.15f;
    }

    if (scroll_active_timer > 0.0f) {
      scroll_active_timer -= dt;
    } else {
      // Rubber band target back to bounds with a soft, bouncy release
      // ONLY triggers when we are certain the user's swipe/scroll run has ended
      if (target_scroll_y < 0.0f) {
        target_scroll_y += (0.0f - target_scroll_y) * (1.0f - expf(-8.0f * dt));
      } else if (target_scroll_y > max_scroll) {
        target_scroll_y +=
            (max_scroll - target_scroll_y) * (1.0f - expf(-8.0f * dt));
      }
    }

    // Frame-rate independent exponential smoothing (logarithmic approach)
    float smoothing_factor = 1.0f - expf(-20.0f * dt); // Follow target snappily

    bool needs_scroll_update = false;

    if (fabsf(target_scroll_y - current_scroll_y) > 0.1f) {
      current_scroll_y += (target_scroll_y - current_scroll_y) * smoothing_factor;
      needs_scroll_update = true;
    } else if (is_dragging_scrollbar) {
      needs_scroll_update = true;
    }

    if (needs_scroll_update) {
      muConstraints *sidebar_cons = muse_constraints_get(&ctx, sidebar);
      if (sidebar_cons) {
        sidebar_cons->scroll.y = current_scroll_y;
        muse_node_set_dirty(&ctx, sidebar);
      }
    }

    muse_compute_layout(&ctx, (float)GetScreenWidth(),
                        (float)GetScreenHeight());
    muRect viewport = {0, 0, (float)GetScreenWidth(), (float)GetScreenHeight()};
    muse_build_render_list(&ctx, viewport);

    muNodeList hits = muse_node_pick(&ctx, mouse_pos.x, mouse_pos.y);
    muNode hovered = MUSE_UNDEFINED_MUID;
    if (hits.count > 0) {
      hovered = hits.items[0];
    }

    BeginDrawing();
    ClearBackground(RAYWHITE);

    for (size_t i = 0; i < ctx.render_list.count; i++) {
      muRenderCommand cmd = ctx.render_list.items[i];

      if (cmd.has_clip) {
        BeginScissorMode((int)cmd.clip.x, (int)cmd.clip.y, (int)cmd.clip.w,
                         (int)cmd.clip.h);
      }

      if (cmd.kind == MU_CMD_DRAWQUAD) {
        Color bg = ColorFromHSV((float)(cmd.node.numeral * 45 % 360), 0.5f,
                                muse_muid_eq(cmd.node, hovered) ? 1.0f : 0.8f);
        DrawRectangleRec((Rectangle){cmd.computed.x, cmd.computed.y,
                                     cmd.computed.w, cmd.computed.h},
                         bg);
        DrawRectangleLinesEx((Rectangle){cmd.computed.x, cmd.computed.y,
                                         cmd.computed.w, cmd.computed.h},
                             muse_muid_eq(cmd.node, hovered) ? 2.0f : 1.0f,
                             Fade(BLACK, 0.3f));
      } else if (cmd.kind == MU_CMD_TEXT) {
        if (cmd.info.text && cmd.info.text->data) {
          DrawText(cmd.info.text->data, (int)cmd.computed.x + 5,
                   (int)cmd.computed.y + 5, 20, BLACK);
        }
      }

      if (cmd.has_clip) {
        EndScissorMode();
      }
    }

    // Draw Scrollbar on top of render list
    muComputed *s_comp = muse_computed_get(&ctx, sidebar);
    if (s_comp && max_scroll > 0.0f) {
      float content_height = max_scroll + s_comp->h;
      float scrollbar_h = (s_comp->h / content_height) * s_comp->h;
      if (scrollbar_h < 20.0f)
        scrollbar_h = 20.0f;
      float scrollable_track = s_comp->h - scrollbar_h;

      float scroll_ratio = current_scroll_y / max_scroll;
      if (scroll_ratio < 0.0f)
        scroll_ratio = 0.0f;
      if (scroll_ratio > 1.0f)
        scroll_ratio = 1.0f;

      float scrollbar_y = s_comp->y + scroll_ratio * scrollable_track;
      float scrollbar_x = s_comp->x + s_comp->w - 15.0f;

      Color sb_color = is_dragging_scrollbar ? GRAY : LIGHTGRAY;
      DrawRectangleRec(
          (Rectangle){scrollbar_x, scrollbar_y, 10.0f, scrollbar_h}, sb_color);
    }

    muse_da_free(&hits);

    EndDrawing();
  }

  muse_context_free(&ctx);
  free(texts);
  CloseWindow();

  return 0;
}
