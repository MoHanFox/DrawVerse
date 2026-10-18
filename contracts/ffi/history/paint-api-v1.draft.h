/* DESIGN DRAFT ONLY. Stage 3 will generate the real header with cbindgen. */
#ifndef DRAWVERSE_PAINT_API_V1_DRAFT_H
#define DRAWVERSE_PAINT_API_V1_DRAFT_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct PaintCore PaintCore;
typedef struct PaintDocument PaintDocument;
typedef struct PaintTask PaintTask;
typedef struct PaintSubscription PaintSubscription;
typedef int32_t PaintStatus;

typedef struct PaintVersion {
    uint32_t struct_size;
    uint32_t major, minor, patch;
} PaintVersion;

typedef struct PaintDocumentDesc {
    uint32_t struct_size;
    uint32_t width, height;
    uint32_t working_space; /* 1 = linear sRGB */
    uint32_t pixel_format;  /* 1 = premultiplied RGBA32F */
    uint32_t reserved[3];  /* zero */
} PaintDocumentDesc;

typedef struct PaintPoint {
    uint32_t struct_size, capabilities;
    double x, y; /* document pixel coordinates, independent of device DPI */
    float pressure, tilt_x, tilt_y, rotation, tangential_pressure;
    uint32_t buttons, tool; /* tool: 1=pen, 2=eraser, 3=mouse */
    uint64_t timestamp_ns;
} PaintPoint;

typedef struct PaintStrokeDesc {
    uint32_t struct_size, mode; /* 1=paint, 2=erase */
    uint64_t layer_id;
    float radius, opacity, spacing;
    float linear_rgba[4]; /* straight linear color; kernel premultiplies */
    uint32_t reserved[3];
} PaintStrokeDesc;

typedef struct PaintTile {
    uint32_t struct_size, format;
    uint8_t *data; /* caller-owned writable buffer */
    uint64_t capacity, stride, required;
    uint32_t width, height;
    uint64_t revision;
} PaintTile;

typedef struct PaintEvent {
    uint32_t struct_size, kind;
    uint64_t document_id, revision, task_id;
    int32_t x, y;
    uint32_t width, height;
    PaintStatus status;
    uint32_t reserved;
} PaintEvent;
typedef void (*PaintEventCallback)(const PaintEvent *event, void *user_data);

PaintStatus paint_core_version(PaintVersion *out_version);
PaintStatus paint_core_create(PaintCore **out_core);
PaintStatus paint_core_destroy(PaintCore **core);
PaintStatus paint_document_destroy(PaintCore *core, PaintDocument **doc);
PaintStatus paint_core_new_document(PaintCore *core, const PaintDocumentDesc *desc,
                                    PaintDocument **out_doc);
/* Heavy synchronous functions are worker-thread-only. Async forms are UI entry points. */
PaintStatus paint_core_load_document(PaintCore *core, const uint8_t *path,
                                     uint64_t path_len, PaintDocument **out_doc);
PaintStatus paint_core_load_document_async(PaintCore *core, const uint8_t *path,
                                           uint64_t path_len, PaintTask **out_task);
PaintStatus paint_core_save_document_async(PaintCore *core, PaintDocument *doc,
                                           const uint8_t *path, uint64_t path_len,
                                           uint32_t format, PaintTask **out_task);
PaintStatus paint_core_begin_stroke(PaintCore *core, PaintDocument *doc,
                                    const PaintStrokeDesc *desc, const PaintPoint *first);
PaintStatus paint_core_stroke_to(PaintCore *core, PaintDocument *doc, const PaintPoint *point);
PaintStatus paint_core_end_stroke(PaintCore *core, PaintDocument *doc);
PaintStatus paint_core_cancel_stroke(PaintCore *core, PaintDocument *doc);
PaintStatus paint_core_undo(PaintCore *core, PaintDocument *doc);
PaintStatus paint_core_redo(PaintCore *core, PaintDocument *doc);
PaintStatus paint_core_render_tile(PaintCore *core, PaintDocument *doc,
                                   uint32_t x, uint32_t y, uint32_t w, uint32_t h,
                                   PaintTile *out_tile);
PaintStatus paint_core_render_tile_async(PaintCore *core, PaintDocument *doc,
                                         uint32_t x, uint32_t y, uint32_t w, uint32_t h,
                                         PaintTask **out_task);
PaintStatus paint_task_cancel(PaintCore *core, PaintTask *task);
PaintStatus paint_task_destroy(PaintCore *core, PaintTask **task);
PaintStatus paint_core_subscribe(PaintCore *core, PaintEventCallback callback,
                                 void *user_data, PaintSubscription **out_subscription);
PaintStatus paint_core_unsubscribe(PaintCore *core, PaintSubscription **subscription);
PaintStatus paint_error_message(uint8_t *buffer, uint64_t capacity, uint64_t *required);
/* Layer enumeration, task status/result and capability DTOs are specified in stage 3. */
#ifdef __cplusplus
}
#endif
#endif
