#ifndef DRAWVERSE_COMPOSITION_SMOKE_H
#define DRAWVERSE_COMPOSITION_SMOKE_H
#include "paint_api.h"
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#include <stdlib.h>
#ifdef __cplusplus
#define COMPOSITION_ASSERT static_assert
#else
#define COMPOSITION_ASSERT _Static_assert
#endif
COMPOSITION_ASSERT(sizeof(PaintCompositionCapabilities)==88,"composition capabilities layout");
COMPOSITION_ASSERT(sizeof(PaintCompositionRequest)==144,"composition request layout");
COMPOSITION_ASSERT(sizeof(PaintCompositionKey)==56,"composition key layout");
COMPOSITION_ASSERT(sizeof(PaintCompositionInfo)==136,"composition info layout");
COMPOSITION_ASSERT(sizeof(PaintCompositionNode)==64,"composition node layout");
COMPOSITION_ASSERT(sizeof(PaintCompositionOperation)==32,"composition operation layout");
COMPOSITION_ASSERT(offsetof(PaintCompositionOperation,reserved)==24,"composition operation reserved offset");
COMPOSITION_ASSERT(sizeof(PaintCompositionTile)==40,"composition tile layout");
COMPOSITION_ASSERT(sizeof(PaintCompositionUsage)==40,"composition usage layout");
COMPOSITION_ASSERT(sizeof(PaintTransformDisplayGeometry)==104,"transform display geometry layout");
COMPOSITION_ASSERT(offsetof(PaintTransformDisplayGeometry,inverse)==24,"transform inverse offset");
COMPOSITION_ASSERT(offsetof(PaintCompositionRequest,viewport)==16,"composition viewport offset");
COMPOSITION_ASSERT(offsetof(PaintCompositionInfo,key)==8,"composition key offset");
#undef COMPOSITION_ASSERT
#define COMPOSITION_CHECK(test) do {if(!(test)){fprintf(stderr,"Composition ABI check failed at %d: %s\n",__LINE__,#test);return 1;}}while(0)
static int composition_smoke(PaintCore *core) {
    {
        PaintTransform geometry;PaintTransformDisplayGeometry mesh;float *vertices;
        memset(&geometry,0,sizeof(geometry));geometry.struct_size=sizeof(geometry);geometry.interpolation=2;
        for(int i=0;i<16;++i){geometry.points[i*2]=(i%4)*32.;geometry.points[i*2+1]=(i/4)*24.;}
        memset(&mesh,0,sizeof(mesh));mesh.struct_size=sizeof(mesh);
        COMPOSITION_CHECK(paint_transform_display_geometry(&geometry,&mesh,NULL,0)==PAINT_BUFFER_TOO_SMALL);
        COMPOSITION_CHECK(mesh.version==1 && mesh.columns>=19 && mesh.rows>=19 && mesh.vertex_bytes<=145*145*16);
        vertices=(float*)malloc((size_t)mesh.vertex_bytes);COMPOSITION_CHECK(vertices!=NULL);
        COMPOSITION_CHECK(paint_transform_display_geometry(&geometry,&mesh,vertices,mesh.vertex_bytes/4)==PAINT_OK);
        COMPOSITION_CHECK(vertices[0]==0.f && vertices[1]==0.f && vertices[2]==0.f && vertices[3]==0.f);
        COMPOSITION_CHECK(vertices[mesh.vertex_bytes/4-4]==96.f && vertices[mesh.vertex_bytes/4-3]==72.f);
        free(vertices);
    }
    PaintSession *session=NULL;
    PaintDocumentDesc desc;
    PaintSessionInfo state;
    PaintCommand command;
    PaintCompositionCapabilities caps;
    PaintCompositionRequest request;
    PaintCompositionInfo info;
    PaintCompositionNode node;
    PaintCompositionTile tile;
    PaintCompositionUsage usage;
    uint64_t sequence=0,lease=0,required=0;
    uint8_t bytes[65536];float red=0;
    clock_t start;
    memset(&caps,0,sizeof(caps));caps.struct_size=sizeof(caps);
    COMPOSITION_CHECK(paint_core_composition_capabilities(core,1,&caps)==PAINT_OK);
    COMPOSITION_CHECK((caps.features & (PAINT_COMPOSITION_FIXED_MASK|PAINT_COMPOSITION_BLEND_MODES))==(PAINT_COMPOSITION_FIXED_MASK|PAINT_COMPOSITION_BLEND_MODES));
    COMPOSITION_CHECK(PAINT_COMPOSITION_NODE_FIXED_MASK==(UINT64_C(1)<<32));
    COMPOSITION_CHECK(caps.max_read_bytes==sizeof(bytes) && caps.tile_edge==64 && caps.max_leases_per_session==2);
    memset(&desc,0,sizeof(desc));desc.struct_size=sizeof(desc);desc.width=64;desc.height=64;
    desc.working_space=PAINT_WORKING_LINEAR_SRGB;desc.pixel_format=PAINT_STORAGE_RGBA32F_PREMULTIPLIED;
    COMPOSITION_CHECK(paint_session_create(core,&desc,&session)==PAINT_OK);
    memset(&command,0,sizeof(command));command.struct_size=sizeof(command);command.kind=PAINT_COMMAND_BEGIN_STROKE;
    command.stroke.struct_size=sizeof(command.stroke);command.stroke.mode=PAINT_MODE_PAINT;
    command.stroke.radius=8;command.stroke.opacity=1;command.stroke.spacing=0.15f;
    command.stroke.linear_rgba[0]=1;command.stroke.linear_rgba[3]=1;
    command.point.struct_size=sizeof(command.point);command.point.x=24;command.point.y=24;
    command.point.pressure=1;command.point.tool=PAINT_TOOL_MOUSE;
    COMPOSITION_CHECK(paint_session_submit(core,session,&command,&sequence)==PAINT_OK);
    command.kind=PAINT_COMMAND_END_STROKE;
    COMPOSITION_CHECK(paint_session_submit(core,session,&command,&sequence)==PAINT_OK);
    start=clock();
    do {
        memset(&state,0,sizeof(state));state.struct_size=sizeof(state);
        COMPOSITION_CHECK(paint_session_info(core,session,&state)==PAINT_OK);
        COMPOSITION_CHECK(clock()-start<10*CLOCKS_PER_SEC);
    }while(state.completed_sequence<sequence);
    COMPOSITION_CHECK(state.last_error_status==PAINT_OK && state.undo_depth==1);
    memset(&request,0,sizeof(request));request.struct_size=sizeof(request);request.version=1;request.features=caps.features;
    request.viewport.struct_size=sizeof(request.viewport);request.viewport.enabled=1;
    request.viewport.width=64;request.viewport.height=64;request.viewport.pixel_width=64;request.viewport.pixel_height=64;
    request.viewport.document_generation=state.document_generation;request.revision=state.revision;
    request.request_id=1;request.device_epoch=1;request.max_nodes=caps.max_nodes;request.max_tiles=caps.max_tiles;
    request.max_pinned_bytes=caps.max_pinned_bytes;request.max_metadata_bytes=caps.max_metadata_bytes;
    COMPOSITION_CHECK(paint_session_request_composition(core,session,&request,&lease)==PAINT_OK && lease!=0);
    start=clock();
    do {
        memset(&info,0,sizeof(info));info.struct_size=sizeof(info);
        COMPOSITION_CHECK(paint_session_composition_info(core,session,lease,&info)==PAINT_OK);
        COMPOSITION_CHECK(clock()-start<10*CLOCKS_PER_SEC);
    }while(info.state==PAINT_COMPOSITION_PENDING);
    COMPOSITION_CHECK(info.state==PAINT_COMPOSITION_READY && info.status==PAINT_OK && info.node_count==1 && info.tile_count==1);
    COMPOSITION_CHECK(info.key.revision==state.revision && info.key.request_id==1);
    memset(&node,0,sizeof(node));node.struct_size=sizeof(node);
    COMPOSITION_CHECK(paint_session_composition_node(core,session,&info.key,0,&node)==PAINT_OK);
    COMPOSITION_CHECK(node.kind==PAINT_COMPOSITION_RASTER && node.mask_index==UINT32_MAX && node.tile_count==1);
    memset(&tile,0,sizeof(tile));tile.struct_size=sizeof(tile);
    COMPOSITION_CHECK(paint_session_composition_tile(core,session,&info.key,0,&tile)==PAINT_OK && tile.content_id!=0);
    COMPOSITION_CHECK(paint_session_read_composition_tile(core,session,&info.key,0,NULL,0,&required)==PAINT_BUFFER_TOO_SMALL && required==sizeof(bytes));
    COMPOSITION_CHECK(paint_session_read_composition_tile(core,session,&info.key,0,bytes,sizeof(bytes),&required)==PAINT_OK);
    memcpy(&red,bytes+(24*64+24)*16,sizeof(red));COMPOSITION_CHECK(red==1.f);
    COMPOSITION_CHECK(paint_session_release_composition(core,session,lease)==PAINT_OK);
    COMPOSITION_CHECK(paint_session_composition_info(core,session,lease,&info)==PAINT_CANCELLED);
    /* Exercise new action numbers and sequence-bound clipboard in every C/C++
       static/shared consumer, including paste into the now empty document. */
    {
        const uint64_t cut_id=node.layer_id;
        uint8_t *copied;
        COMPOSITION_CHECK(PAINT_LAYER_ACTION_CUT==6 && PAINT_LAYER_ACTION_CUT_SELECTED_PIXELS==7);
        COMPOSITION_CHECK(paint_session_layer_action(core,session,PAINT_LAYER_ACTION_CUT,&cut_id,1,0,NULL,0,&sequence)==PAINT_OK);
        start=clock();do {
            COMPOSITION_CHECK(paint_session_info(core,session,&state)==PAINT_OK);
            COMPOSITION_CHECK(clock()-start<10*CLOCKS_PER_SEC);
        }while(state.completed_sequence<sequence);
        COMPOSITION_CHECK(state.last_error_status==PAINT_OK && state.layer_count==0 && state.undo_depth==2);
        COMPOSITION_CHECK(paint_session_read_layer_clipboard(core,session,sequence,NULL,0,&required)==PAINT_BUFFER_TOO_SMALL && required>16 && required<=UINT64_C(268435456));
        copied=(uint8_t*)malloc((size_t)required);COMPOSITION_CHECK(copied!=NULL);
        COMPOSITION_CHECK(paint_session_read_layer_clipboard(core,session,sequence,copied,required,&required)==PAINT_OK);
        COMPOSITION_CHECK(paint_session_layer_action(core,session,PAINT_LAYER_ACTION_PASTE,NULL,0,0,copied,required,&sequence)==PAINT_OK);
        free(copied);start=clock();do {
            COMPOSITION_CHECK(paint_session_info(core,session,&state)==PAINT_OK);
            COMPOSITION_CHECK(clock()-start<10*CLOCKS_PER_SEC);
        }while(state.completed_sequence<sequence);
        COMPOSITION_CHECK(state.last_error_status==PAINT_OK && state.layer_count==1 && state.undo_depth==3);
    }
    {
        PaintCompositionOperation operation;
        const uint64_t grouped=state.active_layer_id;
        COMPOSITION_CHECK((caps.features & PAINT_COMPOSITION_GROUPS)!=0);
        COMPOSITION_CHECK(paint_session_group_layers(core,session,&grouped,1,(const uint8_t*)"ABI group",9,&sequence)==PAINT_OK);
        start=clock();do {
            COMPOSITION_CHECK(paint_session_info(core,session,&state)==PAINT_OK);
            COMPOSITION_CHECK(clock()-start<10*CLOCKS_PER_SEC);
        }while(state.completed_sequence<sequence);
        COMPOSITION_CHECK(state.layer_count==2 && state.undo_depth==4 && state.last_error_status==PAINT_OK);
        request.revision=state.revision;request.request_id=2;
        COMPOSITION_CHECK(paint_session_request_composition(core,session,&request,&lease)==PAINT_OK);
        start=clock();do {
            COMPOSITION_CHECK(paint_session_composition_info(core,session,lease,&info)==PAINT_OK);
            COMPOSITION_CHECK(clock()-start<10*CLOCKS_PER_SEC);
        }while(info.state==PAINT_COMPOSITION_PENDING);
        COMPOSITION_CHECK(info.state==PAINT_COMPOSITION_READY && (info.required_features & PAINT_COMPOSITION_GROUPS)!=0);
        COMPOSITION_CHECK(info.reserved[0]==4 && info.reserved[1]>>32==4);
        memset(&operation,0,sizeof(operation));operation.struct_size=sizeof(operation);
        COMPOSITION_CHECK(paint_session_composition_operation(core,session,&info.key,3,&operation)==PAINT_OK);
        COMPOSITION_CHECK(operation.kind==PAINT_COMPOSITION_INFLUENCE && operation.node_index==1 && operation.output==1 && operation.source==3 && operation.backdrop==0);
        COMPOSITION_CHECK(paint_session_composition_operation(core,session,&info.key,4,&operation)==PAINT_NOT_FOUND && operation.kind==0);
        COMPOSITION_CHECK(paint_session_release_composition(core,session,lease)==PAINT_OK);
        COMPOSITION_CHECK(paint_session_composition_operation(core,session,&info.key,0,&operation)==PAINT_CANCELLED);
    }
    COMPOSITION_CHECK(paint_session_destroy(core,&session)==PAINT_OK && session==NULL);
    memset(&usage,0,sizeof(usage));usage.struct_size=sizeof(usage);
    COMPOSITION_CHECK(paint_core_composition_usage(core,&usage)==PAINT_OK && usage.plan_bytes==0 && usage.read_bytes==0);
    return 0;
}
#undef COMPOSITION_CHECK
#endif
