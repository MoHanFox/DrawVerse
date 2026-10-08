#include "paint_api.h"
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include "async_smoke.h"
#include "storage_smoke.h"

#define CHECK(condition) do { if (!(condition)) { fprintf(stderr, "C ABI check failed at line %d: %s\n", __LINE__, #condition); return 1; } } while (0)
_Static_assert(sizeof(void *) == 8, "ABI v1 requires a 64-bit target");
_Static_assert(sizeof(PaintVersion) == 16, "version layout drift");
_Static_assert(sizeof(PaintCapabilities) == 32, "capability layout drift");
_Static_assert(sizeof(PaintDocumentDesc) == 32, "desc layout drift");
_Static_assert(sizeof(PaintDocumentInfo) == 64, "document layout drift");
_Static_assert(sizeof(PaintLayerInfo) == 40, "layer layout drift");
_Static_assert(sizeof(PaintPoint) == 64, "point layout drift");
_Static_assert(offsetof(PaintPoint, timestamp_ns) == 56, "point offset drift");
_Static_assert(sizeof(PaintStrokeDesc) == 56, "stroke layout drift");
_Static_assert(offsetof(PaintStrokeDesc, linear_rgba) == 28, "color offset drift");
_Static_assert(sizeof(PaintTile) == 56, "tile layout drift");
_Static_assert(offsetof(PaintTile, revision) == 48, "revision offset drift");
_Static_assert(sizeof(PaintEvent) == 56, "event layout drift");
_Static_assert(sizeof(PaintStorageOptions) == 48, "storage options layout drift");
_Static_assert(offsetof(PaintStorageOptions, path) == 8, "storage path layout drift");
_Static_assert(sizeof(PaintStorageInfo) == 32, "storage info layout drift");

typedef struct CallbackContext {
    PaintCore *core;
    PaintDocument *doc;
    unsigned calls;
    unsigned state_calls;
    int failed;
} CallbackContext;

static void on_event(const PaintEvent *event, void *user) {
    CallbackContext *context = (CallbackContext *)user;
    PaintDocumentInfo info = {0};
    info.struct_size = sizeof(info);
    if (paint_document_info(context->core, context->doc, &info) != PAINT_OK ||
        event->document_id != info.document_id || event->struct_size != sizeof(*event)) {
        context->failed = 1;
    }
    context->calls++;
    if (event->kind == PAINT_EVENT_DOCUMENT_STATE_CHANGED) context->state_calls++;
}

