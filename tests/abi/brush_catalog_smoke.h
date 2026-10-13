#ifndef DRAWVERSE_BRUSH_CATALOG_SMOKE_H
#define DRAWVERSE_BRUSH_CATALOG_SMOKE_H
#include <stddef.h>
#include <string.h>
#include <time.h>
#ifdef __cplusplus
static_assert(sizeof(PaintBrushSettings) == 64 && offsetof(PaintBrushSettings, tip) == 24);
static_assert(sizeof(PaintBrushPresetInfo) == 64 && sizeof(PaintBrushLibraryInfo) == 32);
static_assert(sizeof(PaintBrushToolDesc) == 56 && offsetof(PaintBrushToolDesc, linear_rgba) == 24);
#else
_Static_assert(sizeof(PaintBrushSettings) == 64 && offsetof(PaintBrushSettings, tip) == 24, "brush settings layout");
_Static_assert(sizeof(PaintBrushPresetInfo) == 64 && sizeof(PaintBrushLibraryInfo) == 32, "brush catalog layout");
_Static_assert(sizeof(PaintBrushToolDesc) == 56 && offsetof(PaintBrushToolDesc, linear_rgba) == 24, "brush tool layout");
#endif
static int brush_catalog_smoke(void) {
    PaintBrushLibrary *library = NULL;
    PaintBrushLibraryInfo info;
    PaintBrushPresetInfo preset;
    PaintBrushSettings settings;
    const char id[] = "round-pressure";
    const char copy[] = "12345678-1234-4234-8234-123456789abc";
    uint64_t old_revision, required = 0;
    uint8_t text[64];
    uint8_t state[4096];
    uint64_t preparation = 0, state_size = 0;
    PaintStatus status;
    clock_t started;
    memset(&info, 0, sizeof(info)); info.struct_size = sizeof(info);
    memset(&preset, 0, sizeof(preset)); preset.struct_size = sizeof(preset);
    memset(&settings, 0, sizeof(settings)); settings.struct_size = sizeof(settings);
    settings.tip.struct_size = sizeof(settings.tip);
    settings.diameter = 24; settings.opacity = 1; settings.spacing = .075f;
    settings.tip.hardness = 1; settings.tip.roundness = .2f; settings.tip.flow = 1;
    settings.tip.pressure_size = 1; settings.tip.pressure_flow = 1;
    if (paint_brush_library_create(0, &library) != PAINT_OK) return 1;
    if (paint_brush_library_share_resources(library, library) != PAINT_NOT_FOUND) return 16;
    if (paint_brush_library_bind_resources(library, NULL, 0) != PAINT_INVALID_ARGUMENT) return 17;
    if (paint_brush_library_tip_outline(library, (const uint8_t *)id, sizeof(id)-1, text, sizeof(text), &required) != PAINT_NOT_FOUND || required != 0) return 18;
    if (paint_brush_library_package_read(library, NULL, 0, text, sizeof(text), &required) != PAINT_INVALID_ARGUMENT || required != 0) return 19;
    if (paint_brush_library_info(library, &info) != PAINT_OK || info.preset_count != 4) return 2;
    old_revision = info.revision;
    if (paint_brush_library_text(library, info.revision, 0, 0, text, sizeof(text), &required) != PAINT_OK || required != sizeof(id)-1 || memcmp(text, id, required)) return 3;
    if (paint_brush_library_edit(library, (const uint8_t *)id, sizeof(id)-1, &settings) != PAINT_OK) return 4;
    if (paint_brush_library_preset_info(library, old_revision, 0, &preset) != PAINT_BUSY) return 5;
    if (paint_brush_library_info(library, &info) != PAINT_OK) return 6;
    if (paint_brush_library_preset_info(library, info.revision, 0, &preset) != PAINT_OK || preset.tip.roundness != .2f) return 7;
    if (paint_brush_library_remove(library, (const uint8_t *)id, sizeof(id)-1) != PAINT_INVALID_ARGUMENT) return 8;
    if (paint_brush_library_copy(library, (const uint8_t *)copy, sizeof(copy)-1, (const uint8_t *)"ABI copy", 8) != PAINT_OK) return 9;
    if (paint_brush_library_info(library, &info) != PAINT_OK || info.preset_count != 5 || info.selected_index != 4) return 10;
    if (paint_brush_library_current_state(library, state, sizeof(state), &state_size) != PAINT_OK) return 12;
    if (paint_brush_library_prepare_state(library, state, state_size, &preparation) != PAINT_OK || !preparation) return 13;
    started = clock();
    do {
        status = paint_brush_library_prepared_state(library, preparation, state, sizeof(state), &state_size);
        if (clock() - started > 5 * CLOCKS_PER_SEC) return 14;
    } while (status == PAINT_BUSY);
    if (status != PAINT_OK) return 15;
    if (paint_brush_library_destroy(&library) != PAINT_OK || library != NULL) return 11;
    return 0;
}
#endif
