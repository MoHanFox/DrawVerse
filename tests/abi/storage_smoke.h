#ifndef DRAWVERSE_STORAGE_SMOKE_H
#define DRAWVERSE_STORAGE_SMOKE_H
#include "paint_api.h"
#include <stddef.h>

/* Runs for C11/C++20 and static/shared consumers. Previous ABI paths still run. */
static int storage_smoke(void) {
    PaintStorageOptions options = {0};
    PaintStorageInfo info = {0};
    PaintCore *configured = NULL;
    options.struct_size = sizeof(options);
    options.resident_bytes = 65536;
    options.scratch_bytes = 1024 * 1024;
    info.struct_size = sizeof(info);
    if (paint_storage_inspect(&options, &info) != PAINT_OK || !info.available_bytes) return 1;
    if (paint_core_create_with_storage(&options, &configured) != PAINT_OK || !configured) return 1;
    if (paint_core_destroy(&configured) != PAINT_OK || configured) return 1;
    options.resident_bytes++;
    if (paint_core_create_with_storage(&options, &configured) != PAINT_INVALID_ARGUMENT || configured) return 1;
    return 0;
}
#endif
