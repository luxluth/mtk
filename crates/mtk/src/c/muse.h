#ifndef MUSE_H_
#define MUSE_H_

#include <math.h>
#include <stdbool.h>
#include <stdint.h>

#ifndef MUSEDEF
#define MUSEDEF
#endif /* MUSEDEF */

//////////////////// da (Tsoding way)

#ifndef MUSE_ASSERT
#include <assert.h>
#define MUSE_ASSERT assert
#endif /* MUSE_ASSERT */

#ifndef MUSE_REALLOC
#include <stdlib.h>
#define MUSE_REALLOC realloc
#endif /* MUSE_REALLOC */

#ifndef MUSE_FREE
#include <stdlib.h>
#define MUSE_FREE free
#endif /* MUSE_FREE */

#ifdef __cplusplus
#define __MUSE_DECLTYPE_CAST(T) (decltype(T))
#else
#define __MUSE_DECLTYPE_CAST(T)
#endif /* __cplusplus */

#define MUSE_DA_INIT_CAP 256

#define muse_da_reserve(da, expected_capacity)                                 \
  do {                                                                         \
    if ((expected_capacity) > (da)->capacity) {                                \
      if ((da)->capacity == 0) {                                               \
        (da)->capacity = MUSE_DA_INIT_CAP;                                     \
      }                                                                        \
      while ((expected_capacity) > (da)->capacity) {                           \
        (da)->capacity *= 2;                                                   \
      }                                                                        \
      (da)->items = __MUSE_DECLTYPE_CAST((da)->items)                          \
          MUSE_REALLOC((da)->items, (da)->capacity * sizeof(*(da)->items));    \
      MUSE_ASSERT((da)->items != NULL && "Buy more RAM lol");                  \
    }                                                                          \
  } while (0)

#define muse_da_append(da, item)                                               \
  do {                                                                         \
    muse_da_reserve((da), (da)->count + 1);                                    \
    (da)->items[(da)->count++] = (item);                                       \
  } while (0)

#define muse_da_append_many(da, new_items, new_items_count)                    \
  do {                                                                         \
    muse_da_reserve((da), (da)->count + (new_items_count));                    \
    memcpy((da)->items + (da)->count, (new_items),                             \
           (new_items_count) * sizeof(*(da)->items));                          \
    (da)->count += (new_items_count);                                          \
  } while (0)

#define muse_da_free(da) MUSE_FREE((da)->items)
#define muse_da_foreach(Type, it, da)                                          \
  for (Type *it = (da)->items; it < (da)->items + (da)->count; ++it)

#define MUSE_DA(T)                                                             \
  struct {                                                                     \
    T *items;                                                                  \
    size_t count;                                                              \
    size_t capacity;                                                           \
  }

#define MUSE_TODO(message)                                                     \
  do {                                                                         \
    fprintf(stderr, "%s:%d: TODO: %s\n", __FILE__, __LINE__, message);         \
    abort();                                                                   \
  } while (0)
#define MUSE_UNREACHABLE(message)                                              \
  do {                                                                         \
    fprintf(stderr, "%s:%d: UNREACHABLE: %s\n", __FILE__, __LINE__, message);  \
    abort();                                                                   \
  } while (0)

////////////////////

////// SPARSE SET

#ifndef MUSE_SPARSE_NULL
#define MUSE_SPARSE_NULL SIZE_MAX
#endif

#define MUSE_SPARSE_SET(T)                                                     \
  struct {                                                                     \
    MUSE_DA(size_t) sparse;                                                    \
    MUSE_DA(muId) dense;                                                       \
    MUSE_DA(T) components;                                                     \
  }

#define muse_sparse_has(set, entity_id)                                        \
  ((entity_id).numeral < (set)->sparse.count &&                                \
   (set)->sparse.items[(entity_id).numeral] != MUSE_SPARSE_NULL &&             \
   (set)->dense.items[(set)->sparse.items[(entity_id).numeral]].generation ==  \
       (entity_id).generation)

#define muse_sparse_get(set, entity_id)                                        \
  (muse_sparse_has((set), (entity_id))                                         \
       ? &(set)->components.items[(set)->sparse.items[(entity_id).numeral]]    \
       : NULL)

#define muse_sparse_insert(set, entity_id, component)                          \
  do {                                                                         \
    if ((entity_id).numeral >= (set)->sparse.count) {                          \
      muse_da_reserve(&((set)->sparse), (entity_id).numeral + 1);              \
      while ((set)->sparse.count <= (entity_id).numeral) {                     \
        (set)->sparse.items[(set)->sparse.count++] = MUSE_SPARSE_NULL;         \
      }                                                                        \
    }                                                                          \
    if ((set)->sparse.items[(entity_id).numeral] == MUSE_SPARSE_NULL) {        \
      (set)->sparse.items[(entity_id).numeral] = (set)->dense.count;           \
      muse_da_append(&((set)->dense), (entity_id));                            \
      muse_da_append(&((set)->components), (component));                       \
    } else {                                                                   \
      size_t dense_idx = (set)->sparse.items[(entity_id).numeral];             \
      (set)->dense.items[dense_idx] = (entity_id);                             \
      (set)->components.items[dense_idx] = (component);                        \
    }                                                                          \
  } while (0)

#define muse_sparse_remove(set, entity_id)                                     \
  do {                                                                         \
    if (muse_sparse_has((set), (entity_id))) {                                 \
      size_t dense_idx = (set)->sparse.items[(entity_id).numeral];             \
      size_t last_idx = (set)->dense.count - 1;                                \
      muId last_entity = (set)->dense.items[last_idx];                         \
      (set)->dense.items[dense_idx] = last_entity;                             \
      (set)->components.items[dense_idx] = (set)->components.items[last_idx];  \
      (set)->sparse.items[last_entity.numeral] = dense_idx;                    \
      (set)->sparse.items[(entity_id).numeral] = MUSE_SPARSE_NULL;             \
      (set)->dense.count--;                                                    \
      (set)->components.count--;                                               \
    }                                                                          \
  } while (0)

#define muse_sparse_free(set)                                                  \
  do {                                                                         \
    muse_da_free(&((set)->sparse));                                            \
    muse_da_free(&((set)->dense));                                             \
    muse_da_free(&((set)->components));                                        \
  } while (0)

//////

#define muse_first_child(ctx, parent)                                          \
  (muse_sparse_has(&(ctx)->hierarchies, (parent))                              \
       ? muse_sparse_get(&(ctx)->hierarchies, (parent))->first_child           \
       : MUSE_UNDEFINED_MUID)

#define muse_last_child(ctx, parent)                                           \
  (muse_sparse_has(&(ctx)->hierarchies, (parent))                              \
       ? muse_sparse_get(&(ctx)->hierarchies, (parent))->last_child            \
       : MUSE_UNDEFINED_MUID)

#define muse_next_sibling(ctx, node)                                           \
  (muse_sparse_has(&(ctx)->hierarchies, (node))                                \
       ? muse_sparse_get(&(ctx)->hierarchies, (node))->next_sibling            \
       : MUSE_UNDEFINED_MUID)

#define muse_prev_sibling(ctx, node)                                           \
  (muse_sparse_has(&(ctx)->hierarchies, (node))                                \
       ? muse_sparse_get(&(ctx)->hierarchies, (node))->prev_sibling            \
       : MUSE_UNDEFINED_MUID)

#define muse_foreach_child(it_name, ctx, parent)                               \
  for (muNode it_name = muse_first_child((ctx), (parent));                     \
       muse_muid_is_valid(it_name);                                            \
       it_name = muse_next_sibling((ctx), it_name))

#define muse_foreach_child_reverse(it_name, ctx, parent)                       \
  for (muNode it_name = muse_last_child((ctx), (parent));                      \
       muse_muid_is_valid(it_name);                                            \
       it_name = muse_prev_sibling((ctx), it_name))

typedef enum {
  MU_PERCENT,
  MU_FIXED,
  MU_FILL,
  MU_FIT,
} muSizeKind;

typedef struct {
  muSizeKind kind;

  union {
    // The element's size is a fraction of its parent's size
    float percent;
    // The element has a hardcoded size
    uint32_t px;
    // The element consumes all remaining available space inside the parent
    // after other siblings are measured
    bool fill;
    // The element shrinks to tightly wrap its internal contents or children
    bool fit;
  };
} muSize;

typedef struct {
  float top;
  float bottom;
  float left;
  float right;
} muEdges;

#define mu_edges_all(v)                                                        \
  ((muEdges){.top = (v), .bottom = (v), .left = (v), .right = (v)})
#define mu_edges_lr(v)                                                         \
  ((muEdges){.top = 0, .bottom = 0, .left = (v), .right = (v)})
#define mu_edges_tb(v)                                                         \
  ((muEdges){.top = (v), .bottom = (v), .left = 0, .right = 0})

typedef struct {
  float x, y;
} muVector2;

typedef struct {
  float x, y, w, h;
} muRect;

typedef struct {
  size_t numeral, generation;
} muId;

typedef muId muNode;

#define MUSE_UNDEFINED_MUID                                                    \
  ((muId){.numeral = MUSE_SPARSE_NULL, .generation = MUSE_SPARSE_NULL})

// Get an invalid muid
MUSEDEF muNode muse_muid_invalid();

// Check if a muId|muNode is valid
MUSEDEF bool muse_muid_is_valid(muId id);
// Compare two muId|muNode to see if they match
MUSEDEF bool muse_muid_eq(muId a, muId b);

typedef MUSE_DA(muNode) muNodeList;

typedef struct {
  muNode parent;
  muNode first_child;
  muNode last_child;
  muNode next_sibling;
  muNode prev_sibling;
} muHierarchy;

#define MUSE_HIERARCHY_DEFAULT                                                 \
  ((muHierarchy){.parent = MUSE_UNDEFINED_MUID,                                \
                 .first_child = MUSE_UNDEFINED_MUID,                           \
                 .last_child = MUSE_UNDEFINED_MUID,                            \
                 .next_sibling = MUSE_UNDEFINED_MUID,                          \
                 .prev_sibling = MUSE_UNDEFINED_MUID})

typedef enum {
  MUSE_FLEX_ROW = 0,            // Left-to-Right
  MUSE_FLEX_COLUMN = 1,         // Top-to-Bottom
  MUSE_FLEX_ROW_REVERSE = 2,    // Right-to-Left
  MUSE_FLEX_COLUMN_REVERSE = 3, // Bottom-to-Top
} muFlexDirection;

typedef enum {
  MUSE_JUSTIFY_START = 0,
  MUSE_JUSTIFY_CENTER,
  MUSE_JUSTIFY_END,
  MUSE_JUSTIFY_SPACE_BETWEEN,
  MUSE_JUSTIFY_SPACE_AROUND,
  MUSE_JUSTIFY_SPACE_EVENLY
} muJustifyContent;

typedef enum {
  MUSE_ALIGN_START = 0,
  MUSE_ALIGN_CENTER,
  MUSE_ALIGN_END,
  MUSE_ALIGN_STRETCH
} muAlignItems;

typedef enum {
  MUSE_ALIGN_SELF_AUTO = 0,
  MUSE_ALIGN_SELF_START,
  MUSE_ALIGN_SELF_CENTER,
  MUSE_ALIGN_SELF_END,
  MUSE_ALIGN_SELF_STRETCH
} muAlignSelf;

