#ifndef DRAWVERSE_BRUSH_CATALOG_SMOKE_H
#define DRAWVERSE_BRUSH_CATALOG_SMOKE_H
#include <stddef.h>
#include <string.h>
#include <time.h>
#ifdef __cplusplus
static_assert(sizeof(PaintWetLoadDesc)==32 && offsetof(PaintWetLoadDesc,linear_rgba)==16);
static_assert(sizeof(PaintWetToolInfo)==40 && offsetof(PaintWetToolInfo,center_linear_premul_rgba)==24);
static_assert(sizeof(PaintBrushSettings) == 64 && offsetof(PaintBrushSettings, tip) == 24);
static_assert(sizeof(PaintBrushPresetInfo) == 64 && sizeof(PaintBrushLibraryInfo) == 32);
static_assert(sizeof(PaintBrushToolDesc) == 56 && offsetof(PaintBrushToolDesc, linear_rgba) == 24);
static_assert(sizeof(PaintTipPose)==24 && offsetof(PaintTipPose,radius)==8 && offsetof(PaintTipPose,roundness)==16);
static_assert(sizeof(PaintCompositeTipPose)==64 && offsetof(PaintCompositeTipPose,primary)==8 && offsetof(PaintCompositeTipPose,secondary)==32 && offsetof(PaintCompositeTipPose,reserved)==56);
static_assert(sizeof(PaintWetToolInfoDual)==80 && offsetof(PaintWetToolInfoDual,secondary_revision)==16 && offsetof(PaintWetToolInfoDual,primary_center_linear_premul_rgba)==32 && offsetof(PaintWetToolInfoDual,secondary_center_linear_premul_rgba)==48 && offsetof(PaintWetToolInfoDual,reserved)==64);
#else
_Static_assert(sizeof(PaintWetToolInfoDual)==80 && offsetof(PaintWetToolInfoDual,secondary_revision)==16 && offsetof(PaintWetToolInfoDual,primary_center_linear_premul_rgba)==32 && offsetof(PaintWetToolInfoDual,secondary_center_linear_premul_rgba)==48 && offsetof(PaintWetToolInfoDual,reserved)==64,"wet dual layout");
_Static_assert(sizeof(PaintWetLoadDesc)==32 && offsetof(PaintWetLoadDesc,linear_rgba)==16,"wet load layout");
_Static_assert(sizeof(PaintWetToolInfo)==40 && offsetof(PaintWetToolInfo,center_linear_premul_rgba)==24,"wet info layout");
_Static_assert(sizeof(PaintCompositeTipPose)==64 && offsetof(PaintCompositeTipPose,primary)==8 && offsetof(PaintCompositeTipPose,secondary)==32 && offsetof(PaintCompositeTipPose,reserved)==56,"composite pose layout");
_Static_assert(sizeof(PaintBrushSettings) == 64 && offsetof(PaintBrushSettings, tip) == 24, "brush settings layout");
_Static_assert(sizeof(PaintBrushPresetInfo) == 64 && sizeof(PaintBrushLibraryInfo) == 32, "brush catalog layout");
_Static_assert(sizeof(PaintBrushToolDesc) == 56 && offsetof(PaintBrushToolDesc, linear_rgba) == 24, "brush tool layout");
_Static_assert(sizeof(PaintTipPose)==24 && offsetof(PaintTipPose,radius)==8 && offsetof(PaintTipPose,roundness)==16,"pose layout");
#endif
static int brush_catalog_smoke(void) {
    PaintVersion version;
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
    PaintWetTool *wet=NULL;
    PaintWetToolInfo wet_info;
    PaintWetLoadDesc wet_load;
    PaintWetToolInfoDual wet_dual;
    PaintBrushBackgroundDesc background;
    PaintBrushPose *pose=NULL;
    PaintPoint point;
    PaintTipPose tip_pose;
    PaintCompositeTipPose composite;
    memset(&version,0,sizeof(version));version.struct_size=sizeof(version);
    if(paint_core_version(&version)!=PAINT_OK || version.minor<52) return 57;
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
    required=999;
    if (PAINT_BRUSH_CURSOR_IMAGE_EDGE != 256 || paint_brush_library_tip_cursor_png(library, (const uint8_t *)id, sizeof(id)-1, text, sizeof(text), &required) != PAINT_NOT_FOUND || required != 0) return 118;
    if (paint_brush_library_package_read(library, NULL, 0, text, sizeof(text), &required) != PAINT_INVALID_ARGUMENT || required != 0) return 19;
    if (paint_brush_library_package_read_taper(library,NULL,0,text,sizeof(text),&required)!=PAINT_INVALID_ARGUMENT || required!=0) return 58;
    if (paint_brush_library_package_write_taper(library,NULL,0,NULL,0)!=PAINT_INVALID_ARGUMENT) return 59;
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
    if (paint_brush_library_source_info(library, (const uint8_t *)id, sizeof(id)-1, text, sizeof(text), &required) != PAINT_NOT_FOUND || required != 0) return 19;
    if (paint_brush_library_abr_read(library, NULL, 0, text, sizeof(text), &required) != PAINT_INVALID_ARGUMENT || required != 0) return 20;
    if (paint_brush_library_current_state(library, state, sizeof(state), &state_size) != PAINT_OK) return 12;
    if (paint_brush_library_prepare_state(library, state, state_size, &preparation) != PAINT_OK || !preparation) return 13;
    if (paint_brush_library_prepare_state_taper(library,state,state_size,&preparation)!=PAINT_OK || !preparation) return 60;
    if(paint_brush_library_prepare_state_wet_dual(library,state,state_size,&preparation)!=PAINT_OK || !preparation) return 61;
    started = clock();
    do {
        status = paint_brush_library_prepared_state(library, preparation, state, sizeof(state), &state_size);
        if (clock() - started > 5 * CLOCKS_PER_SEC) return 14;
    } while (status == PAINT_BUSY);
    if (status != PAINT_OK) return 15;
    memset(&point,0,sizeof(point));point.struct_size=sizeof(point);point.pressure=.5f;point.tool=PAINT_TOOL_PEN;point.capabilities=PAINT_INPUT_PRESSURE;
    memset(&tip_pose,0,sizeof(tip_pose));tip_pose.struct_size=sizeof(tip_pose);
    if(paint_brush_pose_create(&pose)!=PAINT_OK) return 21;
    if(paint_brush_pose_bind_prepared(pose,library,preparation,(const uint8_t*)id,sizeof(id)-1,10)!=PAINT_OK) return 22;
    if(paint_brush_pose_read(pose,&point,1,0,0,0,&tip_pose)!=PAINT_OK || tip_pose.radius!=5 || tip_pose.roundness!=.2f) return 23;
    memset(&composite,0,sizeof(composite));
    if(paint_brush_pose_read_composite(pose,&point,1,0,0,0,&composite)!=PAINT_OK || composite.struct_size!=64 || composite.flags!=0 || composite.primary.radius!=tip_pose.radius || composite.secondary.radius!=0 || composite.reserved!=0) return 50;
    if(paint_brush_pose_clear(pose)!=PAINT_OK) return 24;
    if(paint_brush_pose_read_composite(pose,&point,1,0,0,0,&composite)!=PAINT_INVALID_ARGUMENT || composite.flags!=0 || composite.primary.radius!=0 || composite.secondary.radius!=0) return 51;

    if(paint_brush_pose_read(pose,&point,1,0,0,0,&tip_pose)!=PAINT_INVALID_ARGUMENT || tip_pose.radius!=0) return 25;
    if(paint_brush_pose_destroy(&pose)!=PAINT_OK || pose!=NULL) return 26;
    memset(&wet_info,0,sizeof(wet_info));
    memset(&wet_load,0,sizeof(wet_load));wet_load.struct_size=sizeof(wet_load);wet_load.radius=4;wet_load.linear_rgba[3]=1;
    if(paint_wet_tool_create(&wet)!=PAINT_OK || wet==NULL) return 52;
    if(paint_wet_tool_info(wet,&wet_info)!=PAINT_OK || wet_info.struct_size!=40 || wet_info.flags!=0 || wet_info.edge!=0) return 53;
    memset(&wet_dual,0,sizeof(wet_dual));memset(&background,0,sizeof(background));background.struct_size=sizeof(background);background.linear_rgba[3]=1;
    if(paint_wet_tool_info_dual(wet,&wet_dual)!=PAINT_OK || wet_dual.struct_size!=80 || wet_dual.flags!=0 || wet_dual.secondary_revision!=0) return 62;
    if(paint_wet_tool_reload_prepared_dual(wet,library,preparation,(const uint8_t*)id,sizeof(id)-1,&wet_load,&background)!=PAINT_INVALID_ARGUMENT) return 63;
    if(paint_brush_library_package_read_wet_dual(library,NULL,0,text,sizeof(text),&required)!=PAINT_INVALID_ARGUMENT || required!=0) return 64;
    if(paint_brush_library_package_write_wet_dual(library,NULL,0,NULL,0)!=PAINT_INVALID_ARGUMENT) return 65;
    if(paint_wet_tool_clean(wet)!=PAINT_OK) return 54;
    if(paint_wet_tool_reload_prepared(wet,library,preparation,(const uint8_t*)id,sizeof(id)-1,&wet_load)!=PAINT_INVALID_ARGUMENT) return 55;
    if(paint_wet_tool_destroy(&wet)!=PAINT_OK || wet!=NULL) return 56;
    if (paint_brush_library_destroy(&library) != PAINT_OK || library != NULL) return 11;
    return 0;
}
#endif
