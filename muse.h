#ifndef MUSE_H_
#define MUSE_H_

#include <assert.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#ifndef MUSEDEF
#define MUSEDEF
#endif /* MUSEDEF */

//////////////////// da (Tsoding way)

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
          realloc((da)->items, (da)->capacity * sizeof(*(da)->items));         \
      assert((da)->items != NULL && "Buy more RAM lol");                       \
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

#define muse_da_free(da) free((da)->items)

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
  float x, y;
} muVector2;

typedef struct {
  size_t numeral, generation;
} muId;

typedef muId muNode;

typedef struct {
  muId parent;
  muId first_child;
  muId last_child;
  muId next_sibling;
  muId prev_sibling;
} muHierarchy;

typedef struct {
  muSize size[2]; // Index 0 = Width, Index 1 = Height
} muConstraints;

typedef struct {
  float x, y, w, h;
} muComputed;

typedef struct {
  MUSE_SPARSE_SET(muHierarchy) hierarchies;
  MUSE_SPARSE_SET(muConstraints) constraints;
  MUSE_SPARSE_SET(muComputed) computed;

  size_t next_entity_numeral;

  struct {
    muId *items;
    size_t count;
    size_t capacity;
  } available_ids;

  muNode root;
  bool rooted; // Just to make it nicer to use
} muContext;

MUSEDEF void muse_free_context(muContext *ctx);

MUSEDEF void muse_root_attach(muContext *ctx, muNode node);
MUSEDEF void muse_root_drop(muContext *ctx);

MUSEDEF void muse_node_parented(muContext *ctx, muNode parent, muNode child);
MUSEDEF void muse_node_after(muContext *ctx, muNode sibling, muNode node);
MUSEDEF void muse_node_before(muContext *ctx, muNode sibling, muNode node);
MUSEDEF void muse_node_unparented(muContext *ctx, muNode node);

MUSEDEF muNode muse_node_create(muContext *ctx);
MUSEDEF void muse_node_destroy(muContext *ctx, muNode node);

#endif // MUSE_H_

#define MUSE_IMPLEMENTATION

#ifdef MUSE_IMPLEMENTATION

MUSEDEF void muse_free_context(muContext *ctx) {
  muse_da_free(&ctx->available_ids);

  muse_sparse_free(&ctx->hierarchies);
  muse_sparse_free(&ctx->constraints);
  muse_sparse_free(&ctx->computed);
}

MUSEDEF void muse_root_attach(muContext *ctx, muNode node) {
  ctx->root = node;
  ctx->rooted = true;
}

MUSEDEF void muse_root_drop(muContext *ctx) { ctx->rooted = false; }

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
  muse_sparse_remove(&ctx->computed, node);
  muse_sparse_remove(&ctx->hierarchies, node);
  muse_sparse_remove(&ctx->constraints, node);

  muse_da_append(&ctx->available_ids, node);
}

#endif // MUSE_IMPLEMENTATION