typedef enum {
  MUSE_POSITION_STRATEGY_INFLOW = 0,
  MUSE_POSITION_STRATEGY_ABSOLUTE = 1,
} muPositionStrategyKind;

typedef struct {
  muPositionStrategyKind strategy;
  union {
    struct {
      float top;
      float left;
      float bottom;
      float right;
    } absolute;
  };
} muPositionStrategy;

typedef enum {
  MU_OVERFLOW_VISIBLE = 0,
  MU_OVERFLOW_HIDDEN,
  MU_OVERFLOW_SCROLL,
  MU_OVERFLOW_AUTO
} muOverflow;

typedef enum {
  MUSE_FLEX_NO_WRAP = 0,
  MUSE_FLEX_WRAP = 1,
  MUSE_FLEX_WRAP_REVERSE = 2,
} muFlexWrap;

typedef struct {
  struct {
    muSize width;
    muSize height;

    float min_width;
    float max_width;
    float min_height;
    float max_height;

    float aspect_ratio;
  } dimension;

  muPositionStrategy positioning;
  muFlexDirection flex_direction;
  muFlexWrap flex_wrap;

  muJustifyContent justify_content;
  muAlignItems align_items;
  muAlignSelf align_self;
  float gap;

  float flex_grow;
  float flex_shrink;
  muSize flex_basis;

  muEdges padding;
  muEdges border;

  muOverflow overflow;
  muVector2 scroll;
  int32_t z_index;
} muConstraints;

#define mu_position(s, ...) ((muPositionStrategy){.strategy = s, __VA_ARGS__})

#define mu_absolute(...)                                                       \
  mu_position(                                                                 \
      MUSE_POSITION_STRATEGY_ABSOLUTE,                                         \
      .absolute = {                                                            \
          .top = NAN, .left = NAN, .bottom = NAN, .right = NAN, __VA_ARGS__})

typedef struct {
  float x, y, w, h;
  float content_w, content_h;
} muComputed;

typedef struct {
  char dummy;
} muDirty;

typedef struct {
  // The actual horizontal space the text occupies
  float computed_width;
  // The total vertical space, accounting for all wrapped lines and line-height
  // spacing.
  float computed_height;
  // The distance from the top of the computed bounding box to the typographic
  // baseline
  // TODO: add alignement strategy (Not yet implememted)
  float baseline_offset;
} muTextComputedOutput;

typedef struct {
  char *data;
  void *userdata;
  float cached_avail_w;
  float cached_avail_h;
  muTextComputedOutput cached_output;
  bool is_cached;
} muText;

typedef struct muContext muContext;

typedef muTextComputedOutput muTextSizingFunc(muContext *ctx, muId text,
                                              float available_width,
                                              float available_height);

typedef enum {
  MU_CMD_DRAWQUAD,
  MU_CMD_TEXT,
  MU_CMD_SCROLLBAR_V,
  MU_CMD_SCROLLBAR_H,
} muRenderCommandKind;

typedef struct {
  muNode node;
  union {
    muText *text;
  } info;

  muComputed computed;
  muRect clip;
  muRenderCommandKind kind;

  int32_t z_index;
  bool has_clip;
} muRenderCommand;

typedef MUSE_DA(muRenderCommand) muRenderList;

typedef struct muContext {
  MUSE_SPARSE_SET(muHierarchy) hierarchies;
  MUSE_SPARSE_SET(muConstraints) constraints;
  MUSE_SPARSE_SET(muComputed) computed;
  MUSE_SPARSE_SET(muDirty) dirties;
  MUSE_SPARSE_SET(muText) texts;

  MUSE_DA(muId) available_ids;
  muRenderList render_list;
  muNodeList pick_list;
  muNodeList layout_order;

  muTextSizingFunc *text_sizing_func;
  size_t next_entity_numeral;

  muNode root;

  bool rooted; // Just to make it nicer to use
  bool render_list_dirty;
  bool layout_order_dirty;
  void *userdata;
} muContext;

// Deallocate the full context
MUSEDEF void muse_context_free(muContext *ctx);

// Set this node as the root of the tree
MUSEDEF void muse_root_attach(muContext *ctx, muNode node);
// Remove the current root (not cleaned up)
MUSEDEF void muse_root_drop(muContext *ctx);

// Append a child node to the end of the parent node tree
MUSEDEF bool muse_node_append(muContext *ctx, muNode parent, muNode child);
// Append a child node to the start of the parent node tree
MUSEDEF bool muse_node_prepend(muContext *ctx, muNode parent, muNode child);
// Detach a node from its parent but don't destroy it,
// ideal for moving element and appending them
// elsewhere. If you want to completly remove the node
// and its subsequent children use `muse_node_destroy`
MUSEDEF bool muse_node_remove(muContext *ctx, muNode node);
// Put a node after a designated sibling
MUSEDEF bool muse_node_put_after(muContext *ctx, muNode sibling, muNode node);
// Put a node before a designated sibling
MUSEDEF bool muse_node_put_before(muContext *ctx, muNode sibling, muNode node);

// Create a new valid node. It's not inserted in the tree but it exists
MUSEDEF muNode muse_node_create(muContext *ctx);
// Destroy a node from the tree removing it children at the same time
MUSEDEF void muse_node_destroy(muContext *ctx, muNode node);
// Get the parent of a node, or MUSE_UNDEFINED_MUID if detached or root
MUSEDEF muNode muse_node_parent(muContext *ctx, muNode node);
// Mark a node as dirty
MUSEDEF void muse_node_set_dirty(muContext *ctx, muNode node);
// Returns a list of nodes intersecting the X/Y coordinates, ordered
// front-to-back.
// NOTE: After processing interactions on the returned list, you
// may want to free the temporary list with `muse_da_free` but you do as you
// whish.
MUSEDEF muNodeList muse_node_pick(muContext *ctx, float x, float y);

// Add constraints or overwrite the current existing contraints on a node
MUSEDEF void muse_constraints_set(muContext *ctx, muNode node,
                                  muConstraints constraints);
// Get a pointer to a node constraints
// You may want to set the node as dirty afterwards
MUSEDEF muConstraints *muse_constraints_get(muContext *ctx, muNode node);

// Transform a node into a text element
MUSEDEF void muse_text_set(muContext *ctx, muNode node, muText text);
// Remove the ability of a node to act like a text
// I don't know why you'll need this but it is there
MUSEDEF void muse_text_unset(muContext *ctx, muNode node);
// Get a pointer to a `muText` from a node
// If you happen to modify the text you may want to also call
// `muse_node_set_dirty`
MUSEDEF muText *muse_text_get(muContext *ctx, muNode node);

// Compute the final layout filling up the context with muComputed
MUSEDEF void muse_compute_layout(muContext *ctx, float viewport_width,
                                 float viewport_height);

// Builds a flattened, Z-sorted array of commands to be consumed by the renderer
MUSEDEF void muse_build_render_list(muContext *ctx, muRect viewport);

// Get the computed bounding box and offset of the node
MUSEDEF muComputed *muse_computed_get(muContext *ctx, muNode node);
MUSEDEF muNode muse_first_child_get(muContext *ctx, muNode parent);
MUSEDEF muNode muse_next_sibling_get(muContext *ctx, muNode node);

#endif // MUSE_H_

#ifdef MUSE_IMPLEMENTATION

MUSEDEF muNode muse_muid_invalid() { return MUSE_UNDEFINED_MUID; }

MUSEDEF bool muse_muid_is_valid(muId id) {
  return id.numeral != MUSE_SPARSE_NULL && id.generation != MUSE_SPARSE_NULL;
}

MUSEDEF bool muse_muid_eq(muId a, muId b) {
  return (a.numeral == b.numeral) && (a.generation == b.generation);
}

MUSEDEF void muse_context_free(muContext *ctx) {
  muse_da_free(&ctx->available_ids);
  muse_da_free(&ctx->render_list);
  muse_da_free(&ctx->pick_list);
  muse_da_free(&ctx->layout_order);

  muse_sparse_free(&ctx->hierarchies);
  muse_sparse_free(&ctx->constraints);
  muse_sparse_free(&ctx->computed);
  muse_sparse_free(&ctx->dirties);
  muse_sparse_free(&ctx->texts);
}

MUSEDEF void muse_root_attach(muContext *ctx, muNode node) {
  ctx->root = node;
  ctx->rooted = true;
  ctx->layout_order_dirty = true;
  muse_node_set_dirty(ctx, node);
}

MUSEDEF void muse_root_drop(muContext *ctx) {
  ctx->root = MUSE_UNDEFINED_MUID;
  ctx->rooted = false;
  ctx->layout_order_dirty = true;
}

MUSEDEF muNode muse_node_create(muContext *ctx) {
  if (ctx->available_ids.count > 0) {
    muId id = ctx->available_ids.items[--ctx->available_ids.count];
    id.generation += 1;

    return id;
  }

  return ((muId){
      .numeral = ctx->next_entity_numeral++,
      .generation = 0,
  });
}

MUSEDEF void muse_node_destroy(muContext *ctx, muNode node) {
  if (!muse_muid_is_valid(node))
    return;

  muse_node_remove(ctx, node);
  muHierarchy *hrc = muse_sparse_get(&ctx->hierarchies, node);
  if (hrc != NULL) {
    muNode current_child = hrc->first_child;
    while (muse_muid_is_valid(current_child)) {
      muHierarchy *child_hrc =
          muse_sparse_get(&ctx->hierarchies, current_child);
      muNode next = child_hrc->next_sibling;

      muse_node_destroy(ctx, current_child);
      current_child = next;
    }
  }

  muse_sparse_remove(&ctx->computed, node);
  muse_sparse_remove(&ctx->constraints, node);
  muse_sparse_remove(&ctx->dirties, node);
  muse_sparse_remove(&ctx->hierarchies, node);
  muse_sparse_remove(&ctx->texts, node);

  muse_da_append(&ctx->available_ids, node);
  ctx->layout_order_dirty = true;
}

MUSEDEF bool muse_node_remove(muContext *ctx, muNode node) {
  if (!muse_muid_is_valid(node) || !muse_sparse_has(&ctx->hierarchies, node))
    return false;

  muHierarchy *current_hrc = muse_sparse_get(&ctx->hierarchies, node);
  muNode parent = current_hrc->parent;

  if (!muse_muid_is_valid(parent)) {
    return true;
  }

  muHierarchy *parent_hrc = muse_sparse_get(&ctx->hierarchies, parent);
  muNode prev = current_hrc->prev_sibling;
  muNode next = current_hrc->next_sibling;

  if (muse_muid_is_valid(prev)) {
    muHierarchy *prev_hrc = muse_sparse_get(&ctx->hierarchies, prev);
    prev_hrc->next_sibling = next;
  } else {
    // If there is no previous sibling, this node was the first child.
    parent_hrc->first_child = next;
  }

  if (muse_muid_is_valid(next)) {
    muHierarchy *next_hrc = muse_sparse_get(&ctx->hierarchies, next);
    next_hrc->prev_sibling = prev;
  } else {
    // If there is no next sibling, this node was the last child.
    parent_hrc->last_child = prev;
  }

  current_hrc->parent = MUSE_UNDEFINED_MUID;
  current_hrc->prev_sibling = MUSE_UNDEFINED_MUID;
  current_hrc->next_sibling = MUSE_UNDEFINED_MUID;

  ctx->layout_order_dirty = true;
  muse_node_set_dirty(ctx, parent);
  return true;
}

