#ifndef DRAWVERSE_ASYNC_SMOKE_H
#define DRAWVERSE_ASYNC_SMOKE_H
#include "paint_api.h"
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#ifdef __cplusplus
#define ABI_ASSERT static_assert
#else
#define ABI_ASSERT _Static_assert
#endif
ABI_ASSERT(sizeof(PaintCommand) == 176, "command layout drift");
ABI_ASSERT(offsetof(PaintCommand, point) == 64, "nested point offset drift");
ABI_ASSERT(sizeof(PaintSessionInfo) == 104, "session info layout drift");
ABI_ASSERT(sizeof(PaintHistoryEntry) == 16, "history entry layout drift");
ABI_ASSERT(offsetof(PaintHistoryEntry, depth) == 8, "history depth offset drift");
ABI_ASSERT(sizeof(PaintViewport) == 64, "viewport layout drift");
ABI_ASSERT(sizeof(PaintFrameInfo) == 96, "frame layout drift");
ABI_ASSERT(sizeof(PaintFileRequest) == 64, "file request layout drift");
ABI_ASSERT(offsetof(PaintFileRequest, path) == 16, "file path offset drift");
ABI_ASSERT(sizeof(PaintFileJobInfo) == 64, "file info layout drift");
ABI_ASSERT(sizeof(PaintLayerAppearance) == 32, "layer appearance layout drift");
ABI_ASSERT(offsetof(PaintLayerAppearance, fill) == 16, "fill offset drift");
ABI_ASSERT(offsetof(PaintLayerAppearance, dissolve_seed) == 28, "seed offset drift");
ABI_ASSERT(sizeof(PaintLayerHierarchy)==24,"hierarchy layout drift");
ABI_ASSERT(sizeof(PaintGroupRequest)==40,"group request layout drift");
ABI_ASSERT(offsetof(PaintGroupRequest,name)==24,"group name offset drift");
ABI_ASSERT(sizeof(PaintLayerDrop)==24,"layer drop layout drift");
ABI_ASSERT(offsetof(PaintLayerDrop,target_id)==16,"drop target offset drift");
ABI_ASSERT(sizeof(PaintLayerClipping)==16,"clipping layout drift");
ABI_ASSERT(offsetof(PaintLayerClipping,base_layer_id)==8,"clipping base offset drift");
ABI_ASSERT(sizeof(PaintSelectionEdit)==64,"selection edit layout drift");
ABI_ASSERT(offsetof(PaintSelectionEdit,x)==32,"selection geometry offset drift");
ABI_ASSERT(sizeof(PaintSelectionInfo)==16,"selection info layout drift");
ABI_ASSERT(sizeof(PaintSelectionStep)==48,"selection step layout drift");
ABI_ASSERT(offsetof(PaintSelectionStep,x)==16,"selection step offset drift");
#undef ABI_ASSERT
#define ASYNC_CHECK(test) do { if (!(test)) { fprintf(stderr, "Async ABI check failed at line %d: %s\n", __LINE__, #test); return 1; } } while (0)