int main(void) {
    CHECK(storage_smoke() == 0);
    PaintCore *core = NULL;
    PaintDocument *doc = NULL;
    PaintSubscription *subscription = NULL;
    PaintVersion version = {0};
    PaintCapabilities caps = {0};
    PaintDocumentDesc desc = {0};
    PaintDocumentInfo info = {0};
    PaintPoint first = {0}, last = {0};
    PaintStrokeDesc stroke = {0};
    PaintTile tile = {0};
    CallbackContext context = {0};
    float pixels[4] = {0};
    uint64_t needed = 0, layer = 0;
    uint8_t message[256] = {0};
    const uint8_t name[] = {0xe7, 0xbb, 0x98, 0xe7, 0x94, 0xbb}; /* UTF-8: drawing */
    uint8_t copied_name[sizeof(name)] = {0};

    version.struct_size = sizeof(version);
    CHECK(paint_core_version(&version) == PAINT_OK && version.major == 1);
    CHECK(paint_core_create(&core) == PAINT_OK && core != NULL);
    caps.struct_size = sizeof(caps);
    CHECK(paint_core_capabilities(core, &caps) == PAINT_OK);
    CHECK((caps.features & (PAINT_FEATURE_DOCUMENT | PAINT_FEATURE_HISTORY | PAINT_FEATURE_LINEAR_TILE_READ | PAINT_FEATURE_EVENTS)) == (PAINT_FEATURE_DOCUMENT | PAINT_FEATURE_HISTORY | PAINT_FEATURE_LINEAR_TILE_READ | PAINT_FEATURE_EVENTS));
    desc.struct_size = sizeof(desc); desc.width = 128; desc.height = 64;
    desc.working_space = PAINT_WORKING_LINEAR_SRGB; desc.pixel_format = PAINT_STORAGE_RGBA32F_PREMULTIPLIED;
    CHECK(paint_core_new_document(core, &desc, &doc) == PAINT_OK);
    CHECK(paint_core_destroy(&core) == PAINT_BUSY && core != NULL);
    CHECK(paint_error_message(NULL, 0, &needed) == PAINT_BUFFER_TOO_SMALL && needed > 0);
    CHECK(paint_error_message(message, sizeof(message), &needed) == PAINT_OK);
    context.core = core; context.doc = doc;
    CHECK(paint_core_subscribe(core, on_event, &context, &subscription) == PAINT_OK);

    CHECK(paint_layer_add(core, doc, name, sizeof(name), &layer) == PAINT_OK && layer > 1);
    CHECK(paint_layer_name(core, doc, layer, copied_name, sizeof(copied_name), &needed) == PAINT_OK);
    CHECK(needed == sizeof(name) && memcmp(name, copied_name, sizeof(name)) == 0);
    stroke.struct_size = sizeof(stroke); stroke.mode = PAINT_MODE_PAINT; stroke.layer_id = layer;
    stroke.radius = 4; stroke.opacity = 1; stroke.spacing = 0.25f;
    stroke.linear_rgba[0] = 1; stroke.linear_rgba[3] = 1;
    first.struct_size = sizeof(first); first.tool = PAINT_TOOL_PEN;
    first.capabilities = PAINT_INPUT_PRESSURE; first.x = 16.5; first.y = 16.5; first.pressure = 1;
    last = first; last.x = 100.5; last.timestamp_ns = 100;
    CHECK(paint_core_begin_stroke(core, doc, &stroke, &first) == PAINT_OK);
    CHECK(paint_core_stroke_to(core, doc, &last) == PAINT_OK);
    CHECK(paint_core_end_stroke(core, doc) == PAINT_OK);
    CHECK(context.calls >= 4 && context.state_calls > 0 && context.failed == 0);

    tile.struct_size = sizeof(tile); tile.format = PAINT_TILE_RGBA32F_LINEAR_PREMULTIPLIED; tile.stride = sizeof(pixels);
    CHECK(paint_core_read_tile(core, doc, 64, 16, 1, 1, &tile) == PAINT_BUFFER_TOO_SMALL);
    CHECK(tile.required == sizeof(pixels) && tile.width == 0);
    tile.data = (uint8_t *)pixels; tile.capacity = sizeof(pixels);
    CHECK(paint_core_read_tile(core, doc, 64, 16, 1, 1, &tile) == PAINT_OK);
    CHECK(pixels[0] == 1 && pixels[1] == 0 && pixels[2] == 0 && pixels[3] == 1);
    CHECK(paint_core_undo(core, doc) == PAINT_OK);
    CHECK(paint_core_read_tile(core, doc, 64, 16, 1, 1, &tile) == PAINT_OK && pixels[3] == 0);
    CHECK(paint_core_redo(core, doc) == PAINT_OK);
    CHECK(paint_core_read_tile(core, doc, 64, 16, 1, 1, &tile) == PAINT_OK && pixels[3] == 1);
    CHECK(paint_layer_set_properties(core, doc, layer, 1, 0.5f) == PAINT_OK);
    CHECK(paint_core_read_tile(core, doc, 64, 16, 1, 1, &tile) == PAINT_OK && pixels[3] == 0.5f);
    info.struct_size = sizeof(info);
    CHECK(paint_document_info(core, doc, &info) == PAINT_OK && info.layer_count == 2 && info.allocated_tiles == 2);
    CHECK(paint_core_unsubscribe(core, &subscription) == PAINT_OK && subscription == NULL);
    CHECK(paint_document_destroy(core, &doc) == PAINT_OK && doc == NULL);
    CHECK(async_smoke(core) == 0);
    CHECK(paint_core_destroy(&core) == PAINT_OK && core == NULL);
    CHECK(paint_core_destroy(&core) == PAINT_OK);
    puts("C11 ABI lifecycle, UTF-8, stroke, history, buffer, and callback checks passed.");
    return 0;
}