MUSEDEF muNode muse_node_parent(muContext *ctx, muNode node) {
  if (!muse_muid_is_valid(node) || !muse_sparse_has(&ctx->hierarchies, node))
    return MUSE_UNDEFINED_MUID;
  muHierarchy *hrc = muse_sparse_get(&ctx->hierarchies, node);
  return (hrc != NULL) ? hrc->parent : MUSE_UNDEFINED_MUID;
}

MUSEDEF bool muse_node_append(muContext *ctx, muNode parent, muNode child) {
  if (!muse_muid_is_valid(parent) || !muse_muid_is_valid(child)) {
    return false;
  }

  if (parent.numeral == child.numeral) {
    return false;
  }

  muse_node_remove(ctx, child);

  if (!muse_sparse_has(&ctx->hierarchies, parent)) {
    muse_sparse_insert(&ctx->hierarchies, parent, MUSE_HIERARCHY_DEFAULT);
  }
  if (!muse_sparse_has(&ctx->hierarchies, child)) {
    muse_sparse_insert(&ctx->hierarchies, child, MUSE_HIERARCHY_DEFAULT);
  }

  muHierarchy *parent_hrc = muse_sparse_get(&ctx->hierarchies, parent);
  muHierarchy *child_hrc = muse_sparse_get(&ctx->hierarchies, child);

  child_hrc->parent = parent;

  if (!muse_muid_is_valid(parent_hrc->first_child)) {
    // Case A: First and only child
    parent_hrc->first_child = child;
    parent_hrc->last_child = child;
  } else {
    // Case B: Append to existing siblings
    muNode last = parent_hrc->last_child;
    muHierarchy *last_hrc = muse_sparse_get(&ctx->hierarchies, last);

    last_hrc->next_sibling = child;
    child_hrc->prev_sibling = last;
    parent_hrc->last_child = child;
  }

  ctx->layout_order_dirty = true;
  muse_node_set_dirty(ctx, parent);

  return true;
}

MUSEDEF bool muse_node_prepend(muContext *ctx, muNode parent, muNode child) {
  if (!muse_muid_is_valid(parent) || !muse_muid_is_valid(child)) {
    return false;
  }

  if (parent.numeral == child.numeral) {
    return false;
  }

  muse_node_remove(ctx, child);

  if (!muse_sparse_has(&ctx->hierarchies, parent)) {
    muse_sparse_insert(&ctx->hierarchies, parent, MUSE_HIERARCHY_DEFAULT);
  }
  if (!muse_sparse_has(&ctx->hierarchies, child)) {
    muse_sparse_insert(&ctx->hierarchies, child, MUSE_HIERARCHY_DEFAULT);
  }

  muHierarchy *parent_hrc = muse_sparse_get(&ctx->hierarchies, parent);
  muHierarchy *child_hrc = muse_sparse_get(&ctx->hierarchies, child);

  child_hrc->parent = parent;

  if (!muse_muid_is_valid(parent_hrc->first_child)) {
    // Case A: First and only child
    parent_hrc->first_child = child;
    parent_hrc->last_child = child;
  } else {
    // Case B: Prepend to existing siblings
    muNode first = parent_hrc->first_child;
    muHierarchy *first_hrc = muse_sparse_get(&ctx->hierarchies, first);

    first_hrc->prev_sibling = child;
    child_hrc->next_sibling = first;
    parent_hrc->first_child = child;
  }

  ctx->layout_order_dirty = true;
  muse_node_set_dirty(ctx, parent);

  return true;
}

MUSEDEF bool muse_node_put_after(muContext *ctx, muNode sibling, muNode node) {
  if (!muse_muid_is_valid(sibling) || !muse_muid_is_valid(node)) {
    return false;
  }

  if (sibling.numeral == node.numeral) {
    return false;
  }

  muse_node_remove(ctx, node);

  if (!muse_sparse_has(&ctx->hierarchies, sibling)) {
    // Cannot attach to sibling with no parent
    return false;
  }

  if (!muse_sparse_has(&ctx->hierarchies, node)) {
    muse_sparse_insert(&ctx->hierarchies, node, MUSE_HIERARCHY_DEFAULT);
  }

  muHierarchy *sibling_hrc = muse_sparse_get(&ctx->hierarchies, sibling);
  muHierarchy *node_hrc = muse_sparse_get(&ctx->hierarchies, node);
  muNode parent = sibling_hrc->parent;

  if (!muse_muid_is_valid(parent)) {
    // Sibling got no parent - What kind of trickery are you doing ?
    return false;
  }

  muHierarchy *parent_hrc = muse_sparse_get(&ctx->hierarchies, parent);

  muNode sibling_next = sibling_hrc->next_sibling;
  node_hrc->parent = parent;

  if (muse_muid_is_valid(sibling_next)) {
    muHierarchy *sibling_next_hrc =
        muse_sparse_get(&ctx->hierarchies, sibling_next);
    node_hrc->next_sibling = sibling_next;
    sibling_next_hrc->prev_sibling = node;
  } else {
    // my prev sibling is last child
    parent_hrc->last_child = node;
  }

  sibling_hrc->next_sibling = node;
  node_hrc->prev_sibling = sibling;

  ctx->layout_order_dirty = true;
  muse_node_set_dirty(ctx, parent);

  return true;
}

MUSEDEF bool muse_node_put_before(muContext *ctx, muNode sibling, muNode node) {
  if (!muse_muid_is_valid(sibling) || !muse_muid_is_valid(node)) {
    return false;
  }

  if (sibling.numeral == node.numeral) {
    return false;
  }

  muse_node_remove(ctx, node);

  if (!muse_sparse_has(&ctx->hierarchies, sibling)) {
    // Cannot attach to sibling with no parent
    return false;
  }

  if (!muse_sparse_has(&ctx->hierarchies, node)) {
    muse_sparse_insert(&ctx->hierarchies, node, MUSE_HIERARCHY_DEFAULT);
  }

  muHierarchy *sibling_hrc = muse_sparse_get(&ctx->hierarchies, sibling);
  muHierarchy *node_hrc = muse_sparse_get(&ctx->hierarchies, node);
  muNode parent = sibling_hrc->parent;

  if (!muse_muid_is_valid(parent)) {
    // Sibling got no parent - Do you actually understand how this works ?
    return false;
  }

  muHierarchy *parent_hrc = muse_sparse_get(&ctx->hierarchies, parent);

  muNode sibling_prev = sibling_hrc->prev_sibling;
  node_hrc->parent = parent;

  if (muse_muid_is_valid(sibling_prev)) {
    muHierarchy *sibling_prev_hrc =
        muse_sparse_get(&ctx->hierarchies, sibling_prev);
    node_hrc->prev_sibling = sibling_prev;
    sibling_prev_hrc->next_sibling = node;
  } else {
    // my prev sibling is first child
    parent_hrc->first_child = node;
  }

  node_hrc->next_sibling = sibling;
  sibling_hrc->prev_sibling = node;

  ctx->layout_order_dirty = true;
  muse_node_set_dirty(ctx, parent);

  return true;
}

MUSEDEF void muse_node_set_dirty(muContext *ctx, muNode node) {
  if (!muse_muid_is_valid(node))
    return;
  muse_sparse_insert(&ctx->dirties, node, (muDirty){0});
  ctx->render_list_dirty = true;
}

MUSEDEF void muse_constraints_set(muContext *ctx, muNode node,
                                  muConstraints constraints) {
  if (!muse_muid_is_valid(node))
    return;

  muse_sparse_insert(&ctx->constraints, node, constraints);
  muse_node_set_dirty(ctx, node);
}

MUSEDEF muConstraints *muse_constraints_get(muContext *ctx, muNode node) {
  if (!muse_muid_is_valid(node))
    return NULL;

  if (!muse_sparse_has(&ctx->constraints, node))
    return NULL;

  return muse_sparse_get(&ctx->constraints, node);
}

MUSEDEF void muse_text_set(muContext *ctx, muNode node, muText text) {
  if (!muse_muid_is_valid(node))
    return;
  text.is_cached = false;
  text.cached_avail_w = -1.0f;
  text.cached_avail_h = -1.0f;
  muse_sparse_insert(&ctx->texts, node, text);
  muse_node_set_dirty(ctx, node);
}

MUSEDEF void muse_text_unset(muContext *ctx, muNode node) {
  if (!muse_muid_is_valid(node))
    return;

  muse_sparse_remove(&ctx->texts, node);
  muse_node_set_dirty(ctx, node);
}

MUSEDEF muText *muse_text_get(muContext *ctx, muNode node) {
  if (!muse_muid_is_valid(node))
    return NULL;

  if (!muse_sparse_has(&ctx->texts, node))
    return NULL;

  return muse_sparse_get(&ctx->texts, node);
}

MUSEDEF muComputed *muse_computed_get(muContext *ctx, muNode node) {
  if (!muse_muid_is_valid(node))
    return NULL;

  if (!muse_sparse_has(&ctx->computed, node))
    return NULL;

  return muse_sparse_get(&ctx->computed, node);
}

MUSEDEF muNode muse_first_child_get(muContext *ctx, muNode parent) {
  if (!ctx || !muse_muid_is_valid(parent) ||
      !muse_sparse_has(&ctx->hierarchies, parent))
    return (muNode){MUSE_SPARSE_NULL, MUSE_SPARSE_NULL};
  return muse_sparse_get(&ctx->hierarchies, parent)->first_child;
}

MUSEDEF muNode muse_next_sibling_get(muContext *ctx, muNode node) {
  if (!ctx || !muse_muid_is_valid(node) ||
      !muse_sparse_has(&ctx->hierarchies, node))
    return (muNode){MUSE_SPARSE_NULL, MUSE_SPARSE_NULL};
  return muse_sparse_get(&ctx->hierarchies, node)->next_sibling;
}

static void muse__m_clamp_min_max(muComputed *comp, muConstraints *cons) {
  if (cons->dimension.min_width > 0.0f && comp->w < cons->dimension.min_width) {
    comp->w = cons->dimension.min_width;
  }
  if (cons->dimension.max_width > 0.0f && comp->w > cons->dimension.max_width) {
    comp->w = cons->dimension.max_width;
  }

  if (cons->dimension.min_height > 0.0f &&
      comp->h < cons->dimension.min_height) {
    comp->h = cons->dimension.min_height;
  }
  if (cons->dimension.max_height > 0.0f &&
      comp->h > cons->dimension.max_height) {
    comp->h = cons->dimension.max_height;
  }
}

static void muse__m_apply_aspect_ratio(muComputed *comp, muConstraints *cons) {
  if (cons->dimension.aspect_ratio <= 0.0f)
    return;

  float ar = cons->dimension.aspect_ratio;
  bool is_w_fixed = (cons->dimension.width.kind == MU_FIXED ||
                     cons->dimension.width.kind == MU_PERCENT);
  bool is_h_fixed = (cons->dimension.height.kind == MU_FIXED ||
                     cons->dimension.height.kind == MU_PERCENT);

  // If author explicitly specified both fixed/percent width and height, respect
  // them
  if (is_w_fixed && is_h_fixed) {
    return;
  }

  if (is_w_fixed && comp->w > 0.0f) {
    comp->h = comp->w / ar;
    muse__m_clamp_min_max(comp, cons);
    return;
  }

  if (is_h_fixed && comp->h > 0.0f) {
    comp->w = comp->h * ar;
    muse__m_clamp_min_max(comp, cons);
    return;
  }

  // When one dimension has a resolved non-zero value and the other is fit/zero
  if (comp->w > 0.0f &&
      (comp->h == 0.0f || cons->dimension.height.kind == MU_FIT)) {
    comp->h = comp->w / ar;
    muse__m_clamp_min_max(comp, cons);
  } else if (comp->h > 0.0f &&
             (comp->w == 0.0f || cons->dimension.width.kind == MU_FIT)) {
    comp->w = comp->h * ar;
    muse__m_clamp_min_max(comp, cons);
  }
}