// Compiled and executed by real C11 and C++20 callers, against static and shared ABI.
static int async_smoke(PaintCore *core) {
    PaintSession *session = NULL;
    PaintDocumentDesc desc;
    PaintSessionInfo info;
    PaintCommand command;
    PaintViewport viewport;
    PaintFrameInfo frame;
    PaintLayerInfo layer;
    PaintTile tile;
    PaintFileRequest file_request;
    PaintFileJobInfo file_info;
    PaintLayerAppearance appearance;
    PaintHistoryEntry history;
    uint64_t job_id = 0;
    uint64_t sequence = 0, request = 0, required = 0;
    uint8_t pixel[4] = {0}, name[1024] = {0};
    clock_t start;
    PaintStatus status;
    memset(&desc, 0, sizeof(desc)); desc.struct_size = sizeof(desc); desc.width = 64; desc.height = 64;
    desc.working_space = PAINT_WORKING_LINEAR_SRGB; desc.pixel_format = PAINT_STORAGE_RGBA32F_PREMULTIPLIED;
    ASYNC_CHECK(paint_session_create(core, &desc, &session) == PAINT_OK && session != NULL);
    ASYNC_CHECK(paint_core_destroy(&core) == PAINT_BUSY);
    memset(&command, 0, sizeof(command)); command.struct_size = sizeof(command);
    command.kind = PAINT_COMMAND_BEGIN_STROKE;
    command.stroke.struct_size = sizeof(command.stroke); command.stroke.mode = PAINT_MODE_PAINT;
    command.stroke.radius = 4; command.stroke.opacity = 1; command.stroke.spacing = .25f;
    command.stroke.linear_rgba[0] = 1; command.stroke.linear_rgba[3] = 1;
    command.point.struct_size = sizeof(command.point); command.point.tool = PAINT_TOOL_MOUSE;
    command.point.x = 16.5; command.point.y = 16.5; command.point.pressure = 1;
    ASYNC_CHECK(paint_session_submit(core, session, &command, &sequence) == PAINT_OK);
    command.kind = PAINT_COMMAND_END_STROKE;
    ASYNC_CHECK(paint_session_submit(core, session, &command, &sequence) == PAINT_OK);
    start = clock();
    do {
        memset(&info, 0, sizeof(info)); info.struct_size = sizeof(info);
        ASYNC_CHECK(paint_session_info(core, session, &info) == PAINT_OK);
        ASYNC_CHECK(clock() - start < 5 * CLOCKS_PER_SEC);
    } while (info.completed_sequence < sequence);
    ASYNC_CHECK(info.undo_depth == 1 && info.layer_count == 1 && info.document_generation == 1);
    memset(&history, 0, sizeof(history)); history.struct_size = sizeof(history);
    ASYNC_CHECK(paint_session_history_entry(core,session,info.publication,1,&history)==PAINT_OK);
    ASYNC_CHECK(history.kind==PAINT_HISTORY_BRUSH && history.depth==1 && history.reserved==0);
    memset(&layer, 0, sizeof(layer)); layer.struct_size = sizeof(layer);
    ASYNC_CHECK(paint_session_layer_info(core, session, info.publication, 0, &layer) == PAINT_OK);
    ASYNC_CHECK(paint_session_layer_name(core, session, info.publication, layer.layer_id, name, sizeof(name), &required) == PAINT_OK);
    ASYNC_CHECK(required == layer.name_length && required > 0);
    memset(&viewport, 0, sizeof(viewport)); viewport.struct_size = sizeof(viewport);
    viewport.enabled = 1; viewport.document_generation = info.document_generation;
    viewport.x = 16; viewport.y = 16; viewport.width = 1; viewport.height = 1;
    viewport.pixel_width = 1; viewport.pixel_height = 1;
    ASYNC_CHECK(paint_session_set_viewport(core, session, &viewport, &request) == PAINT_OK && request > 0);
    start = clock();
    do {
        memset(&frame, 0, sizeof(frame)); frame.struct_size = sizeof(frame);
        status = paint_session_frame_info(core, session, 0, &frame);
        ASYNC_CHECK(status == PAINT_OK || status == PAINT_BUSY);
        ASYNC_CHECK(clock() - start < 5 * CLOCKS_PER_SEC);
    } while (status == PAINT_BUSY || frame.request_id != request || frame.revision < info.revision);
    memset(&tile, 0, sizeof(tile)); tile.struct_size = sizeof(tile); tile.format = PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED;
    tile.data = pixel; tile.capacity = sizeof(pixel); tile.stride = sizeof(pixel);
    ASYNC_CHECK(paint_session_read_frame(core, session, 0, frame.request_id, frame.frame_id, &tile) == PAINT_OK);
    ASYNC_CHECK(pixel[0] == 255 && pixel[1] == 0 && pixel[2] == 0 && pixel[3] == 255);
    memset(&appearance,0,sizeof(appearance)); appearance.struct_size=sizeof(appearance);
    ASYNC_CHECK(paint_session_layer_appearance(core,session,info.publication,layer.layer_id,&appearance)==PAINT_OK);
    ASYNC_CHECK(appearance.fill==1 && appearance.locks==0 && appearance.blend_mode==0);
    appearance.fill=.5f; appearance.blend_mode=3; appearance.locks=PAINT_LOCK_POSITION;
    ASYNC_CHECK(paint_session_set_layer_appearance(core,session,layer.layer_id,&appearance,&sequence)==PAINT_OK);
    start=clock();
    do {
        info.struct_size=sizeof(info); ASYNC_CHECK(paint_session_info(core,session,&info)==PAINT_OK);
        ASYNC_CHECK(clock()-start<5*CLOCKS_PER_SEC);
    } while(info.completed_sequence<sequence);
    ASYNC_CHECK(info.last_error_status==PAINT_OK);
    ASYNC_CHECK(paint_session_layer_appearance(core,session,info.publication,layer.layer_id,&appearance)==PAINT_OK);
    ASYNC_CHECK(appearance.fill==.5f && appearance.blend_mode==3 && appearance.locks==PAINT_LOCK_POSITION);
    ASYNC_CHECK(paint_session_move_layer(core,session,layer.layer_id,1,0,&sequence)==PAINT_OK);
    start=clock();
    do {
        info.struct_size=sizeof(info); ASYNC_CHECK(paint_session_info(core,session,&info)==PAINT_OK);
        ASYNC_CHECK(clock()-start<5*CLOCKS_PER_SEC);
    } while(info.completed_sequence<sequence);
    ASYNC_CHECK(info.last_error_status==PAINT_LAYER_LOCKED);
    memset(&file_request, 0, sizeof(file_request)); file_request.struct_size = sizeof(file_request);
    file_request.kind = PAINT_FILE_SAVE; file_request.format = PAINT_FILE_OPENRASTER; file_request.quality = 95;
    file_request.document_generation = info.document_generation;
    file_request.path = (const uint8_t *)DRAWVERSE_ABI_SMOKE_FILE;
    file_request.path_length = strlen(DRAWVERSE_ABI_SMOKE_FILE);
    file_request.linear_background[0] = file_request.linear_background[1] = file_request.linear_background[2] = file_request.linear_background[3] = 1;
    ASYNC_CHECK(paint_session_file_submit(core, session, &file_request, &job_id) == PAINT_OK && job_id > 0);
    start = clock();
    do {
        memset(&file_info, 0, sizeof(file_info)); file_info.struct_size = sizeof(file_info);
        ASYNC_CHECK(paint_session_file_info(core, session, job_id, &file_info) == PAINT_OK);
        ASYNC_CHECK(clock() - start < 5 * CLOCKS_PER_SEC);
    } while (file_info.state < PAINT_FILE_SUCCEEDED);
    ASYNC_CHECK(file_info.state == PAINT_FILE_SUCCEEDED && file_info.status == PAINT_OK);
    ASYNC_CHECK(paint_session_file_cancel(core, session, job_id) == PAINT_BUSY);
    ASYNC_CHECK(paint_session_file_message(core, session, job_id, NULL, 0, &required) == PAINT_OK && required == 0);
    file_request.kind = PAINT_FILE_OPEN; file_request.format = PAINT_FILE_AUTO;
    ASYNC_CHECK(paint_session_file_submit(core, session, &file_request, &job_id) == PAINT_OK);
    start = clock();
    do {
        memset(&file_info, 0, sizeof(file_info)); file_info.struct_size = sizeof(file_info);
        ASYNC_CHECK(paint_session_file_info(core, session, job_id, &file_info) == PAINT_OK);
        ASYNC_CHECK(clock() - start < 5 * CLOCKS_PER_SEC);
    } while (file_info.state < PAINT_FILE_SUCCEEDED);
    ASYNC_CHECK(file_info.state == PAINT_FILE_SUCCEEDED && file_info.result_generation == 2);
    {
        PaintGroupRequest group;
        PaintLayerHierarchy hierarchy;
        uint64_t group_id;
        memset(&group,0,sizeof(group)); group.struct_size=sizeof(group);group.kind=PAINT_GROUP_WRAP;
        group.layer_id=1;group.name=(const uint8_t*)"Group";group.name_length=5;
        ASYNC_CHECK(paint_session_group(core,session,&group,&sequence)==PAINT_OK);
        start=clock();
        do {info.struct_size=sizeof(info);ASYNC_CHECK(paint_session_info(core,session,&info)==PAINT_OK);ASYNC_CHECK(clock()-start<5*CLOCKS_PER_SEC);} while(info.completed_sequence<sequence);
        ASYNC_CHECK(info.layer_count==2);group_id=info.active_layer_id;
        memset(&hierarchy,0,sizeof(hierarchy));hierarchy.struct_size=sizeof(hierarchy);
        ASYNC_CHECK(paint_session_layer_hierarchy(core,session,info.publication,1,&hierarchy)==PAINT_OK);
        ASYNC_CHECK(hierarchy.kind==PAINT_LAYER_PIXEL && hierarchy.parent_id==group_id && hierarchy.depth==1);
        ASYNC_CHECK(paint_session_layer_hierarchy(core,session,info.publication,group_id,&hierarchy)==PAINT_OK);
        ASYNC_CHECK(hierarchy.kind==PAINT_LAYER_GROUP && hierarchy.parent_id==0 && hierarchy.depth==0);
        group.kind=PAINT_GROUP_UNGROUP;group.layer_id=group_id;group.name=NULL;group.name_length=0;
        ASYNC_CHECK(paint_session_group(core,session,&group,&sequence)==PAINT_OK);
        start=clock();
        do {info.struct_size=sizeof(info);ASYNC_CHECK(paint_session_info(core,session,&info)==PAINT_OK);ASYNC_CHECK(clock()-start<5*CLOCKS_PER_SEC);} while(info.completed_sequence<sequence);
        ASYNC_CHECK(info.layer_count==1);
    }
    {
        PaintLayerDrop drop;
        PaintLayerHierarchy hierarchy;
        uint64_t mask_id;
        command.kind=PAINT_COMMAND_ADD_MASK;command.layer_id=1;
        ASYNC_CHECK(paint_session_submit(core,session,&command,&sequence)==PAINT_OK);
        start=clock();
        do {info.struct_size=sizeof(info);ASYNC_CHECK(paint_session_info(core,session,&info)==PAINT_OK);ASYNC_CHECK(clock()-start<5*CLOCKS_PER_SEC);} while(info.completed_sequence<sequence);
        ASYNC_CHECK(info.layer_count==2);mask_id=info.active_layer_id;
        memset(&hierarchy,0,sizeof(hierarchy));hierarchy.struct_size=sizeof(hierarchy);
        ASYNC_CHECK(paint_session_layer_hierarchy(core,session,info.publication,mask_id,&hierarchy)==PAINT_OK);
        ASYNC_CHECK(hierarchy.kind==PAINT_LAYER_MASK && hierarchy.parent_id==1 && hierarchy.depth==1);
        memset(&drop,0,sizeof(drop));drop.struct_size=sizeof(drop);drop.layer_id=1;
        ASYNC_CHECK(paint_session_drop_layer(core,session,&drop,&sequence)==PAINT_OK);
        start=clock();
        do {info.struct_size=sizeof(info);ASYNC_CHECK(paint_session_info(core,session,&info)==PAINT_OK);ASYNC_CHECK(clock()-start<5*CLOCKS_PER_SEC);} while(info.completed_sequence<sequence);
        ASYNC_CHECK(info.last_error_status==PAINT_OK);
        command.kind=PAINT_COMMAND_NEW_WHITE_DOCUMENT;command.width=64;command.height=64;
        ASYNC_CHECK(paint_session_submit(core,session,&command,&sequence)==PAINT_OK);
        start=clock();
        do {info.struct_size=sizeof(info);ASYNC_CHECK(paint_session_info(core,session,&info)==PAINT_OK);ASYNC_CHECK(clock()-start<5*CLOCKS_PER_SEC);} while(info.completed_sequence<sequence);
        ASYNC_CHECK(info.layer_count==1 && info.undo_depth==0);
    }
    {
        PaintLayerClipping clipping;
        memset(&clipping,0,sizeof(clipping));clipping.struct_size=sizeof(clipping);
        command.kind=PAINT_COMMAND_ADD_LAYER;command.text=(const uint8_t*)"Clipped";command.text_length=7;
        ASYNC_CHECK(paint_session_submit(core,session,&command,&sequence)==PAINT_OK);
        start=clock();
        do {info.struct_size=sizeof(info);ASYNC_CHECK(paint_session_info(core,session,&info)==PAINT_OK);ASYNC_CHECK(clock()-start<5*CLOCKS_PER_SEC);} while(info.completed_sequence<sequence);
        clipping.enabled=1;
        ASYNC_CHECK(paint_session_set_layer_clipping(core,session,info.active_layer_id,&clipping,&sequence)==PAINT_OK);
        start=clock();
        do {info.struct_size=sizeof(info);ASYNC_CHECK(paint_session_info(core,session,&info)==PAINT_OK);ASYNC_CHECK(clock()-start<5*CLOCKS_PER_SEC);} while(info.completed_sequence<sequence);
        ASYNC_CHECK(info.last_error_status==PAINT_OK);
        ASYNC_CHECK(paint_session_layer_clipping(core,session,info.publication,info.active_layer_id,&clipping)==PAINT_OK);
        ASYNC_CHECK(clipping.enabled==1 && clipping.base_layer_id==1);
        ASYNC_CHECK(paint_session_layer_clipping(core,session,info.publication+99,info.active_layer_id,&clipping)==PAINT_BUSY);
        clipping.enabled=2;sequence=999;
        ASYNC_CHECK(paint_session_set_layer_clipping(core,session,info.active_layer_id,&clipping,&sequence)==PAINT_INVALID_ARGUMENT && sequence==0);
    }
    {
        PaintSelectionEdit selection;PaintSelectionInfo summary;PaintSelectionStep step;
        memset(&selection,0,sizeof(selection));selection.struct_size=sizeof(selection);selection.action=PAINT_SELECTION_SHAPE;selection.shape=PAINT_SELECTION_ELLIPSE;selection.antialias=1;selection.x=10;selection.y=12;selection.width=30;selection.height=20;
        ASYNC_CHECK(paint_session_edit_selection(core,session,&selection,&sequence)==PAINT_OK);
        start=clock();do{info.struct_size=sizeof(info);ASYNC_CHECK(paint_session_info(core,session,&info)==PAINT_OK);ASYNC_CHECK(clock()-start<5*CLOCKS_PER_SEC);}while(info.completed_sequence<sequence);
        ASYNC_CHECK(info.last_error_status==PAINT_OK);
        memset(&summary,0,sizeof(summary));summary.struct_size=sizeof(summary);ASYNC_CHECK(paint_session_selection_info(core,session,info.publication,&summary)==PAINT_OK && summary.enabled==1 && summary.step_count==1);
        memset(&step,0,sizeof(step));step.struct_size=sizeof(step);ASYNC_CHECK(paint_session_selection_step(core,session,info.publication,0,&step)==PAINT_OK && step.shape==PAINT_SELECTION_ELLIPSE && step.width==30);
        ASYNC_CHECK(paint_session_selection_info(core,session,info.publication+99,&summary)==PAINT_BUSY);
        ASYNC_CHECK(paint_session_selection_step(core,session,info.publication,999,&step)==PAINT_NOT_FOUND);
        selection.width=-1;sequence=999;ASYNC_CHECK(paint_session_edit_selection(core,session,&selection,&sequence)==PAINT_INVALID_ARGUMENT && sequence==0);
    }
    ASYNC_CHECK(remove(DRAWVERSE_ABI_SMOKE_FILE) == 0);
    ASYNC_CHECK(paint_session_destroy(core, &session) == PAINT_OK && session == NULL);
    ASYNC_CHECK(paint_session_destroy(core, &session) == PAINT_OK);
    return 0;
}
#undef ASYNC_CHECK
#endif
