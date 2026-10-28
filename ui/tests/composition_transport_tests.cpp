// G1 transport vertical slice. The window displays a worker-interpreted CPU
// reference of ABI packets. GPU document composition belongs to G2.
#include "paint_api.h"
#include <QtTest>
#include <QtEndian>
#include <QQuickItem>
#include <QQuickWindow>
#include <QSGSimpleTextureNode>
#include <QSGRendererInterface>
#include <QScopeGuard>
#include <QTemporaryDir>
#include <QJsonDocument>
#include <QFile>
#include <QTimer>
#include <QThread>
#include <QDeadlineTimer>
#include <QDir>
#include <QJsonObject>
#include <array>
#include <algorithm>
#include <atomic>
#include <bit>
#include <cmath>
#include <memory>
#include <stdexcept>

namespace {
template<class T> T dto() {T result{};result.struct_size=sizeof(T);return result;}
void check(bool success,const char *message) {if(!success)throw std::runtime_error(message);}
void ok(PaintStatus status) {check(status==PAINT_OK,"ABI operation failed");}
template<class F> void await(F test) {
    QDeadlineTimer deadline(15000);
    while(!test()) {check(!deadline.hasExpired(),"worker timed out");QThread::msleep(1);}
}
using Pixel=std::array<float,4>;
struct Packet {
    PaintCompositionInfo info{};
    QVector<PaintCompositionNode> nodes;
    QVector<PaintCompositionTile> tiles;
    QVector<QByteArray> pixels;
    QImage image,reference;
    PaintCompositionUsage usage{};
    quintptr readerThread=0;
    int maxError=0;
};
Pixel sample(const Packet &packet,const PaintCompositionNode &node,int x,int y) {
    x-=node.offset_x;y-=node.offset_y;
    const auto floor64=[](int v){return v>=0?v/64:(v-63)/64;};
    const int tx=floor64(x),ty=floor64(y);
    for(quint32 i=node.first_tile;i<node.first_tile+node.tile_count;++i) {
        const auto &tile=packet.tiles[i];
        if(tile.tile_x!=tx || tile.tile_y!=ty)continue;
        const auto *at=reinterpret_cast<const uchar*>(packet.pixels[i].constData())+((y-ty*64)*64+x-tx*64)*16;
        Pixel result{};
        for(int c=0;c<4;++c)result[c]=std::bit_cast<float>(qFromLittleEndian<quint32>(at+c*4));
        return result;
    }
    return x>=0 && y>=0 && quint32(x)<node.white_width && quint32(y)<node.white_height?Pixel{1,1,1,1}:Pixel{};
}
void interpret(Packet &packet) {
    const auto &info=packet.info;
    packet.image=QImage(int(info.document_width),int(info.document_height),QImage::Format_RGBA8888_Premultiplied);
    for(int y=0;y<packet.image.height();++y)for(int x=0;x<packet.image.width();++x) {
        Pixel result{};
        for(const auto &node:packet.nodes) {
            if(node.kind!=PAINT_COMPOSITION_RASTER || !node.visible)continue;
            auto source=sample(packet,node,x,y);
            if(node.mask_index!=UINT32_MAX) {
                check(node.mask_index<quint32(packet.nodes.size()),"mask index out of range");
                const auto &mask=packet.nodes[node.mask_index];
                if(mask.visible)for(auto &v:source)v*=1-sample(packet,mask,x,y)[3]*mask.opacity;
            }
            for(auto &v:source)v*=node.opacity*node.fill;
            const float alpha=source[3]+result[3]*(1-source[3]);
            for(int c=0;c<4;++c)result[c]=std::min(alpha,source[c]+result[c]*(1-source[3]));
        }
        auto *out=packet.image.scanLine(y)+x*4;
        const float a=std::clamp(result[3],0.f,1.f);
        for(int c=0;c<3;++c) {
            const float v=a>0?std::clamp(result[c]/a,0.f,1.f):0;
            const float encoded=v<=.0031308f?12.92f*v:1.055f*std::pow(v,1.f/2.4f)-.055f;
            out[c]=uchar(std::lround(encoded*a*255));
        }
        out[3]=uchar(std::lround(a*255));
    }
}

std::shared_ptr<Packet> readPacket(const QByteArray &scratch) {
    check(QThread::currentThread()!=qApp->thread(),"pixel IO attempted on GUI thread");
    auto packet=std::make_shared<Packet>();packet->readerThread=quintptr(QThread::currentThreadId());
    PaintCore *core=nullptr;PaintSession *session=nullptr;quint64 lease=0;
    const auto cleanup=qScopeGuard([&]{
        if(lease)paint_session_release_composition(core,session,lease);
        if(session)paint_session_destroy(core,&session);
        if(core)paint_core_destroy(&core);
    });
    auto storage=dto<PaintStorageOptions>();storage.path=reinterpret_cast<const uint8_t*>(scratch.constData());
    storage.path_length=quint64(scratch.size());storage.resident_bytes=65536;storage.scratch_bytes=64*1024*1024;
    ok(paint_core_create_with_storage(&storage,&core));
    auto desc=dto<PaintDocumentDesc>();desc.width=128;desc.height=96;
    desc.working_space=PAINT_WORKING_LINEAR_SRGB;desc.pixel_format=PAINT_STORAGE_RGBA32F_PREMULTIPLIED;
    ok(paint_session_create(core,&desc,&session));
    const auto state=[&]{auto info=dto<PaintSessionInfo>();ok(paint_session_info(core,session,&info));return info;};
    const auto command=[&](PaintCommand command) {
        command.struct_size=sizeof(command);quint64 sequence=0;ok(paint_session_submit(core,session,&command,&sequence));
        await([&]{return state().completed_sequence>=sequence;});check(state().last_error_status==PAINT_OK,"document fixture command failed");
    };
    PaintCommand cmd{};cmd.kind=PAINT_COMMAND_NEW_WHITE_DOCUMENT;cmd.width=128;cmd.height=96;command(cmd);
    const QByteArray name="G1 transport patterned raster";cmd={};cmd.kind=PAINT_COMMAND_ADD_LAYER;
    cmd.text=reinterpret_cast<const uint8_t*>(name.constData());cmd.text_length=quint64(name.size());command(cmd);
    const auto raster=state().active_layer_id;
    const auto dab=[&](double x,double y,std::array<float,4> color) {
        PaintCommand begin{};begin.kind=PAINT_COMMAND_BEGIN_STROKE;begin.stroke=dto<PaintStrokeDesc>();
        begin.stroke.mode=PAINT_MODE_PAINT;begin.stroke.radius=26;begin.stroke.opacity=1;begin.stroke.spacing=.15f;
        std::copy(color.begin(),color.end(),begin.stroke.linear_rgba);begin.point=dto<PaintPoint>();
        begin.point.x=x;begin.point.y=y;begin.point.pressure=1;begin.point.tool=PAINT_TOOL_MOUSE;command(begin);
        PaintCommand end{};end.kind=PAINT_COMMAND_END_STROKE;command(end);
    };
    dab(40,36,{.8f,.2f,.4f,.65f});dab(90,70,{.15f,.7f,.9f,.8f});
    auto appearance=dto<PaintLayerAppearance>();appearance.fill=.7f;appearance.offset_x=-7;appearance.offset_y=3;
    quint64 sequence=0;ok(paint_session_set_layer_appearance(core,session,raster,&appearance,&sequence));
    await([&]{return state().completed_sequence>=sequence;});
    cmd={};cmd.kind=PAINT_COMMAND_LAYER_PROPERTIES;cmd.layer_id=raster;cmd.visible=1;cmd.opacity=.6f;command(cmd);
    cmd={};cmd.kind=PAINT_COMMAND_ADD_MASK;cmd.layer_id=raster;command(cmd);dab(56,44,{0,0,0,1});
    cmd={};cmd.kind=PAINT_COMMAND_LAYER_PROPERTIES;cmd.layer_id=state().active_layer_id;cmd.visible=1;cmd.opacity=.45f;command(cmd);
    const auto before=state();
    auto caps=dto<PaintCompositionCapabilities>();ok(paint_core_composition_capabilities(core,1,&caps));
    auto request=dto<PaintCompositionRequest>();request.version=1;request.features=caps.features;
    request.viewport=dto<PaintViewport>();request.viewport.enabled=1;request.viewport.width=128;request.viewport.height=96;
    request.viewport.pixel_width=128;request.viewport.pixel_height=96;request.viewport.document_generation=before.document_generation;
    request.revision=before.revision;request.request_id=1;request.device_epoch=1;
    // The Qt consumer has its own fixed packet budget, independent of Rust pins.
    request.max_nodes=64;request.max_tiles=64;request.max_pinned_bytes=4*1024*1024;request.max_metadata_bytes=128*1024;
    ok(paint_session_request_composition(core,session,&request,&lease));
    await([&]{packet->info=dto<PaintCompositionInfo>();ok(paint_session_composition_info(core,session,lease,&packet->info));return packet->info.state!=PAINT_COMPOSITION_PENDING;});
    check(packet->info.state==PAINT_COMPOSITION_READY,"transport plan rejected");
    check(packet->info.key.publication==before.publication,"plan metadata publication mismatch");
    check(packet->info.node_count<=64 && packet->info.tile_count<=64,"Qt packet budget");
    for(quint32 i=0;i<packet->info.node_count;++i) {
        auto node=dto<PaintCompositionNode>();ok(paint_session_composition_node(core,session,&packet->info.key,i,&node));packet->nodes.append(node);
    }
    for(quint32 i=0;i<packet->info.tile_count;++i) {
        auto tile=dto<PaintCompositionTile>();ok(paint_session_composition_tile(core,session,&packet->info.key,i,&tile));
        check(tile.required_bytes==65536,"unexpected tile format/size");QByteArray bytes(65536,Qt::Uninitialized);quint64 required=0;
        ok(paint_session_read_composition_tile(core,session,&packet->info.key,i,reinterpret_cast<uint8_t*>(bytes.data()),65536,&required));
        check(required==65536,"short tile");packet->tiles.append(tile);packet->pixels.append(std::move(bytes));
    }
    interpret(*packet);
    quint64 frameRequest=0;ok(paint_session_set_viewport(core,session,&request.viewport,&frameRequest));
    auto frame=dto<PaintFrameInfo>();
    await([&]{frame=dto<PaintFrameInfo>();const auto status=paint_session_frame_info(core,session,0,&frame);
        check(status==PAINT_OK || status==PAINT_BUSY,"CPU frame failed");return status==PAINT_OK && frame.request_id==frameRequest && frame.revision==before.revision;});
    packet->reference=QImage(128,96,QImage::Format_RGBA8888_Premultiplied);
    auto output=dto<PaintTile>();output.format=PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED;
    output.data=packet->reference.bits();output.capacity=quint64(packet->reference.sizeInBytes());output.stride=quint64(packet->reference.bytesPerLine());
    ok(paint_session_read_frame(core,session,0,frame.request_id,frame.frame_id,&output));
    for(qsizetype i=0;i<packet->image.sizeInBytes();++i)packet->maxError=std::max(packet->maxError,
        std::abs(int(packet->image.constBits()[i])-int(packet->reference.constBits()[i])));
    check(packet->maxError<=1,"transport interpretation differs from CPU renderer");
    const auto after=state();check(after.revision==before.revision && after.undo_depth==before.undo_depth && after.completed_sequence==before.completed_sequence,"transport mutated document/history/input sequence");
    ok(paint_session_release_composition(core,session,lease));lease=0;
    packet->usage=dto<PaintCompositionUsage>();ok(paint_core_composition_usage(core,&packet->usage));
    check(packet->usage.plan_bytes==0 && packet->usage.read_bytes==0,"transport resources retained after release");
    return packet;
}
class PacketReader final : public QThread {
public:
    QByteArray scratch;std::shared_ptr<const Packet> packet;QString error;
    void run() override {try {packet=readPacket(scratch);}catch(const std::exception &e){error=QString::fromUtf8(e.what());}}
};
class PacketItem final : public QQuickItem {
public:
    QImage image;std::atomic<int> adopted{0};
    explicit PacketItem(QQuickItem *parent):QQuickItem(parent){setFlag(ItemHasContents);}
    QSGNode *updatePaintNode(QSGNode *old,UpdatePaintNodeData*) override {
        auto *node=static_cast<QSGSimpleTextureNode*>(old);
        if(!node){node=new QSGSimpleTextureNode;node->setTexture(window()->createTextureFromImage(image));node->setOwnsTexture(true);node->setFiltering(QSGTexture::Nearest);}
        node->setRect(boundingRect());++adopted;return node;
    }
};
}
class CompositionTransportTests final : public QObject {
    Q_OBJECT
private slots:
    void workerPlanTileReadAndRealWindow() {
        QTemporaryDir temp;PacketReader reader;reader.scratch=QDir::toNativeSeparators(temp.path()).toUtf8();
        int heartbeats=0;QTimer heartbeat;heartbeat.setInterval(8);connect(&heartbeat,&QTimer::timeout,[&]{++heartbeats;});heartbeat.start();
        reader.start();QTRY_VERIFY_WITH_TIMEOUT(reader.isFinished(),30000);
        QVERIFY2(reader.error.isEmpty(),qPrintable(reader.error));QVERIFY(reader.packet);QVERIFY(heartbeats>0);
        const auto &packet=*reader.packet;QVERIFY(packet.readerThread!=quintptr(QThread::currentThreadId()));
        QQuickWindow window;window.resize(256,192);window.setColor(Qt::black);
        auto *item=new PacketItem(window.contentItem());item->image=packet.image;item->setSize({256,192});
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));QTRY_VERIFY(item->adopted.load()>0);
        const auto api=window.rendererInterface()->graphicsApi();
        QVERIFY(api==QSGRendererInterface::Software || api==QSGRendererInterface::Direct3D11);
        const auto image=window.grabWindow().convertToFormat(QImage::Format_RGBA8888);
        QVERIFY(!image.isNull());int maxWindow=0;
        for(int y=0;y<image.height();++y)for(int x=0;x<image.width();++x) {
            const auto actual=image.pixelColor(x,y);const auto expected=packet.image.pixelColor(x*128/image.width(),y*96/image.height());
            maxWindow=std::max({maxWindow,std::abs(actual.red()-expected.red()),std::abs(actual.green()-expected.green()),std::abs(actual.blue()-expected.blue())});
        }
        QVERIFY(maxWindow<=1);
        qInfo()<<"G1 CPU transport reference"<<"API"<<int(api)<<"DPR"<<window.devicePixelRatio()
            <<"nodes"<<packet.info.node_count<<"tiles"<<packet.info.tile_count<<"linear tile bytes"<<packet.info.pinned_bytes
            <<"budget charge"<<packet.info.budget_bytes<<"peak staging"<<packet.usage.peak_read_bytes
            <<"max packet/CPU error"<<packet.maxError<<"max window error"<<maxWindow;
        const auto path=qEnvironmentVariable("DRAWVERSE_G1_REPORT");
        if(!path.isEmpty()) {
            QVERIFY(image.save(path+".png"));QVERIFY(packet.reference.save(path+".cpu.png"));
            QFile file(path);QVERIFY(file.open(QIODevice::WriteOnly));
            file.write(QJsonDocument(QJsonObject{{"passed",true},{"api",int(api)},{"qt",qVersion()},
                {"dpr",window.devicePixelRatio()},{"nodes",int(packet.info.node_count)},{"tiles",int(packet.info.tile_count)},
                {"pinned_bytes",double(packet.info.pinned_bytes)},{"budget_bytes",double(packet.info.budget_bytes)},
                {"max_cpu_error",packet.maxError},{"max_window_error",maxWindow},{"gui_heartbeats",heartbeats},
                {"remaining_plan_bytes",double(packet.usage.plan_bytes)},{"remaining_read_bytes",double(packet.usage.read_bytes)},
                {"peak_read_bytes",double(packet.usage.peak_read_bytes)},{"gpu_composition",false}}).toJson());
        }
    }
};
QTEST_MAIN(CompositionTransportTests)
#include "composition_transport_tests.moc"