static void muse__m_collect_layout_order(muContext *ctx, muNode node) {
  if (!muse_muid_is_valid(node))
    return;
  muse_da_append(&ctx->layout_order, node);
  muse_foreach_child(child, ctx, node) {
    muse__m_collect_layout_order(ctx, child);
  }
}

static void muse__m_ensure_layout_order(muContext *ctx) {
  if (!ctx->layout_order_dirty && ctx->layout_order.count > 0)
    return;
  ctx->layout_order.count = 0;
  if (ctx->rooted && muse_muid_is_valid(ctx->root)) {
    muse__m_collect_layout_order(ctx, ctx->root);
  }
  ctx->layout_order_dirty = false;
}

static void muse__m_compute_top_down_node(muContext *ctx, muNode node,
                                          muComputed parent_bounds) {
  if (!muse_muid_is_valid(node))
    return;

  if (!muse_sparse_has(&ctx->computed, node)) {
    muse_sparse_insert(&ctx->computed, node, (muComputed){0});
  }

  muComputed *comp = muse_sparse_get(&ctx->computed, node);
  muConstraints *cons = muse_sparse_get(&ctx->constraints, node);

  if (cons != NULL && muse_sparse_has(&ctx->dirties, node)) {
    bool is_root = muse_muid_eq(node, ctx->root);

    // WIDTH
    if (cons->dimension.width.kind == MU_FIXED) {
      comp->w = (float)cons->dimension.width.px;
    } else if (cons->dimension.width.kind == MU_PERCENT) {
      comp->w = parent_bounds.w * cons->dimension.width.percent;
    } else if (is_root && cons->dimension.width.kind == MU_FILL) {
      comp->w = parent_bounds.w;
    } else {
      // MU_FIT or MU_FILL
      comp->w = 0.0f;
    }

    // HEIGHT
    if (cons->dimension.height.kind == MU_FIXED) {
      comp->h = (float)cons->dimension.height.px;
    } else if (cons->dimension.height.kind == MU_PERCENT) {
      comp->h = parent_bounds.h * cons->dimension.height.percent;
    } else if (is_root && cons->dimension.height.kind == MU_FILL) {
      comp->h = parent_bounds.h;
    } else {
      comp->h = 0.0f;
    }

    muse__m_clamp_min_max(comp, cons);
    muse__m_apply_aspect_ratio(comp, cons);

    // ABSOLUTE POSITIONING
    if (cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE) {
      bool has_left = !isnan(cons->positioning.absolute.left);
      bool has_right = !isnan(cons->positioning.absolute.right);
      bool has_top = !isnan(cons->positioning.absolute.top);
      bool has_bottom = !isnan(cons->positioning.absolute.bottom);

      if (has_left && has_right) {
        if (cons->dimension.width.kind == MU_FIT ||
            cons->dimension.width.kind == MU_FILL) {
          comp->w = parent_bounds.w - cons->positioning.absolute.left -
                    cons->positioning.absolute.right;
          comp->x = parent_bounds.x + cons->positioning.absolute.left;
        } else {
          // Left wins
          comp->x = parent_bounds.x + cons->positioning.absolute.left;
        }
      } else if (has_left) {
        comp->x = parent_bounds.x + cons->positioning.absolute.left;
      } else if (has_right) {
        comp->x = parent_bounds.x + parent_bounds.w -
                  cons->positioning.absolute.right - comp->w;
      }

      if (has_top && has_bottom) {
        if (cons->dimension.height.kind == MU_FIT ||
            cons->dimension.height.kind == MU_FILL) {
          comp->h = parent_bounds.h - cons->positioning.absolute.top -
                    cons->positioning.absolute.bottom;
          comp->y = parent_bounds.y + cons->positioning.absolute.top;
        } else {
          // Top wins
          comp->y = parent_bounds.y + cons->positioning.absolute.top;
        }
      } else if (has_top) {
        comp->y = parent_bounds.y + cons->positioning.absolute.top;
      } else if (has_bottom) {
        comp->y = parent_bounds.y + parent_bounds.h -
                  cons->positioning.absolute.bottom - comp->h;
      }
    }
  }
}

static inline muTextComputedOutput muse__m_get_text_size(muContext *ctx,
                                                         muNode node,
                                                         float avail_w,
                                                         float avail_h) {
  muText *txt = muse_sparse_get(&ctx->texts, node);
  if (txt != NULL && txt->is_cached && txt->cached_avail_w == avail_w &&
      txt->cached_avail_h == avail_h) {
    return txt->cached_output;
  }
  muTextComputedOutput output =
      ctx->text_sizing_func(ctx, node, avail_w, avail_h);
  if (txt != NULL) {
    txt->cached_avail_w = avail_w;
    txt->cached_avail_h = avail_h;
    txt->cached_output = output;
    txt->is_cached = true;
  }
  return output;
}

static inline bool muse__is_row(muFlexDirection dir) {
  return dir == MUSE_FLEX_ROW || dir == MUSE_FLEX_ROW_REVERSE;
}

static inline bool muse__is_column(muFlexDirection dir) {
  return dir == MUSE_FLEX_COLUMN || dir == MUSE_FLEX_COLUMN_REVERSE;
}

static inline bool muse__is_reverse(muFlexDirection dir) {
  return dir == MUSE_FLEX_ROW_REVERSE || dir == MUSE_FLEX_COLUMN_REVERSE;
}

static void muse__m_compute_bottom_up_node(muContext *ctx, muNode node) {
  if (!muse_muid_is_valid(node))
    return;

  muConstraints *cons = muse_sparse_get(&ctx->constraints, node);
  muComputed *comp = muse_sparse_get(&ctx->computed, node);

  if (cons != NULL && comp != NULL) {
    bool fit_w = cons->dimension.width.kind == MU_FIT;
    bool fit_h = cons->dimension.height.kind == MU_FIT;

    if (fit_w || fit_h) {
      float intrinsic_w = 0.0f;
      float intrinsic_h = 0.0f;

      if (muse_sparse_has(&ctx->texts, node) && ctx->text_sizing_func != NULL) {
        float off_w = cons->padding.left + cons->border.left +
                      cons->padding.right + cons->border.right;
        float off_h = cons->padding.top + cons->border.top +
                      cons->padding.bottom + cons->border.bottom;
        float avail_w =
            fit_w ? INFINITY : (comp->w > off_w ? comp->w - off_w : comp->w);
        float avail_h =
            fit_h ? INFINITY : (comp->h > off_h ? comp->h - off_h : comp->h);

        muTextComputedOutput text_size =
            muse__m_get_text_size(ctx, node, avail_w, avail_h);

        intrinsic_w = text_size.computed_width;
        intrinsic_h = text_size.computed_height;
      } else {
        bool is_row_dir = muse__is_row(cons->flex_direction);
        bool is_wrapping = (cons->flex_wrap == MUSE_FLEX_WRAP ||
                            cons->flex_wrap == MUSE_FLEX_WRAP_REVERSE);

        if (!is_wrapping) {
          float sum_main = 0.0f;
          float max_cross = 0.0f;
          int child_count = 0;

          muse_foreach_child(child, ctx, node) {
            muConstraints *c_cons = muse_sparse_get(&ctx->constraints, child);
            if (c_cons &&
                c_cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE)
              continue;

            muComputed *c_comp = muse_sparse_get(&ctx->computed, child);
            if (!c_comp)
              continue;

            child_count += 1;

            if (is_row_dir) {
              sum_main += c_comp->w;
              if (c_comp->h > max_cross)
                max_cross = c_comp->h;
            } else {
              sum_main += c_comp->h;
              if (c_comp->w > max_cross)
                max_cross = c_comp->w;
            }
          }

          if (child_count > 1) {
            sum_main += cons->gap * (child_count - 1);
          }

          intrinsic_w = is_row_dir ? sum_main : max_cross;
          intrinsic_h = !is_row_dir ? sum_main : max_cross;
        } else {
          float off_w = cons->padding.left + cons->border.left +
                        cons->padding.right + cons->border.right;
          float off_h = cons->padding.top + cons->border.top +
                        cons->padding.bottom + cons->border.bottom;
          float max_line_main =
              is_row_dir ? (comp->w - off_w) : (comp->h - off_h);
          if (max_line_main <= 0.0f)
            max_line_main = INFINITY;

          float cur_line_main = 0.0f;
          float cur_line_cross = 0.0f;
          int cur_line_count = 0;
          float total_cross = 0.0f;
          float max_main_used = 0.0f;
          int line_count = 0;

          muse_foreach_child(child, ctx, node) {
            muConstraints *c_cons = muse_sparse_get(&ctx->constraints, child);
            if (c_cons &&
                c_cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE)
              continue;

            muComputed *c_comp = muse_sparse_get(&ctx->computed, child);
            if (!c_comp)
              continue;

            float child_main = is_row_dir ? c_comp->w : c_comp->h;
            float child_cross = is_row_dir ? c_comp->h : c_comp->w;

            float needed = child_main + (cur_line_count > 0 ? cons->gap : 0.0f);
            if (cur_line_count > 0 && cur_line_main + needed > max_line_main) {
              total_cross += cur_line_cross;
              line_count++;
              if (cur_line_main > max_main_used)
                max_main_used = cur_line_main;

              cur_line_main = child_main;
              cur_line_cross = child_cross;
              cur_line_count = 1;
            } else {
              cur_line_main += needed;
              if (child_cross > cur_line_cross)
                cur_line_cross = child_cross;
              cur_line_count++;
            }
          }

          if (cur_line_count > 0) {
            total_cross += cur_line_cross;
            line_count++;
            if (cur_line_main > max_main_used)
              max_main_used = cur_line_main;
          }

          if (line_count > 1) {
            total_cross += cons->gap * (line_count - 1);
          }

          intrinsic_w = is_row_dir ? max_main_used : total_cross;
          intrinsic_h = !is_row_dir ? max_main_used : total_cross;
        }
      }

      float off_w = cons->padding.left + cons->border.left +
                    cons->padding.right + cons->border.right;
      float off_h = cons->padding.top + cons->border.top +
                    cons->padding.bottom + cons->border.bottom;

      if (fit_w)
        comp->w = intrinsic_w + off_w;
      if (fit_h)
        comp->h = intrinsic_h + off_h;

      muse__m_clamp_min_max(comp, cons);
      muse__m_apply_aspect_ratio(comp, cons);
    }
  }
}

static inline float muse__m_get_flex_basis(muConstraints *c_cons,
                                           muComputed *c_comp, bool is_row_dir,
                                           float inner_main) {
  if (c_cons->flex_basis.kind == MU_PERCENT) {
    return inner_main * c_cons->flex_basis.percent;
  } else if (c_cons->flex_basis.kind == MU_FIXED) {
    return (float)c_cons->flex_basis.px;
  } else if (is_row_dir && c_cons->dimension.width.kind == MU_PERCENT) {
    return inner_main * c_cons->dimension.width.percent;
  } else if (!is_row_dir && c_cons->dimension.height.kind == MU_PERCENT) {
    return inner_main * c_cons->dimension.height.percent;
  } else if (is_row_dir && c_cons->dimension.width.kind == MU_FIXED) {
    return (float)c_cons->dimension.width.px;
  } else if (!is_row_dir && c_cons->dimension.height.kind == MU_FIXED) {
    return (float)c_cons->dimension.height.px;
  } else {
    bool is_main_fill = is_row_dir ? (c_cons->dimension.width.kind == MU_FILL)
                                   : (c_cons->dimension.height.kind == MU_FILL);
    if (c_cons->flex_grow > 0.0f || is_main_fill) {
      return 0.0f;
    }
    if (c_cons->dimension.aspect_ratio > 0.0f) {
      if (is_row_dir && c_comp->w == 0.0f && c_comp->h > 0.0f) {
        return c_comp->h * c_cons->dimension.aspect_ratio;
      } else if (!is_row_dir && c_comp->h == 0.0f && c_comp->w > 0.0f) {
        return c_comp->w / c_cons->dimension.aspect_ratio;
      }
    }
    return is_row_dir ? c_comp->w : c_comp->h;
  }
}

static void muse__m_compute_flex_distribution_node(muContext *ctx,
                                                   muNode node) {
  if (!muse_muid_is_valid(node))
    return;

  muConstraints *cons = muse_sparse_get(&ctx->constraints, node);
  muComputed *comp = muse_sparse_get(&ctx->computed, node);

  if (cons != NULL && comp != NULL) {
    float off_w = cons->padding.left + cons->border.left + cons->padding.right +
                  cons->border.right;
    float off_h = cons->padding.top + cons->border.top + cons->padding.bottom +
                  cons->border.bottom;

    float inner_w = comp->w - off_w;
    float inner_h = comp->h - off_h;
    if (inner_w < 0.0f)
      inner_w = 0.0f;
    if (inner_h < 0.0f)
      inner_h = 0.0f;

    bool is_row_dir = muse__is_row(cons->flex_direction);
    float available_main = is_row_dir ? inner_w : inner_h;

    // A) Pre-resolve cross-axis dimensions (Stretch, Fill, Percent) and
    // main-axis percentages
    muse_foreach_child(child, ctx, node) {
      muConstraints *c_cons = muse_sparse_get(&ctx->constraints, child);
      muComputed *c_comp = muse_sparse_get(&ctx->computed, child);
      if (c_cons == NULL || c_comp == NULL ||
          c_cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE)
        continue;

      muAlignItems effective_align = cons->align_items;
      if (c_cons->align_self != MUSE_ALIGN_SELF_AUTO) {
        switch (c_cons->align_self) {
        case MUSE_ALIGN_SELF_START:
          effective_align = MUSE_ALIGN_START;
          break;
        case MUSE_ALIGN_SELF_CENTER:
          effective_align = MUSE_ALIGN_CENTER;
          break;
        case MUSE_ALIGN_SELF_END:
          effective_align = MUSE_ALIGN_END;
          break;
        case MUSE_ALIGN_SELF_STRETCH:
          effective_align = MUSE_ALIGN_STRETCH;
          break;
        default:
          break;
        }
      }

      bool modified = false;
      if (!is_row_dir) {
        // Parent is Column. Cross-axis is Width.
        if (c_cons->dimension.width.kind == MU_FILL ||
            effective_align == MUSE_ALIGN_STRETCH) {
          if (c_cons->dimension.width.kind != MU_FIXED) {
            c_comp->w = inner_w;
            modified = true;
          }
        } else if (c_cons->dimension.width.kind == MU_PERCENT) {
          c_comp->w = inner_w * c_cons->dimension.width.percent;
          modified = true;
        }

        // Main-axis percentage for Column (Height)
        if (c_cons->dimension.height.kind == MU_PERCENT) {
          c_comp->h = inner_h * c_cons->dimension.height.percent;
          modified = true;
        }
      } else {
        // Parent is Row. Cross-axis is Height.
        if (c_cons->dimension.height.kind == MU_FILL ||
            effective_align == MUSE_ALIGN_STRETCH) {
          if (c_cons->dimension.height.kind != MU_FIXED) {
            c_comp->h = inner_h;
            modified = true;
          }
        } else if (c_cons->dimension.height.kind == MU_PERCENT) {
          c_comp->h = inner_h * c_cons->dimension.height.percent;
          modified = true;
        }

        // Main-axis percentage for Row (Width)
        if (c_cons->dimension.width.kind == MU_PERCENT) {
          c_comp->w = inner_w * c_cons->dimension.width.percent;
          modified = true;
        }
      }

      if (modified || c_cons->dimension.aspect_ratio > 0.0f) {
        muse__m_clamp_min_max(c_comp, c_cons);
        muse__m_apply_aspect_ratio(c_comp, c_cons);
      }

      // Re-measure wrapped text with resolved available width including padding
      // and border
      if (muse_sparse_has(&ctx->texts, child) &&
          ctx->text_sizing_func != NULL) {
        float c_off_w = c_cons->padding.left + c_cons->border.left +
                        c_cons->padding.right + c_cons->border.right;
        float c_off_h = c_cons->padding.top + c_cons->border.top +
                        c_cons->padding.bottom + c_cons->border.bottom;
        float c_avail_w =
            (c_comp->w > c_off_w) ? (c_comp->w - c_off_w) : INFINITY;
        float c_avail_h =
            (c_comp->h > c_off_h) ? (c_comp->h - c_off_h) : INFINITY;

        if (c_cons->dimension.width.kind == MU_FIT) {
          c_avail_w = INFINITY;
        }

        muTextComputedOutput text_size =
            muse__m_get_text_size(ctx, child, c_avail_w, c_avail_h);

        float needed_w = text_size.computed_width + c_off_w;
        float needed_h = text_size.computed_height + c_off_h;

        if (c_cons->dimension.width.kind == MU_FIT) {
          c_comp->w = needed_w;
        }
        if (c_cons->dimension.height.kind == MU_FIT || needed_h > c_comp->h) {
          c_comp->h = needed_h;
        }
        muse__m_clamp_min_max(c_comp, c_cons);
      }
    }

    // B) Calculate total flex basis and flex grow/shrink sums
    float total_basis = 0.0f;
    float total_flex_grow = 0.0f;
    int in_flow_count = 0;

    muse_foreach_child(child, ctx, node) {
      muConstraints *c_cons = muse_sparse_get(&ctx->constraints, child);
      muComputed *c_comp = muse_sparse_get(&ctx->computed, child);
      if (c_cons == NULL || c_comp == NULL ||
          c_cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE)
        continue;

      in_flow_count += 1;
      float basis =
          muse__m_get_flex_basis(c_cons, c_comp, is_row_dir, available_main);
      total_basis += basis;

      bool is_main_fill = is_row_dir
                              ? (c_cons->dimension.width.kind == MU_FILL)
                              : (c_cons->dimension.height.kind == MU_FILL);
      float grow = (c_cons->flex_grow > 0.0f) ? c_cons->flex_grow
                                              : (is_main_fill ? 1.0f : 0.0f);
      total_flex_grow += grow;
    }

    if (in_flow_count > 1) {
      total_basis += cons->gap * (in_flow_count - 1);
    }

    float free_space = available_main - total_basis;

    if (total_flex_grow > 0.0f && free_space > 0.0f) {
      // C1) Distribute free space
      muse_foreach_child(child, ctx, node) {
        muConstraints *c_cons = muse_sparse_get(&ctx->constraints, child);
        muComputed *c_comp = muse_sparse_get(&ctx->computed, child);
        if (c_cons == NULL || c_comp == NULL ||
            c_cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE)
          continue;

        bool is_main_fill = is_row_dir
                                ? (c_cons->dimension.width.kind == MU_FILL)
                                : (c_cons->dimension.height.kind == MU_FILL);
        float grow = (c_cons->flex_grow > 0.0f) ? c_cons->flex_grow
                                                : (is_main_fill ? 1.0f : 0.0f);
        if (grow > 0.0f) {
          float basis = muse__m_get_flex_basis(c_cons, c_comp, is_row_dir,
                                               available_main);
          float allocated = basis + (grow / total_flex_grow) * free_space;
          if (is_row_dir) {
            c_comp->w = allocated;
          } else {
            c_comp->h = allocated;
          }
          muse__m_clamp_min_max(c_comp, c_cons);
          muse__m_apply_aspect_ratio(c_comp, c_cons);
        }
      }
    } else if (free_space < 0.0f && available_main > 0.0f &&
               cons->overflow != MU_OVERFLOW_SCROLL &&
               cons->overflow != MU_OVERFLOW_AUTO) {
      // C2) Shrink space weighted by flex_shrink * basis
      float overflow_space = -free_space;
      float total_scaled_shrink = 0.0f;

      muse_foreach_child(child, ctx, node) {
        muConstraints *c_cons = muse_sparse_get(&ctx->constraints, child);
        muComputed *c_comp = muse_sparse_get(&ctx->computed, child);
        if (c_cons == NULL || c_comp == NULL ||
            c_cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE)
          continue;

        float basis =
            muse__m_get_flex_basis(c_cons, c_comp, is_row_dir, available_main);
        if (c_cons->flex_shrink > 0.0f && basis > 0.0f) {
          total_scaled_shrink += c_cons->flex_shrink * basis;
        }
      }

      if (total_scaled_shrink > 0.0f) {
        muse_foreach_child(child, ctx, node) {
          muConstraints *c_cons = muse_sparse_get(&ctx->constraints, child);
          muComputed *c_comp = muse_sparse_get(&ctx->computed, child);
          if (c_cons == NULL || c_comp == NULL ||
              c_cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE)
            continue;

          float basis = muse__m_get_flex_basis(c_cons, c_comp, is_row_dir,
                                               available_main);
          if (c_cons->flex_shrink > 0.0f && basis > 0.0f) {
            float shrink_ratio =
                (c_cons->flex_shrink * basis) / total_scaled_shrink;
            float shrink_amount = shrink_ratio * overflow_space;
            float new_main = basis - shrink_amount;
            if (new_main < 0.0f)
              new_main = 0.0f;
            if (is_row_dir) {
              c_comp->w = new_main;
            } else {
              c_comp->h = new_main;
            }
            muse__m_clamp_min_max(c_comp, c_cons);
            muse__m_apply_aspect_ratio(c_comp, c_cons);
          }
        }
      }
    }

    // D) Resolve percent and fill dimensions for absolute positioned children
    muse_foreach_child(child, ctx, node) {
      muConstraints *c_cons = muse_sparse_get(&ctx->constraints, child);
      muComputed *c_comp = muse_sparse_get(&ctx->computed, child);
      if (c_cons != NULL && c_comp != NULL &&
          c_cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE) {
        if (c_cons->dimension.width.kind == MU_PERCENT) {
          c_comp->w = inner_w * c_cons->dimension.width.percent;
        } else if (c_cons->dimension.width.kind == MU_FILL) {
          c_comp->w = inner_w;
        }
        if (c_cons->dimension.height.kind == MU_PERCENT) {
          c_comp->h = inner_h * c_cons->dimension.height.percent;
        } else if (c_cons->dimension.height.kind == MU_FILL) {
          c_comp->h = inner_h;
        }
        muse__m_clamp_min_max(c_comp, c_cons);
        muse__m_apply_aspect_ratio(c_comp, c_cons);
      }
    }

    // E) Recompute wrapping container cross dimension once inner_w is resolved
    if ((cons->flex_wrap == MUSE_FLEX_WRAP ||
         cons->flex_wrap == MUSE_FLEX_WRAP_REVERSE) &&
        is_row_dir && cons->dimension.height.kind == MU_FIT && inner_w > 0.0f) {
      float line_w = 0.0f;
      float line_max_h = 0.0f;
      float total_wrap_h = 0.0f;
      int line_items = 0;
      int lines = 0;
      muse_foreach_child(c, ctx, node) {
        muConstraints *c_cons = muse_sparse_get(&ctx->constraints, c);
        if (c_cons &&
            c_cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE)
          continue;
        muComputed *c_comp = muse_sparse_get(&ctx->computed, c);
        if (!c_comp)
          continue;
        float needed = c_comp->w + (line_items > 0 ? cons->gap : 0.0f);
        if (line_items > 0 && line_w + needed > inner_w) {
          total_wrap_h += line_max_h;
          lines++;
          line_w = c_comp->w;
          line_max_h = c_comp->h;
          line_items = 1;
        } else {
          line_w += needed;
          if (c_comp->h > line_max_h)
            line_max_h = c_comp->h;
          line_items++;
        }
      }
      if (line_items > 0) {
        total_wrap_h += line_max_h;
        lines++;
      }
      if (lines > 1) {
        total_wrap_h += cons->gap * (lines - 1);
      }
      comp->h = total_wrap_h + off_h;
      muse__m_clamp_min_max(comp, cons);
    }
  }
}

static void muse__m_compute_positional_alignment_node(muContext *ctx,
                                                      muNode node) {
  if (!muse_muid_is_valid(node))
    return;

  muConstraints *cons = muse_sparse_get(&ctx->constraints, node);
  muComputed *comp = muse_sparse_get(&ctx->computed, node);

  if (cons != NULL && comp != NULL) {
    float off_l = cons->padding.left + cons->border.left;
    float off_t = cons->padding.top + cons->border.top;
    float off_r = cons->padding.right + cons->border.right;
    float off_b = cons->padding.bottom + cons->border.bottom;

    float inner_w = comp->w - (off_l + off_r);
    float inner_h = comp->h - (off_t + off_b);
    if (inner_w < 0.0f)
      inner_w = 0.0f;
    if (inner_h < 0.0f)
      inner_h = 0.0f;

    bool is_row_dir = muse__is_row(cons->flex_direction);
    bool is_rev = muse__is_reverse(cons->flex_direction);

    float inner_main = is_row_dir ? inner_w : inner_h;
    float inner_cross = is_row_dir ? inner_h : inner_w;

    float base_x = comp->x - cons->scroll.x;
    float base_y = comp->y - cons->scroll.y;
    float cross_start = is_row_dir ? (base_y + off_t) : (base_x + off_l);

    // Multi-line flex wrap layout
    if (cons->flex_wrap == MUSE_FLEX_WRAP ||
        cons->flex_wrap == MUSE_FLEX_WRAP_REVERSE) {
      muNode child_nodes[256];
      int child_cnt = 0;
      muse_foreach_child(c, ctx, node) {
        muConstraints *c_cons = muse_sparse_get(&ctx->constraints, c);
        if (c_cons &&
            c_cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE) {
          float abs_x = base_x;
          float abs_y = base_y;
          muComputed *c_comp = muse_sparse_get(&ctx->computed, c);
          if (c_comp) {
            if (!isnan(c_cons->positioning.absolute.left)) {
              abs_x = base_x + c_cons->positioning.absolute.left;
            } else if (!isnan(c_cons->positioning.absolute.right)) {
              abs_x = base_x + comp->w - c_comp->w -
                      c_cons->positioning.absolute.right;
            }
            if (!isnan(c_cons->positioning.absolute.top)) {
              abs_y = base_y + c_cons->positioning.absolute.top;
            } else if (!isnan(c_cons->positioning.absolute.bottom)) {
              abs_y = base_y + comp->h - c_comp->h -
                      c_cons->positioning.absolute.bottom;
            }
            c_comp->x = abs_x;
            c_comp->y = abs_y;
          }
          continue;
        }
        if (child_cnt < 256) {
          child_nodes[child_cnt++] = c;
        }
      }

      int line_start = 0;
      float cur_cross_start = cross_start;

      while (line_start < child_cnt) {
        float line_main_used = 0.0f;
        float line_cross_max = 0.0f;
        int line_child_count = 0;
        int line_end = line_start;

        while (line_end < child_cnt) {
          muNode c = child_nodes[line_end];
          muComputed *c_comp = muse_sparse_get(&ctx->computed, c);
          if (!c_comp) {
            line_end++;
            continue;
          }

          float child_main = is_row_dir ? c_comp->w : c_comp->h;
          float child_cross = is_row_dir ? c_comp->h : c_comp->w;
          float needed = child_main + (line_child_count > 0 ? cons->gap : 0.0f);

          if (line_child_count > 0 && line_main_used + needed > inner_main) {
            break;
          }

          line_main_used += needed;
          if (child_cross > line_cross_max)
            line_cross_max = child_cross;
          line_child_count++;
          line_end++;
        }

        float remaining_main = inner_main - line_main_used;
        if (remaining_main < 0.0f)
          remaining_main = 0.0f;
        float start_main_offset = 0.0f;
        float space_between = cons->gap;

        switch (cons->justify_content) {
        case MUSE_JUSTIFY_CENTER:
          start_main_offset = remaining_main / 2.0f;
          break;
        case MUSE_JUSTIFY_END:
          start_main_offset = remaining_main;
          break;
        case MUSE_JUSTIFY_SPACE_BETWEEN:
          if (line_child_count > 1)
            space_between = remaining_main / (line_child_count - 1) + cons->gap;
          break;
        case MUSE_JUSTIFY_SPACE_AROUND:
          if (line_child_count > 0) {
            space_between = remaining_main / line_child_count + cons->gap;
            start_main_offset = (space_between - cons->gap) / 2.0f;
          }
          break;
        case MUSE_JUSTIFY_SPACE_EVENLY:
          if (line_child_count > 0) {
            space_between = remaining_main / (line_child_count + 1) + cons->gap;
            start_main_offset = space_between - cons->gap;
          }
          break;
        default:
          break;
        }

        float cursor_main = 0.0f;
        if (is_row_dir) {
          cursor_main = is_rev ? (base_x + comp->w - off_r - start_main_offset)
                               : (base_x + off_l + start_main_offset);
        } else {
          cursor_main = is_rev ? (base_y + comp->h - off_b - start_main_offset)
                               : (base_y + off_t + start_main_offset);
        }

        for (int i = line_start; i < line_end; i++) {
          muNode c = child_nodes[i];
          muConstraints *c_cons = muse_sparse_get(&ctx->constraints, c);
          muComputed *c_comp = muse_sparse_get(&ctx->computed, c);
          if (!c_cons || !c_comp)
            continue;

          float child_cross = is_row_dir ? c_comp->h : c_comp->w;
          float cross_offset = 0.0f;
          muAlignItems effective_align = cons->align_items;
          if (c_cons->align_self != MUSE_ALIGN_SELF_AUTO) {
            switch (c_cons->align_self) {
            case MUSE_ALIGN_SELF_START:
              effective_align = MUSE_ALIGN_START;
              break;
            case MUSE_ALIGN_SELF_CENTER:
              effective_align = MUSE_ALIGN_CENTER;
              break;
            case MUSE_ALIGN_SELF_END:
              effective_align = MUSE_ALIGN_END;
              break;
            case MUSE_ALIGN_SELF_STRETCH:
              effective_align = MUSE_ALIGN_STRETCH;
              break;
            default:
              break;
            }
          }

          switch (effective_align) {
          case MUSE_ALIGN_CENTER:
            cross_offset = (line_cross_max - child_cross) / 2.0f;
            break;
          case MUSE_ALIGN_END:
            cross_offset = line_cross_max - child_cross;
            break;
          case MUSE_ALIGN_STRETCH:
            if (is_row_dir && c_cons->dimension.height.kind != MU_FIXED) {
              c_comp->h = line_cross_max;
            } else if (!is_row_dir &&
                       c_cons->dimension.width.kind != MU_FIXED) {
              c_comp->w = line_cross_max;
            }
            break;
          default:
            break;
          }

          if (is_row_dir) {
            float child_x = is_rev ? (cursor_main - c_comp->w) : cursor_main;
            c_comp->x = child_x;
            c_comp->y = cur_cross_start + cross_offset;
            if (is_rev) {
              cursor_main -= c_comp->w + space_between;
            } else {
              cursor_main += c_comp->w + space_between;
            }
          } else {
            float child_y = is_rev ? (cursor_main - c_comp->h) : cursor_main;
            c_comp->x = cur_cross_start + cross_offset;
            c_comp->y = child_y;
            if (is_rev) {
              cursor_main -= c_comp->h + space_between;
            } else {
              cursor_main += c_comp->h + space_between;
            }
          }
        }

        cur_cross_start += line_cross_max + cons->gap;
        line_start = line_end;
      }
      return;
    }

    float total_main = 0.0f;
    int child_count = 0;

    muse_foreach_child(child, ctx, node) {
      muConstraints *c_cons = muse_sparse_get(&ctx->constraints, child);
      if (c_cons == NULL ||
          c_cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE)
        continue;

      muComputed *c_comp = muse_sparse_get(&ctx->computed, child);
      total_main += is_row_dir ? c_comp->w : c_comp->h;
      child_count += 1;
    }

    if (child_count > 1) {
      total_main += cons->gap * (child_count - 1);
    }

    float remaining_main = inner_main - total_main;
    if (remaining_main < 0.0f)
      remaining_main = 0.0f;

    float start_main_offset = 0.0f;
    float space_between = cons->gap;

    switch (cons->justify_content) {
    case MUSE_JUSTIFY_CENTER:
      start_main_offset = remaining_main / 2.0f;
      break;
    case MUSE_JUSTIFY_END:
      start_main_offset = remaining_main;
      break;
    case MUSE_JUSTIFY_SPACE_BETWEEN:
      if (child_count > 1)
        space_between = remaining_main / (child_count - 1) + cons->gap;
      break;
    case MUSE_JUSTIFY_SPACE_AROUND:
      if (child_count > 0) {
        space_between = remaining_main / child_count + cons->gap;
        start_main_offset = (space_between - cons->gap) / 2.0f;
      }
      break;
    case MUSE_JUSTIFY_SPACE_EVENLY:
      if (child_count > 0) {
        space_between = remaining_main / (child_count + 1) + cons->gap;
        start_main_offset = space_between - cons->gap;
      }
      break;
    default:
      break;
    }

    float cursor_main = 0.0f;
    if (is_row_dir) {
      if (is_rev) {
        cursor_main = base_x + comp->w - off_r - start_main_offset;
      } else {
        cursor_main = base_x + off_l + start_main_offset;
      }
    } else {
      if (is_rev) {
        cursor_main = base_y + comp->h - off_b - start_main_offset;
      } else {
        cursor_main = base_y + off_t + start_main_offset;
      }
    }

#define MUSE_LAYOUT_CHILD(child_node)                                          \
  do {                                                                         \
    muConstraints *c_cons = muse_sparse_get(&ctx->constraints, (child_node));  \
    muComputed *c_comp = muse_sparse_get(&ctx->computed, (child_node));        \
    if (c_cons != NULL && c_comp != NULL) {                                    \
      if (c_cons->positioning.strategy == MUSE_POSITION_STRATEGY_ABSOLUTE) {   \
        float abs_x = base_x;                                                  \
        float abs_y = base_y;                                                  \
        if (!isnan(c_cons->positioning.absolute.left)) {                       \
          abs_x = base_x + c_cons->positioning.absolute.left;                  \
        } else if (!isnan(c_cons->positioning.absolute.right)) {               \
          abs_x = base_x + comp->w - c_comp->w -                               \
                  c_cons->positioning.absolute.right;                          \
        }                                                                      \
        if (!isnan(c_cons->positioning.absolute.top)) {                        \
          abs_y = base_y + c_cons->positioning.absolute.top;                   \
        } else if (!isnan(c_cons->positioning.absolute.bottom)) {              \
          abs_y = base_y + comp->h - c_comp->h -                               \
                  c_cons->positioning.absolute.bottom;                         \
        }                                                                      \
        c_comp->x = abs_x;                                                     \
        c_comp->y = abs_y;                                                     \
      } else {                                                                 \
        float child_cross = is_row_dir ? c_comp->h : c_comp->w;                \
        float cross_offset = 0.0f;                                             \
        muAlignItems effective_align = cons->align_items;                      \
        if (c_cons->align_self != MUSE_ALIGN_SELF_AUTO) {                      \
          switch (c_cons->align_self) {                                        \
          case MUSE_ALIGN_SELF_START:                                          \
            effective_align = MUSE_ALIGN_START;                                \
            break;                                                             \
          case MUSE_ALIGN_SELF_CENTER:                                         \
            effective_align = MUSE_ALIGN_CENTER;                               \
            break;                                                             \
          case MUSE_ALIGN_SELF_END:                                            \
            effective_align = MUSE_ALIGN_END;                                  \
            break;                                                             \
          case MUSE_ALIGN_SELF_STRETCH:                                        \
            effective_align = MUSE_ALIGN_STRETCH;                              \
            break;                                                             \
          default:                                                             \
            break;                                                             \
          }                                                                    \
        }                                                                      \
        switch (effective_align) {                                             \
        case MUSE_ALIGN_CENTER:                                                \
          cross_offset = (inner_cross - child_cross) / 2.0f;                   \
          break;                                                               \
        case MUSE_ALIGN_END:                                                   \
          cross_offset = inner_cross - child_cross;                            \
          break;                                                               \
        case MUSE_ALIGN_STRETCH:                                               \
          if (is_row_dir && c_cons->dimension.height.kind != MU_FIXED) {       \
            c_comp->h = inner_cross;                                           \
          } else if (!is_row_dir &&                                            \
                     c_cons->dimension.width.kind != MU_FIXED) {               \
            c_comp->w = inner_cross;                                           \
          }                                                                    \
          break;                                                               \
        default:                                                               \
          break;                                                               \
        }                                                                      \
        if (is_row_dir) {                                                      \
          float child_x = is_rev ? (cursor_main - c_comp->w) : cursor_main;    \
          c_comp->x = child_x;                                                 \
          c_comp->y = cross_start + cross_offset;                              \
          if (is_rev) {                                                        \
            cursor_main -= c_comp->w + space_between;                          \
          } else {                                                             \
            cursor_main += c_comp->w + space_between;                          \
          }                                                                    \
        } else {                                                               \
          float child_y = is_rev ? (cursor_main - c_comp->h) : cursor_main;    \
          c_comp->x = cross_start + cross_offset;                              \
          c_comp->y = child_y;                                                 \
          if (is_rev) {                                                        \
            cursor_main -= c_comp->h + space_between;                          \
          } else {                                                             \
            cursor_main += c_comp->h + space_between;                          \
          }                                                                    \
        }                                                                      \
      }                                                                        \
    }                                                                          \
  } while (0)

    if (is_rev) {
      muse_foreach_child_reverse(child, ctx, node) { MUSE_LAYOUT_CHILD(child); }
    } else {
      muse_foreach_child(child, ctx, node) { MUSE_LAYOUT_CHILD(child); }
    }

#undef MUSE_LAYOUT_CHILD
  }
}

static void muse__m_compute_content_bounds_node(muContext *ctx, muNode node) {
  if (!muse_muid_is_valid(node))
    return;

  muComputed *comp = muse_sparse_get(&ctx->computed, node);
  if (!comp)
    return;

  float max_w = comp->w;
  float max_h = comp->h;

  muConstraints *cons = muse_sparse_get(&ctx->constraints, node);
  float scroll_y = cons ? cons->scroll.y : 0.0f;
  float scroll_x = cons ? cons->scroll.x : 0.0f;

  if (muse_sparse_has(&ctx->texts, node) && ctx->text_sizing_func != NULL) {
    float off_w = cons ? (cons->padding.left + cons->border.left +
                          cons->padding.right + cons->border.right)
                       : 0.0f;
    float off_h = cons ? (cons->padding.top + cons->border.top +
                          cons->padding.bottom + cons->border.bottom)
                       : 0.0f;
    float avail_w = (comp->w > off_w) ? (comp->w - off_w) : INFINITY;
    float avail_h = (comp->h > off_h) ? (comp->h - off_h) : INFINITY;
    muTextComputedOutput text_size =
        muse__m_get_text_size(ctx, node, avail_w, avail_h);
    max_w = text_size.computed_width + off_w;
    max_h = text_size.computed_height + off_h;
  } else {
    muse_foreach_child(child, ctx, node) {
      muComputed *c_comp = muse_sparse_get(&ctx->computed, child);
      if (c_comp) {
        float child_bottom = (c_comp->y - comp->y) + c_comp->h + scroll_y;
        float child_right = (c_comp->x - comp->x) + c_comp->w + scroll_x;
        if (child_bottom > max_h)
          max_h = child_bottom;
        if (child_right > max_w)
          max_w = child_right;
      }
    }
  }

  if (cons) {
    max_h += cons->padding.bottom;
    max_w += cons->padding.right;
  }

  comp->content_w = (max_w > comp->w) ? max_w : comp->w;
  comp->content_h = (max_h > comp->h) ? max_h : comp->h;

  if (cons != NULL) {
    if (cons->scroll.y < 0.0f) {
      float inner_h = comp->h - cons->padding.top - cons->padding.bottom;
      float max_y = (comp->content_h - inner_h > 0.0f)
                        ? (comp->content_h - inner_h)
                        : 0.0f;
      float pct = (-cons->scroll.y - 0.0001f);
      if (pct < 0.0f)
        pct = 0.0f;
      if (pct > 1.0f)
        pct = 1.0f;
      cons->scroll.y = pct * max_y;
    }
    if (cons->scroll.x < 0.0f) {
      float inner_w = comp->w - cons->padding.left - cons->padding.right;
      float max_x = (comp->content_w - inner_w > 0.0f)
                        ? (comp->content_w - inner_w)
                        : 0.0f;
      float pct = (-cons->scroll.x - 0.0001f);
      if (pct < 0.0f)
        pct = 0.0f;
      if (pct > 1.0f)
        pct = 1.0f;
      cons->scroll.x = pct * max_x;
    }
  }
}

MUSEDEF void muse_compute_layout(muContext *ctx, float viewport_width,
                                 float viewport_height) {
  if (!ctx->rooted)
    return;
  if (ctx->dirties.dense.count == 0)
    return;

  // Ensure topological layout order array is up-to-date
  muse__m_ensure_layout_order(ctx);
  if (ctx->layout_order.count == 0)
    return;

  // PASS 1: Dirty propagation
  for (size_t i = 0; i < ctx->dirties.dense.count; i++) {
    muNode dirty_node = ctx->dirties.dense.items[i];

    muConstraints *constraints = muse_sparse_get(&ctx->constraints, dirty_node);
    muHierarchy *hrc = muse_sparse_get(&ctx->hierarchies, dirty_node);

    if (constraints == NULL || hrc == NULL)
      continue;

    // A) Pull : If my size/position changed, notify parent
    muNode curr_parent = hrc->parent;
    while (muse_muid_is_valid(curr_parent)) {
      if (muse_sparse_has(&ctx->dirties, curr_parent))
        break;

      muConstraints *p_cons = muse_sparse_get(&ctx->constraints, curr_parent);
      if (p_cons != NULL) {
        muse_node_set_dirty(ctx, curr_parent);

        if (p_cons->dimension.width.kind == MU_FIT ||
            p_cons->dimension.height.kind == MU_FIT) {
          // FIT parent walks up
          muHierarchy *p_hrc = muse_sparse_get(&ctx->hierarchies, curr_parent);
          curr_parent = (p_hrc != NULL) ? p_hrc->parent : MUSE_UNDEFINED_MUID;
        } else {
          break;
        }
      } else {
        break;
      }
    }

    // B) Push: If parent changed, notify immediate children
    muse_foreach_child(child, ctx, dirty_node) {
      if (muse_sparse_has(&ctx->dirties, child))
        continue;

      muse_node_set_dirty(ctx, child);
    }
  }

  muComputed viewport_bounds = {
      .x = 0.0f, .y = 0.0f, .w = viewport_width, .h = viewport_height};

  // PASS 2: Available Space (Top-Down Flat Pre-Order Loop with Subtree
  // Skipping)
  for (size_t i = 0; i < ctx->layout_order.count; i++) {
    muNode node = ctx->layout_order.items[i];
    if (!muse_sparse_has(&ctx->dirties, node))
      continue;

    muHierarchy *hrc = muse_sparse_get(&ctx->hierarchies, node);
    muComputed parent_bounds = viewport_bounds;

    if (hrc && muse_muid_is_valid(hrc->parent)) {
      muComputed *p_comp = muse_sparse_get(&ctx->computed, hrc->parent);
      muConstraints *p_cons = muse_sparse_get(&ctx->constraints, hrc->parent);
      if (p_comp) {
        float off_l =
            p_cons ? (p_cons->padding.left + p_cons->border.left) : 0.0f;
        float off_t =
            p_cons ? (p_cons->padding.top + p_cons->border.top) : 0.0f;
        float off_r =
            p_cons ? (p_cons->padding.right + p_cons->border.right) : 0.0f;
        float off_b =
            p_cons ? (p_cons->padding.bottom + p_cons->border.bottom) : 0.0f;

        parent_bounds.x = p_comp->x + off_l;
        parent_bounds.y = p_comp->y + off_t;
        parent_bounds.w = p_comp->w - (off_l + off_r);
        parent_bounds.h = p_comp->h - (off_t + off_b);
        if (parent_bounds.w < 0.0f)
          parent_bounds.w = 0.0f;
        if (parent_bounds.h < 0.0f)
          parent_bounds.h = 0.0f;
      }
    }

    muse__m_compute_top_down_node(ctx, node, parent_bounds);
  }

  // PASS 3: Intrinsic Sizing (Bottom-Up Flat Reverse Loop with Subtree
  // Skipping)
  for (size_t i = ctx->layout_order.count; i-- > 0;) {
    muNode node = ctx->layout_order.items[i];
    if (!muse_sparse_has(&ctx->dirties, node))
      continue;

    muse__m_compute_bottom_up_node(ctx, node);
  }

  // PASS 4: Flex Distribution (Top-Down Flat Pre-Order Loop with Subtree
  // Skipping)
  for (size_t i = 0; i < ctx->layout_order.count; i++) {
    muNode node = ctx->layout_order.items[i];
    if (!muse_sparse_has(&ctx->dirties, node))
      continue;

    muse__m_compute_flex_distribution_node(ctx, node);
  }

  // PASS 5: Positional Alignment (Top-Down Flat Pre-Order Loop with Subtree
  // Skipping)
  if (ctx->rooted && muse_muid_is_valid(ctx->root)) {
    muComputed *r_comp = muse_sparse_get(&ctx->computed, ctx->root);
    if (r_comp) {
      r_comp->x = 0.0f;
      r_comp->y = 0.0f;
    }
  }
  for (size_t i = 0; i < ctx->layout_order.count; i++) {
    muNode node = ctx->layout_order.items[i];
    if (!muse_sparse_has(&ctx->dirties, node))
      continue;

    muse__m_compute_positional_alignment_node(ctx, node);
  }

  // PASS 5.5: Content Bounds Calculation (Bottom-Up Flat Reverse Loop with
  // Subtree Skipping)
  for (size_t i = ctx->layout_order.count; i-- > 0;) {
    muNode node = ctx->layout_order.items[i];
    if (!muse_sparse_has(&ctx->dirties, node))
      continue;

    muse__m_compute_content_bounds_node(ctx, node);
  }

  // PASS 6: Clear Dirties
  for (size_t i = 0; i < ctx->dirties.dense.count; i++) {
    muNode dirty_node = ctx->dirties.dense.items[i];
    ctx->dirties.sparse.items[dirty_node.numeral] = MUSE_SPARSE_NULL;
  }
  ctx->dirties.dense.count = 0;
  ctx->dirties.components.count = 0;
  ctx->render_list_dirty = true;
}

typedef enum {
  MU_SORT_ITEM_NODE = 0,
  MU_SORT_ITEM_SCROLLBAR_V = 1,
  MU_SORT_ITEM_SCROLLBAR_H = 2,
} muSortItemKind;

typedef struct {
  muNode node;
  muRect clip;
  muRect content_clip;
  size_t sequence;
  int32_t z_index;
  bool has_clip;
  bool has_content_clip;
  muSortItemKind kind;
} muse__m_SortItem;

typedef MUSE_DA(muse__m_SortItem) muse__m_SortList;

static void muse__m_flatten_recursive(muContext *ctx, muNode node,
                                      muse__m_SortList *list, size_t *seq,
                                      muRect current_clip, bool has_clip,
                                      int32_t inherited_z) {
  if (!muse_muid_is_valid(node))
    return;

  muConstraints *cons = muse_sparse_get(&ctx->constraints, node);
  int32_t z =
      (cons != NULL && cons->z_index != 0) ? cons->z_index : inherited_z;
  muComputed *comp = muse_sparse_get(&ctx->computed, node);

  muRect new_clip = current_clip;
  bool new_has_clip = has_clip;

  if (cons != NULL &&
      (cons->overflow == MU_OVERFLOW_HIDDEN ||
       cons->overflow == MU_OVERFLOW_SCROLL) &&
      comp != NULL) {
    float cx = comp->x + cons->border.left;
    float cy = comp->y + cons->border.top;
    float cw = comp->w - cons->border.left - cons->border.right;
    float ch = comp->h - cons->border.top - cons->border.bottom;

    if (has_clip) {
      float x1 = (new_clip.x > cx) ? new_clip.x : cx;
      float y1 = (new_clip.y > cy) ? new_clip.y : cy;
      float x2 = (new_clip.x + new_clip.w < cx + cw) ? new_clip.x + new_clip.w
                                                     : cx + cw;
      float y2 = (new_clip.y + new_clip.h < cy + ch) ? new_clip.y + new_clip.h
                                                     : cy + ch;

      new_clip.x = x1;
      new_clip.y = y1;
      new_clip.w = (x2 > x1) ? (x2 - x1) : 0.0f;
      new_clip.h = (y2 > y1) ? (y2 - y1) : 0.0f;
    } else {
      new_clip = (muRect){cx, cy, cw, ch};
      new_has_clip = true;
    }
  }

  bool visible = true;
  if (has_clip && comp != NULL) {
    if (comp->x >= current_clip.x + current_clip.w ||
        comp->x + comp->w <= current_clip.x ||
        comp->y >= current_clip.y + current_clip.h ||
        comp->y + comp->h <= current_clip.y) {
      visible = false;
    }
  }

  if (visible) {
    muse__m_SortItem item = {.node = node,
                             .z_index = z,
                             .sequence = (*seq)++,
                             .clip = current_clip,
                             .content_clip = new_clip,
                             .has_clip = has_clip,
                             .has_content_clip = new_has_clip,
                             .kind = MU_SORT_ITEM_NODE};
    muse_da_append(list, item);
  }

  if (new_has_clip && (new_clip.w <= 0.0f || new_clip.h <= 0.0f)) {
    return;
  }

  muse_foreach_child(child, ctx, node) {
    muse__m_flatten_recursive(ctx, child, list, seq, new_clip, new_has_clip, z);
  }

  // Shadow subnodes: Emit shadow scrollbar items after all children are
  // flattened
  if (cons != NULL && comp != NULL &&
      (cons->overflow == MU_OVERFLOW_SCROLL ||
       cons->overflow == MU_OVERFLOW_AUTO)) {
    float inner_h = (comp->h - cons->padding.top - cons->padding.bottom);
    float inner_w = (comp->w - cons->padding.left - cons->padding.right);

    if (comp->content_h > comp->h + 0.5f && inner_h > 0.0f) {
      muse__m_SortItem v_thumb = {.node = node,
                                  .z_index = z,
                                  .sequence = (*seq)++,
                                  .clip = new_clip,
                                  .has_clip = new_has_clip,
                                  .kind = MU_SORT_ITEM_SCROLLBAR_V};
      muse_da_append(list, v_thumb);
    }

    if (comp->content_w > comp->w + 0.5f && inner_w > 0.0f) {
      muse__m_SortItem h_thumb = {.node = node,
                                  .z_index = z,
                                  .sequence = (*seq)++,
                                  .clip = new_clip,
                                  .has_clip = new_has_clip,
                                  .kind = MU_SORT_ITEM_SCROLLBAR_H};
      muse_da_append(list, h_thumb);
    }
  }
}

static int muse__m_render_cmp(const void *a, const void *b) {
  const muse__m_SortItem *ia = (const muse__m_SortItem *)a;
  const muse__m_SortItem *ib = (const muse__m_SortItem *)b;
  if (ia->z_index != ib->z_index) {
    return (ia->z_index > ib->z_index) - (ia->z_index < ib->z_index);
  }
  // Stable sort: keep tree order if z-indexes match
  return (ia->sequence > ib->sequence) - (ia->sequence < ib->sequence);
}

MUSEDEF void muse_build_render_list(muContext *ctx, muRect viewport) {
  if (!ctx->rooted) {
    ctx->render_list.count = 0;
    return;
  }

  if (!ctx->render_list_dirty)
    return;

  ctx->render_list.count = 0;

  muse__m_SortList temp_list = {0};
  size_t seq = 0;
  muse__m_flatten_recursive(ctx, ctx->root, &temp_list, &seq, viewport, true,
                            0);
  if (temp_list.count > 0) {
    qsort(temp_list.items, temp_list.count, sizeof(muse__m_SortItem),
          muse__m_render_cmp);
    muse_da_reserve(&ctx->render_list, temp_list.count);
  }

  for (size_t i = 0; i < temp_list.count; i++) {
    muNode node = temp_list.items[i].node;
    int32_t z = temp_list.items[i].z_index;
    muRect clip = temp_list.items[i].clip;
    bool has_clip = temp_list.items[i].has_clip;
    muSortItemKind kind = temp_list.items[i].kind;
    muComputed *comp = muse_sparse_get(&ctx->computed, node);

    if (comp != NULL) {
      if (kind == MU_SORT_ITEM_NODE) {
        muRenderCommand quad_cmd = {.kind = MU_CMD_DRAWQUAD,
                                    .node = node,
                                    .computed = *comp,
                                    .clip = clip,
                                    .has_clip = has_clip,
                                    .z_index = z};

        muse_da_append(&ctx->render_list, quad_cmd);
        muText *text = muse_sparse_get(&ctx->texts, node);
        if (text != NULL && text->data != NULL) {
          muConstraints *cons = muse_sparse_get(&ctx->constraints, node);
          muRect text_clip =
              (cons != NULL && (cons->overflow == MU_OVERFLOW_HIDDEN ||
                                cons->overflow == MU_OVERFLOW_SCROLL))
                  ? temp_list.items[i].content_clip
                  : clip;
          bool text_has_clip =
              (cons != NULL && (cons->overflow == MU_OVERFLOW_HIDDEN ||
                                cons->overflow == MU_OVERFLOW_SCROLL))
                  ? temp_list.items[i].has_content_clip
                  : has_clip;

          muRenderCommand text_cmd = {.kind = MU_CMD_TEXT,
                                      .info = {.text = text},
                                      .node = node,
                                      .computed = *comp,
                                      .clip = text_clip,
                                      .has_clip = text_has_clip,
                                      .z_index = z};
          muse_da_append(&ctx->render_list, text_cmd);
        }
      } else if (kind == MU_SORT_ITEM_SCROLLBAR_V) {
        muRenderCommand sb_cmd = {.kind = MU_CMD_SCROLLBAR_V,
                                  .node = node,
                                  .computed = *comp,
                                  .clip = clip,
                                  .has_clip = has_clip,
                                  .z_index = z};
        muse_da_append(&ctx->render_list, sb_cmd);
      } else if (kind == MU_SORT_ITEM_SCROLLBAR_H) {
        muRenderCommand sb_cmd = {.kind = MU_CMD_SCROLLBAR_H,
                                  .node = node,
                                  .computed = *comp,
                                  .clip = clip,
                                  .has_clip = has_clip,
                                  .z_index = z};
        muse_da_append(&ctx->render_list, sb_cmd);
      }
    }
  }

  muse_da_free(&temp_list);
  ctx->render_list_dirty = false;
}

MUSEDEF muNodeList muse_node_pick(muContext *ctx, float x, float y) {
  ctx->pick_list.count = 0;

  if (!ctx->rooted || ctx->render_list.count == 0)
    return ctx->pick_list;

  muNode last_checked = MUSE_UNDEFINED_MUID;

  for (size_t i = ctx->render_list.count; i > 0; i--) {
    muRenderCommand cmd = ctx->render_list.items[i - 1];
    muNode node = cmd.node;

    if (muse_muid_eq(node, last_checked))
      continue;

    bool in_bounds =
        (x >= cmd.computed.x && x <= cmd.computed.x + cmd.computed.w &&
         y >= cmd.computed.y && y <= cmd.computed.y + cmd.computed.h);

    if (!in_bounds)
      continue;

    if (cmd.has_clip) {
      if (x < cmd.clip.x || x > cmd.clip.x + cmd.clip.w || y < cmd.clip.y ||
          y > cmd.clip.y + cmd.clip.h) {
        continue;
      }
    }

    last_checked = node;
    muse_da_append(&ctx->pick_list, node);
  }

  return ctx->pick_list;
}

#endif // MUSE_IMPLEMENTATION
