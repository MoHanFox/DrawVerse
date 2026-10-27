#include "CompositionNode.h"
#include "platform/InteractionTrace.h"
#include <QQuickWindow>
#include <QFile>
#include <QSGRendererInterface>
#include <QtEndian>
#include <QSet>
#include <array>
#include <algorithm>
#include <cmath>
#include <cstring>
#include <map>
#include <tuple>
#ifdef DRAWVERSE_HAS_COMPOSITION_RHI
#include <rhi/qrhi.h>
#ifdef Q_OS_WIN
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <d3d11.h>
#endif
static void initializeCompositionResources(){Q_INIT_RESOURCE(composition_shaders);Q_INIT_RESOURCE(composition_precise_shaders);}

namespace {
constexpr quint64 textureLimit=256ull*1024*1024;
constexpr quint64 processTextureLimit=2ull*1024*1024*1024;
std::atomic<quint64> reservedTextureBytes{0};
// Reserve payload for the current allocation plus every possible deferred frame.
// This is deliberately conservative; driver overhead is not a payload byte count.
class TextureReservation final {
public:
    ~TextureReservation(){release();}
    bool grow(quint64 value){
        if(value<=bytes)return true;
        const auto extra=value-bytes;auto current=reservedTextureBytes.load();
        do{if(extra>processTextureLimit-current)return false;}
        while(!reservedTextureBytes.compare_exchange_weak(current,current+extra));
        bytes=value;return true;
    }
    void release(){if(!released.exchange(true))reservedTextureBytes.fetch_sub(bytes);}
    void retire(QQuickWindow *window,int frames,const std::shared_ptr<TextureReservation> &self){
        const auto done=[self]{self->release();for(const auto &c:self->connections)QObject::disconnect(c);};
        connections[0]=QObject::connect(window,&QQuickWindow::beforeRendering,window,[self,frames,done]()mutable{if(--frames<=0)done();},Qt::DirectConnection);
        connections[1]=QObject::connect(window,&QQuickWindow::sceneGraphInvalidated,window,done,Qt::DirectConnection);
        connections[2]=QObject::connect(window,&QObject::destroyed,done);
    }
private:
    quint64 bytes=0;
    std::atomic<bool> released{false};
    std::array<QMetaObject::Connection,3> connections;
};
struct Parameters {
    std::array<qint32,4> dimensions{},source{},mask{},sourceOrigin{},maskOrigin{};
    std::array<float,4> factors{},transformBounds{},transformInfo{};
    std::array<double,4> inverse0{},inverse1{},inverse2{};
};
static_assert(sizeof(Parameters)==224);
static_assert(offsetof(Parameters,inverse0)==128 && offsetof(Parameters,inverse1)==160 && offsetof(Parameters,inverse2)==192);
class DirectTexture final : public QSGTexture {
public:
    QRhiTexture *value=nullptr;
    qint64 comparisonKey()const override{return qint64(quintptr(value));}
    QRhiTexture *rhiTexture()const override{return value;}
    QSize textureSize()const override{return value?value->pixelSize():QSize();}
    bool hasAlphaChannel()const override{return true;}
    bool hasMipmaps()const override{return false;}
};
struct Target {
    std::unique_ptr<QRhiTexture> texture;
    std::unique_ptr<QRhiRenderPassDescriptor> pass;
    std::unique_ptr<QRhiTextureRenderTarget> target;
};
struct Pass {
    std::unique_ptr<QRhiBuffer> uniform;
    std::unique_ptr<QRhiShaderResourceBindings> bindings;
    QRhiGraphicsPipeline *pipeline=nullptr;
    Parameters parameters;
    int output=0;
    bool mesh=false,precise=false;
};
struct PageMap {
    std::unique_ptr<QRhiTexture> texture;
    QByteArray bytes;
    QSize size;
    QPoint origin;
};
class RhiCompositionNode final : public CompositionNode {
public:
    RhiCompositionNode(QQuickWindow *w,CompositionPacketPtr p,std::shared_ptr<CompositionCounters> c,std::function<void(QString)> f,std::function<void(CompositionPacketPtr)> a)
        :window(w),packet(std::move(p)),counters(std::move(c)),fallback(std::move(f)),adoption(std::move(a)){}
    ~RhiCompositionNode()override{
        QObject::disconnect(connection);
        QObject::disconnect(adoptionConnection);
        if(reservation)reservation->retire(window,inFlightFrames+2,reservation);
    }
    bool initialize(){
        if(qEnvironmentVariable("DRAWVERSE_COMPOSITION_FAIL")=="init")return fail("CPU: injected composition initialization failure");
        auto *ri=window->rendererInterface();
        if(ri->graphicsApi()!=QSGRendererInterface::Direct3D11)return fail("CPU: composition requires D3D11");
        rhi=static_cast<QRhi*>(ri->getResource(window,QSGRendererInterface::RhiResource));
        if(!rhi || !rhi->isTextureFormatSupported(QRhiTexture::RGBA32F,QRhiTexture::RenderTarget) ||
           rhi->resourceLimit(QRhi::TextureSizeMax)<4096)return fail("CPU: composition format/extent capability");
        const auto source=[](const char *name){QFile file(name);if(!file.open(QIODevice::ReadOnly))return QShader{};return QShader::fromSerialized(file.readAll());};
        vertex=source(":/composition/composition.vert.qsb");fragment=source(":/composition/composition.frag.qsb");meshVertex=source(":/composition/composition_transform.vert.qsb");
        if(!vertex.isValid() || !fragment.isValid() || !meshVertex.isValid())return fail("CPU: composition shaders unavailable");
        sampler.reset(rhi->newSampler(QRhiSampler::Nearest,QRhiSampler::Nearest,QRhiSampler::None,QRhiSampler::ClampToEdge,QRhiSampler::ClampToEdge));
        if(!sampler->create())return fail("CPU: composition sampler allocation");
        bool doubles=false;
#ifdef Q_OS_WIN
        auto *device=static_cast<ID3D11Device*>(ri->getResource(window,QSGRendererInterface::DeviceResource));
        D3D11_FEATURE_DATA_DOUBLES basic{};D3D11_FEATURE_DATA_D3D11_OPTIONS options{};
        doubles=device && SUCCEEDED(device->CheckFeatureSupport(D3D11_FEATURE_DOUBLES,&basic,sizeof(basic))) && basic.DoublePrecisionFloatShaderOps &&
            SUCCEEDED(device->CheckFeatureSupport(D3D11_FEATURE_D3D11_OPTIONS,&options,sizeof(options))) && options.ExtendedDoublesShaderInstructions;
#endif
        preciseFragment=source(":/composition/composition_precise.frag.qsb");warpFragment=source(":/composition/composition_warp.frag.qsb");
        counters->preciseTransform=doubles && qEnvironmentVariable("DRAWVERSE_COMPOSITION_FAIL")!="precision" && preciseFragment.isValid() && warpFragment.isValid()?1:-1;
        const int frameSlots=rhi->resourceLimit(QRhi::FramesInFlight);
        if(frameSlots<1 || frameSlots>8)return fail("CPU: composition in-flight budget");
        inFlightFrames=frameSlots;reservation=std::make_shared<TextureReservation>();
        if(!reserveTargets(packet))return false;
        atlas.reset(rhi->newTexture(QRhiTexture::RGBA32F,{2048,2048}));
        if(!atlas->create())return fail("CPU: composition atlas allocation");
        staging.resize(frameSlots);
        connection=QObject::connect(window,&QQuickWindow::beforeRendering,window,[this]{record();},Qt::DirectConnection);
        adoptionConnection=QObject::connect(window,&QQuickWindow::afterRendering,window,[this]{
            const auto wanted=counters->handoffRevision.load();
            if(wanted && rendered && !failed && !rect().isEmpty() && renderedPacket==packet && !packet->preview &&
               packet->info.key.revision>=wanted && adoptedPacket!=packet){adoptedPacket=packet;adoption(packet);}
        },Qt::DirectConnection);
        auto p=std::move(packet);if(!sync(std::move(p)))return false;
        // Layout-only bindings and compatible pass descriptors outlive every
        // per-frame binding. Compile once while preparing the original source,
        // before the first edit, rather than creating pipelines on each Move.
        pipelineLayout.reset(rhi->newShaderResourceBindings());pipelineLayout->setBindings({
            QRhiShaderResourceBinding::uniformBuffer(0,QRhiShaderResourceBinding::VertexStage|QRhiShaderResourceBinding::FragmentStage,nullptr,0,sizeof(Parameters)),
            QRhiShaderResourceBinding::sampledTexture(1,QRhiShaderResourceBinding::FragmentStage,nullptr,nullptr),
            QRhiShaderResourceBinding::sampledTexture(2,QRhiShaderResourceBinding::FragmentStage,nullptr,nullptr),
            QRhiShaderResourceBinding::sampledTexture(3,QRhiShaderResourceBinding::FragmentStage,nullptr,nullptr),
            QRhiShaderResourceBinding::sampledTexture(4,QRhiShaderResourceBinding::FragmentStage,nullptr,nullptr)});
        if(!pipelineLayout->create())return fail("CPU: composition pipeline layout allocation");
        pipelinePasses[0].reset(linear[0].pass->newCompatibleRenderPassDescriptor());
        pipelinePasses[1].reset(display.pass->newCompatibleRenderPassDescriptor());
        if(!pipeline(0) || !pipeline(1))return fail("CPU: composition pipeline allocation");
        if(counters->preciseTransform.load()==1 && (!pipeline(2) || !pipeline(3))){
            counters->preciseTransform=-1;pipelines[2].reset();pipelines[3].reset();
        }
        return true;
    }
    bool sync(CompositionPacketPtr p)override{
        if(failed || !p || p->info.key.device_epoch!=(packet?packet->info.key.device_epoch:p->info.key.device_epoch))return false;
        if(packet && p->info.key.document_generation==packet->info.key.document_generation &&
           p->info.key.request_id<packet->info.key.request_id)return true;
        if(packet==p)return true;
        updateDirty(p);packet=std::move(p);dirty=true;
        counters->epoch=packet->info.key.device_epoch;counters->request=packet->info.key.request_id;
        return prepareTargets();
    }
    bool ready()const override{return rendered && !failed;}
private:
    bool fail(const char *message){if(!failed){failed=true;++counters->failures;fallback(QString::fromUtf8(message));}return false;}
    bool createTarget(Target &t,QRhiTexture::Format format,QSize size){
        t.texture.reset(rhi->newTexture(format,size,1,QRhiTexture::RenderTarget|QRhiTexture::UsedAsTransferSource));
        if(!t.texture->create())return false;
        t.target.reset(rhi->newTextureRenderTarget(QRhiTextureRenderTargetDescription(QRhiColorAttachment(t.texture.get()))));
        t.target->setFlags(QRhiTextureRenderTarget::PreserveColorContents);
        t.pass.reset(t.target->newCompatibleRenderPassDescriptor());t.target->setRenderPassDescriptor(t.pass.get());
        return t.target->create();
    }
    bool reserveTargets(const CompositionPacketPtr &p){
        const quint64 surfaces=quint64(p->surfaceCount)+(scratch.texture || (p->transform && p->transform->geometry.warp)?1:0);
        const quint64 charge=64ull*1024*1024+quint64(p->info.document_width)*p->info.document_height*16*surfaces+
            quint64(p->pixels.width())*p->pixels.height()*4+4ull*1024*1024;
        if(charge>textureLimit)return fail("CPU: composition GPU payload budget");
        if(!reservation->grow(charge*quint64(inFlightFrames+2)))return fail("CPU: composition process/in-flight GPU payload budget");
        counters->textureBytes=charge;return true;
    }
    bool prepareTargets(){
        const QSize doc(int(packet->info.document_width),int(packet->info.document_height));
        const auto pixels=packet->pixels;
        if(packet->transform && counters->preciseTransform.load()!=1)return fail("CPU: precise transform device capability");
        if(packet->transform && packet->transform->geometry.warp && !scratch.texture){
            if(!reserveTargets(packet) || !createTarget(scratch,QRhiTexture::RGBA32F,doc))return fail("CPU: composition transform target budget/allocation");
        }
        const bool topology=groupTargets.size()!=size_t(packet->surfaceCount-2);
        if(doc==documentSize && pixels==outputSize && !topology)return true;
        // Resources are owned by this render node. QRhi defers native destruction
        // as required; no cross-window texture registry participates.
        passes.clear();completePasses=false;maps.clear();axisX.reset();axisY.reset();
        if(!reserveTargets(packet))return false;
        if(doc!=documentSize || topology){
            groupTargets.clear();groupTargets.resize(size_t(packet->surfaceCount-2));
            for(auto &t:groupTargets)if(!createTarget(t,QRhiTexture::RGBA32F,doc))return fail("CPU: composition group target allocation");
            changed=QRect(QPoint(),doc);
        }
        if(doc!=documentSize){
            for(auto &t:linear)t={};
            if(scratch.texture){scratch={};if(!createTarget(scratch,QRhiTexture::RGBA32F,doc))return fail("CPU: composition transform target allocation");}
            if(!createTarget(linear[0],QRhiTexture::RGBA32F,doc) || !createTarget(linear[1],QRhiTexture::RGBA32F,doc))return fail("CPU: composition render-target allocation");
            changed=QRect(QPoint(),doc);
        }
        display={};if(!createTarget(display,QRhiTexture::RGBA8,pixels))return fail("CPU: composition display-target allocation");
        documentSize=doc;outputSize=pixels;texture.value=display.texture.get();setTexture(&texture);setFiltering(QSGTexture::Linear);
        markDirty(QSGNode::DirtyMaterial);rendered=false;
        return true;
    }
    void updateDirty(const CompositionPacketPtr &next){
        const QRect full(0,0,int(next->info.document_width),int(next->info.document_height));
        if(!packet || next->info.key.document_generation!=packet->info.key.document_generation ||
           next->info.document_width!=packet->info.document_width || next->info.document_height!=packet->info.document_height ||
           next->nodes.size()!=packet->nodes.size() ||
           next->operations.size()!=packet->operations.size() || next->surfaceCount!=packet->surfaceCount || next->outputSurface!=packet->outputSurface ||
           (!next->operations.isEmpty() && std::memcmp(next->operations.constData(),packet->operations.constData(),size_t(next->operations.size())*sizeof(PaintCompositionOperation))) ||
           (next->geometrySequence!=packet->geometrySequence && (next->transform || packet->transform))){changed=full;return;}
        for(int i=0;i<next->nodes.size();++i){
            auto a=next->nodes[i],b=packet->nodes[i];a.first_tile=a.tile_count=b.first_tile=b.tile_count=0;
            if(std::memcmp(&a,&b,sizeof(a))){changed=full;return;}
        }
        using Key=std::tuple<int,qint32,qint32>;
        using Entry=std::pair<quint64,QRect>;
        const auto entries=[&](const CompositionPacket &p){
            std::map<Key,Entry> result;
            for(int n=0;n<p.nodes.size();++n){
                const auto &node=p.nodes[n];
                for(quint32 i=node.first_tile;i<node.first_tile+node.tile_count;++i){
                    const auto &tile=p.tiles[int(i)];
                    const qint64 left=qint64(tile.tile_x)*64+node.offset_x,top=qint64(tile.tile_y)*64+node.offset_y;
                    const qint64 x0=std::max<qint64>(0,left),y0=std::max<qint64>(0,top);
                    const qint64 x1=std::min<qint64>(full.width(),left+64),y1=std::min<qint64>(full.height(),top+64);
                    result.emplace(Key{n,tile.tile_x,tile.tile_y},Entry{tile.content_id,x0<x1 && y0<y1?QRect(int(x0),int(y0),int(x1-x0),int(y1-y0)):QRect()});
                }
            }
            return result;
        };
        const auto before=entries(*packet),after=entries(*next);changed={};
        for(const auto &[key,value]:before){const auto at=after.find(key);if(at==after.end() || at->second.first!=value.first)changed=changed.united(value.second);}
        for(const auto &[key,value]:after){const auto at=before.find(key);if(at==before.end() || at->second.first!=value.first)changed=changed.united(value.second);}
    }
    bool map(PageMap &out,const PaintCompositionNode &node,const QHash<quint64,int> &locations,QRect sourceRect={}){
        const int ox=node.offset_x,oy=node.offset_y;
        if(std::abs(qint64(ox))>1048576 || std::abs(qint64(oy))>1048576 || node.white_width>INT_MAX || node.white_height>INT_MAX)
            return fail("CPU: composition coordinate range");
        const auto floor64=[](int x){return x>=0?x/64:(x-63)/64;};
        if(sourceRect.isEmpty())sourceRect=QRect(QPoint(),documentSize);
        out.origin={floor64(sourceRect.left()-ox),floor64(sourceRect.top()-oy)};
        const int w=floor64(sourceRect.right()-ox)-out.origin.x()+1;
        const int h=floor64(sourceRect.bottom()-oy)-out.origin.y()+1;
        if(w<1 || h<1 || w>67 || h>67)return fail("CPU: composition transform source table budget");
        out.size={w,h};out.bytes=QByteArray(w*h*16,'\0');
        for(quint32 i=node.first_tile;i<node.first_tile+node.tile_count;++i){
            if(i>=quint32(packet->tiles.size()))return fail("CPU: composition tile range");
            const auto &tile=packet->tiles[int(i)];const int x=tile.tile_x-out.origin.x(),y=tile.tile_y-out.origin.y();
            if(x<0 || y<0 || x>=w || y>=h)continue;
            const float slot=float(locations.value(tile.content_id,-1)+1);
            std::memcpy(out.bytes.data()+(y*w+x)*16,&slot,4);
        }
        out.texture.reset(rhi->newTexture(QRhiTexture::RGBA32F,out.size));if(!out.texture->create())return false;
        ++counters->pageTableCreates;return true;
    }
    QRhiGraphicsPipeline *pipeline(int kind){
        if(pipelines[size_t(kind)])return pipelines[size_t(kind)].get();
        InteractionTrace::Scope trace("composition.pipeline.create");
        auto value=std::unique_ptr<QRhiGraphicsPipeline>(rhi->newGraphicsPipeline());
        value->setFlags(QRhiGraphicsPipeline::UsesScissor);
        value->setShaderStages({{QRhiShaderStage::Vertex,kind==3?meshVertex:vertex},{QRhiShaderStage::Fragment,kind==3?warpFragment:kind==2?preciseFragment:fragment}});
        if(kind==3){
            QRhiVertexInputLayout input;input.setBindings({QRhiVertexInputBinding(48,QRhiVertexInputBinding::PerInstance)});
            input.setAttributes({QRhiVertexInputAttribute(0,0,QRhiVertexInputAttribute::Float4,0),QRhiVertexInputAttribute(0,1,QRhiVertexInputAttribute::Float4,16),QRhiVertexInputAttribute(0,2,QRhiVertexInputAttribute::Float4,32)});
            value->setVertexInputLayout(input);
        }
        value->setShaderResourceBindings(pipelineLayout.get());value->setRenderPassDescriptor(pipelinePasses[size_t(kind==1)].get());
        if(!value->create())return nullptr;
        ++counters->pipelineCreates;pipelines[size_t(kind)]=std::move(value);return pipelines[size_t(kind)].get();
    }
    bool makePass(Pass &pass,Target &target,QRhiTexture *back,QRhiTexture *a,QRhiTexture *b,QRhiTexture *c){
        pass.uniform.reset(rhi->newBuffer(QRhiBuffer::Dynamic,QRhiBuffer::UniformBuffer,sizeof(Parameters)));
        if(!pass.uniform->create())return false;
        ++counters->parameterBufferCreates;
        pass.bindings.reset(rhi->newShaderResourceBindings());pass.bindings->setBindings({
            QRhiShaderResourceBinding::uniformBuffer(0,QRhiShaderResourceBinding::VertexStage|QRhiShaderResourceBinding::FragmentStage,pass.uniform.get()),
            QRhiShaderResourceBinding::sampledTexture(1,QRhiShaderResourceBinding::FragmentStage,back,sampler.get()),
            QRhiShaderResourceBinding::sampledTexture(2,QRhiShaderResourceBinding::FragmentStage,a,sampler.get()),
            QRhiShaderResourceBinding::sampledTexture(3,QRhiShaderResourceBinding::FragmentStage,b,sampler.get()),
            QRhiShaderResourceBinding::sampledTexture(4,QRhiShaderResourceBinding::FragmentStage,c,sampler.get())});
        if(!pass.bindings->create())return false;
        ++counters->bindingCreates;
        pass.pipeline=pipeline(pass.mesh?3:pass.precise?2:target.texture->format()==QRhiTexture::RGBA8?1:0);
        return pass.pipeline!=nullptr;
    }
    std::unique_ptr<QRhiTexture> axis(int count,double start,double span,int limit,QByteArray &bytes){
        bytes=QByteArray(count*16,'\0');
        for(int i=0;i<count;++i){const double p=start+(double(i)+.5)*span/count;const float value=p<0 || p>=limit?-1.f:float(std::floor(p));std::memcpy(bytes.data()+i*16,&value,4);}
        std::unique_ptr<QRhiTexture> result(rhi->newTexture(QRhiTexture::RGBA32F,{count,1}));
        if(!result->create())return {};++counters->axisCreates;return result;
    }
    static void transformParameters(Parameters &p,const CompositionTransform &t){
        const auto b=t.sourceBounds;
        p.transformBounds={float(b.x()),float(b.y()),float(b.width()),float(b.height())};
        p.transformInfo={1,float(t.geometry.warp),float(t.geometry.interpolation==5?(t.scale<1?4:3):t.geometry.interpolation),0};
        p.inverse0={t.display.inverse[0],t.display.inverse[1],t.display.inverse[2],0};
        p.inverse1={t.display.inverse[3],t.display.inverse[4],t.display.inverse[5],0};
        p.inverse2={t.display.inverse[6],t.display.inverse[7],t.display.inverse[8],0};
    }
    bool reusableTransformPasses()const{
        const auto &before=renderedPacket;
        if(!before || !completePasses || passes.empty() || !packet->transform || !before->transform ||
           packet->info.key.document_generation!=before->info.key.document_generation ||
           packet->info.key.device_epoch!=before->info.key.device_epoch || packet->info.key.revision!=before->info.key.revision ||
           packet->info.document_width!=before->info.document_width || packet->info.document_height!=before->info.document_height ||
           packet->region!=before->region || packet->pixels!=before->pixels || packet->lod!=before->lod ||
           packet->transform->node!=before->transform->node || packet->transform->sourceBounds!=before->transform->sourceBounds ||
           packet->transform->geometry.warp!=before->transform->geometry.warp ||
           packet->nodes.size()!=before->nodes.size() || packet->tiles.size()!=before->tiles.size())return false;
        // Only local transform geometry/publication may differ. A changed source, tile slot
        // dependency, viewport or pass topology takes the complete preparation
        // path. Equality is checked against the packet these live passes used.
        return (packet->nodes.isEmpty() || !std::memcmp(packet->nodes.constData(),before->nodes.constData(),size_t(packet->nodes.size())*sizeof(PaintCompositionNode))) &&
            (packet->tiles.isEmpty() || !std::memcmp(packet->tiles.constData(),before->tiles.constData(),size_t(packet->tiles.size())*sizeof(PaintCompositionTile)));
    }
    bool prepareMesh(QRhiResourceUpdateBatch *batch){
        if(!packet->transform || !packet->transform->geometry.warp)return true;
        const auto &t=*packet->transform;
        if(!meshVertices){meshVertices.reset(rhi->newBuffer(QRhiBuffer::Dynamic,QRhiBuffer::VertexBuffer,144*144*2*48));
            if(!meshVertices->create())return fail("CPU: composition transform buffer allocation");}
        const int count=(int(t.display.columns)-1)*(int(t.display.rows)-1)*2;
        QByteArray triangles(count*48,Qt::Uninitialized);auto *out=triangles.data();
        const auto *source=reinterpret_cast<const float*>(t.vertices.constData());quint64 work=0;
        for(quint32 y=0;y+1<t.display.rows;++y)for(quint32 x=0;x+1<t.display.columns;++x){
            const quint32 a=y*t.display.columns+x,b=a+1,c=a+t.display.columns,d=c+1;
            for(const auto indices:std::array<std::array<quint32,3>,2>{{{a,b,d},{a,d,c}}}){
                float minX=INFINITY,minY=INFINITY,maxX=-INFINITY,maxY=-INFINITY;
                for(auto index:indices){const auto *vertexData=source+index*4;std::memcpy(out,vertexData,16);out+=16;
                    minX=std::min(minX,vertexData[0]);minY=std::min(minY,vertexData[1]);maxX=std::max(maxX,vertexData[0]);maxY=std::max(maxY,vertexData[1]);}
                const int w=std::max(0,std::min(documentSize.width(),int(std::ceil(maxX)))-std::max(0,int(std::floor(minX))));
                const int h=std::max(0,std::min(documentSize.height(),int(std::ceil(maxY)))-std::max(0,int(std::floor(minY))));
                work+=quint64(w)*h;if(work>33554432)return fail("CPU: composition transform triangle work budget");
            }
        }
        meshTriangleCount=count;batch->updateDynamicBuffer(meshVertices.get(),0,triangles.size(),triangles.constData());return true;
    }
    bool prepare(QRhiResourceUpdateBatch *batch){
        InteractionTrace::Scope trace("composition.prepare");
        if(!prepareTargets())return false;
        if(reusableTransformPasses()){
            if(packet->geometrySequence!=renderedPacket->geometrySequence){
                if(!prepareMesh(batch))return false;
                for(auto &pass:passes)if(pass->precise){
                    transformParameters(pass->parameters,*packet->transform);
                    batch->updateDynamicBuffer(pass->uniform.get(),0,sizeof(Parameters),&pass->parameters);
                }
            }
            ++counters->geometryReuses;return true;
        }
        passes.clear();completePasses=!changed.isEmpty();maps.clear();
        QHash<quint64,int> locations;
        // Retain every source needed by this packet before choosing eviction slots.
        for(const auto &tile:packet->tiles)if(cache.contains(tile.content_id))locations.insert(tile.content_id,cache.value(tile.content_id));
        QSet<int> occupied;for(auto slot:locations)occupied.insert(slot);
        int next=0;quint64 bytes=0,uploads=0;
        const int frame=rhi->currentFrameSlot();
        if(frame<0 || frame>=staging.size())return fail("CPU: composition frame slot");
        staging[frame].clear();
        for(int i=0;i<packet->tiles.size();++i){
            const auto id=packet->tiles[i].content_id;if(locations.contains(id))continue;
            while(occupied.contains(next))++next;
            if(next>=1024)return fail("CPU: composition cache full");
            const auto previous=slotIds[size_t(next)];if(previous)cache.remove(previous);
            cache.insert(id,next);slotIds[size_t(next)]=id;locations.insert(id,next);occupied.insert(next);
            QRhiTextureSubresourceUploadDescription upload(packet->data[i]->bytes);upload.setDestinationTopLeft({(next%32)*64,(next/32)*64});upload.setSourceSize({64,64});
            batch->uploadTexture(atlas.get(),QRhiTextureUploadDescription({QRhiTextureUploadEntry(0,0,upload)}));
            staging[frame].append(packet->data[i]);++uploads;bytes+=65536;
        }
        if(uploads)counters->lastUploadNs=InteractionTrace::now();
        counters->uploads.fetch_add(uploads);counters->uploadBytes.fetch_add(bytes);
        maps.resize(size_t(packet->nodes.size()));
        for(int i=0;i<packet->nodes.size();++i){
            const QRect sourceRect=packet->transform && packet->transform->node==i?packet->transform->sourceBounds.toAlignedRect().adjusted(-2,-2,2,2):QRect();
            if(!map(maps[size_t(i)],packet->nodes[i],locations,sourceRect))return fail("CPU: composition page table allocation");
            batch->uploadTexture(maps[size_t(i)].texture.get(),QRhiTextureUploadDescription({QRhiTextureUploadEntry(0,0,QRhiTextureSubresourceUploadDescription(maps[size_t(i)].bytes))}));
        }
        if(maps.empty())return fail("CPU: composition empty plan");
        if(!prepareMesh(batch))return false;
        QByteArray xs,ys;axisX=axis(outputSize.width(),packet->region.x(),packet->region.width(),documentSize.width(),xs);
        axisY=axis(outputSize.height(),packet->region.y(),packet->region.height(),documentSize.height(),ys);
        if(!axisX || !axisY)return fail("CPU: composition axis allocation");
        batch->uploadTexture(axisX.get(),QRhiTextureUploadDescription({QRhiTextureUploadEntry(0,0,QRhiTextureSubresourceUploadDescription(xs))}));
        batch->uploadTexture(axisY.get(),QRhiTextureUploadDescription({QRhiTextureUploadEntry(0,0,QRhiTextureSubresourceUploadDescription(ys))}));
        int current=0;
        if(!packet->operations.isEmpty()){
            if(packet->transform)return fail("CPU: composition group transform closure");
            if(!changed.isEmpty())for(const auto &op:packet->operations){
                const auto &node=packet->nodes[int(op.node_index)];
                const int mi=node.mask_index<quint32(packet->nodes.size())?int(node.mask_index):int(op.node_index);
                const auto &mask=packet->nodes[mi];const auto &srcMap=maps[size_t(op.node_index)],&maskMap=maps[size_t(mi)];
                auto pass=std::make_unique<Pass>();auto &p=pass->parameters;
                pass->output=op.output<2?int(op.output):int(op.output)+2;
                p.dimensions={documentSize.width(),documentSize.height(),outputSize.width(),outputSize.height()};
                p.source={node.offset_x,node.offset_y,qint32(node.white_width),qint32(node.white_height)};
                p.mask={mask.offset_x,mask.offset_y,qint32(mask.white_width),qint32(mask.white_height)};
                p.sourceOrigin={srcMap.origin.x(),srcMap.origin.y(),qint32(quint32(node.reserved)),0};
                p.maskOrigin={maskMap.origin.x(),maskMap.origin.y(),0,0};
                float kind=0;
                if(op.kind==PAINT_COMPOSITION_CLEAR)kind=2;
                else if(op.kind==PAINT_COMPOSITION_COPY)kind=5;
                else if(op.kind==PAINT_COMPOSITION_BLEND_SURFACE)kind=6;
                else if(op.kind==PAINT_COMPOSITION_INFLUENCE)kind=7;
                p.factors={node.opacity*node.fill,mi!=int(op.node_index) && mask.visible?mask.opacity:0.f,0,kind};
                auto *source=op.kind>=PAINT_COMPOSITION_BLEND_SURFACE?surface(int(op.source)).texture.get():srcMap.texture.get();
                if(!makePass(*pass,surface(int(op.output)),surface(int(op.backdrop)).texture.get(),atlas.get(),source,maskMap.texture.get()))return fail("CPU: composition group pipeline allocation");
                batch->updateDynamicBuffer(pass->uniform.get(),0,sizeof(p),&p);passes.push_back(std::move(pass));
            }
            current=packet->outputSurface;
        }else{
        if(!changed.isEmpty()){
            auto clear=std::make_unique<Pass>();clear->output=0;auto &p=clear->parameters;
            p.dimensions={documentSize.width(),documentSize.height(),outputSize.width(),outputSize.height()};p.factors={0,0,0,2};
            if(!makePass(*clear,linear[0],linear[1].texture.get(),atlas.get(),maps[0].texture.get(),maps[0].texture.get()))return fail("CPU: composition clear pipeline allocation");
            batch->updateDynamicBuffer(clear->uniform.get(),0,sizeof(p),&p);passes.push_back(std::move(clear));
        }
        for(int i=0;i<packet->nodes.size();++i){
            const auto &node=packet->nodes[i];if(node.kind!=0 || !node.visible)continue;
            if(changed.isEmpty()){current=1-current;continue;}
            const int mi=node.mask_index<quint32(packet->nodes.size())?int(node.mask_index):i;
            const auto &mask=packet->nodes[mi];
            const auto &srcMap=maps[size_t(i)],&maskMap=maps[size_t(mi)];
            auto pass=std::make_unique<Pass>();auto &p=pass->parameters;
            pass->output=1-current;
            p.dimensions={documentSize.width(),documentSize.height(),outputSize.width(),outputSize.height()};
            p.source={node.offset_x,node.offset_y,qint32(node.white_width),qint32(node.white_height)};
            p.mask={mask.offset_x,mask.offset_y,qint32(mask.white_width),qint32(mask.white_height)};
            p.sourceOrigin={srcMap.origin.x(),srcMap.origin.y(),qint32(quint32(node.reserved)),0};p.maskOrigin={maskMap.origin.x(),maskMap.origin.y(),0,0};
            p.factors={node.opacity*node.fill,mi!=i && mask.visible?mask.opacity:0.f,0,0};
            if(packet->transform && packet->transform->node==i){
                const auto &t=*packet->transform;pass->precise=true;transformParameters(p,t);
                if(t.geometry.warp){
                    auto clear=std::make_unique<Pass>();clear->output=3;clear->parameters=p;clear->parameters.factors[3]=2;
                    if(!makePass(*clear,scratch,linear[current].texture.get(),atlas.get(),srcMap.texture.get(),maskMap.texture.get()))return fail("CPU: composition transform clear pipeline");
                    batch->updateDynamicBuffer(clear->uniform.get(),0,sizeof(p),&clear->parameters);passes.push_back(std::move(clear));
                    p.factors[3]=3;pass->mesh=true;pass->output=3;
                    if(!makePass(*pass,scratch,linear[current].texture.get(),atlas.get(),srcMap.texture.get(),maskMap.texture.get()))return fail("CPU: composition transform mesh pipeline");
                    batch->updateDynamicBuffer(pass->uniform.get(),0,sizeof(p),&p);passes.push_back(std::move(pass));
                    pass=std::make_unique<Pass>();pass->output=1-current;pass->parameters.dimensions={documentSize.width(),documentSize.height(),outputSize.width(),outputSize.height()};pass->parameters.factors[3]=4;
                    if(!makePass(*pass,linear[1-current],linear[current].texture.get(),scratch.texture.get(),srcMap.texture.get(),maskMap.texture.get()))return fail("CPU: composition transform over pipeline");
                    batch->updateDynamicBuffer(pass->uniform.get(),0,sizeof(Parameters),&pass->parameters);passes.push_back(std::move(pass));current=1-current;continue;
                }
            }
            if(!makePass(*pass,linear[1-current],linear[current].texture.get(),atlas.get(),srcMap.texture.get(),maskMap.texture.get()))return fail("CPU: composition pipeline allocation");
            batch->updateDynamicBuffer(pass->uniform.get(),0,sizeof(p),&p);passes.push_back(std::move(pass));current=1-current;
        }
        }
        auto pass=std::make_unique<Pass>();auto &p=pass->parameters;
        pass->output=2;
        p.dimensions={documentSize.width(),documentSize.height(),outputSize.width(),outputSize.height()};p.factors={0,0,float(1<<packet->lod),1};
        if(!makePass(*pass,display,linear[current].texture.get(),axisX.get(),axisY.get(),maps[0].texture.get()))return fail("CPU: composition display pipeline allocation");
        batch->updateDynamicBuffer(pass->uniform.get(),0,sizeof(p),&p);passes.push_back(std::move(pass));return true;
    }
    void record(){
        const auto receiver=counters->readback;
        const bool readRequested=receiver && receiver->version.load()!=readVersion && receiver->geometry.load()==packet->geometrySequence;
        if(failed || (!dirty && !readRequested))return;
        if(qEnvironmentVariable("DRAWVERSE_COMPOSITION_FAIL")=="upload"){fail("CPU: injected composition upload failure");return;}
        auto *ri=window->rendererInterface();
        if(ri->getResource(window,QSGRendererInterface::RhiResource)!=rhi || rhi->isDeviceLost()){fail("CPU: composition device epoch lost");return;}
        auto *cb=static_cast<QRhiCommandBuffer*>(ri->getResource(window,QSGRendererInterface::RhiRedirectCommandBuffer));
        if(!cb){auto *swap=static_cast<QRhiSwapChain*>(ri->getResource(window,QSGRendererInterface::RhiSwapchainResource));if(swap)cb=swap->currentFrameCommandBuffer();}
        if(!cb){fail("CPU: composition window command buffer");return;}
        InteractionTrace::Scope trace("composition.record");
        if(InteractionTrace::enabled.load())InteractionTrace::point("composition.previous_completed_gpu_ms",cb->lastCompletedGpuTime()*1000.);
        int finalLinear=packet->outputSurface;
        if(packet->operations.isEmpty())for(const auto &node:packet->nodes)if(node.kind==0 && node.visible)finalLinear=1-finalLinear;
        if(dirty){
        auto *batch=rhi->nextResourceUpdateBatch();
        if(!prepare(batch)){batch->release();return;}
        cb->resourceUpdate(batch);
        for(size_t i=0;i<passes.size();++i){
            const auto &pass=*passes[i];const bool last=pass.output==2;auto &target=last?display:(pass.output==3?scratch:surface(pass.output<2?pass.output:pass.output-2));const auto size=last?outputSize:documentSize;
            cb->beginPass(target.target.get(),Qt::transparent,{1,0});cb->setGraphicsPipeline(pass.pipeline);cb->setViewport({0,0,float(size.width()),float(size.height())});
            const auto scissor=last?QRect(QPoint(),size):changed;
            // QRhi scissor coordinates are bottom-left even on D3D11.
            cb->setScissor({scissor.x(),size.height()-scissor.y()-scissor.height(),scissor.width(),scissor.height()});
            cb->setShaderResources(pass.bindings.get());
            if(pass.mesh){const QRhiCommandBuffer::VertexInput binding(meshVertices.get(),0);cb->setVertexInput(0,1,&binding);cb->draw(6,meshTriangleCount);}
            else cb->draw(3);
            cb->endPass();
        }
        counters->passes.fetch_add(passes.size());++counters->adoptions;counters->revision=packet->info.key.revision;
        if(!changed.isEmpty()){
            const quint64 area=quint64(changed.width())*changed.height();counters->lastDirtyArea=area;
            counters->composedPixels.fetch_add(area*(passes.size()-1));
        }
        counters->lastSubmitNs=InteractionTrace::now();counters->lastSubmitGeometry=packet->geometrySequence;
        renderedPacket=packet;dirty=false;if(!rendered){rendered=true;fallback({});}
        }
        if(readRequested){
            readVersion=receiver->version.load();auto *reads=rhi->nextResourceUpdateBatch();
            counters->testReadbacks.fetch_add(2);
            for(int i=0;i<2;++i){
                auto result=std::make_shared<QRhiReadbackResult>();
                result->completed=[result,receiver,i]{
                    {std::lock_guard lock(receiver->mutex);(i?receiver->display:receiver->linear)=result->data;}
                    ++receiver->completed;auto completed=std::move(result->completed);
                };
                reads->readBackTexture(QRhiReadbackDescription(i?display.texture.get():linear[finalLinear].texture.get()),result.get());
            }
            cb->resourceUpdate(reads);
        }
    }
    QQuickWindow *window;
    CompositionPacketPtr packet;
    CompositionPacketPtr renderedPacket,adoptedPacket;
    std::shared_ptr<CompositionCounters> counters;
    std::function<void(QString)> fallback;
    std::function<void(CompositionPacketPtr)> adoption;
    QMetaObject::Connection connection,adoptionConnection;
    QRhi *rhi=nullptr;
    int inFlightFrames=0;
    std::shared_ptr<TextureReservation> reservation;
    bool dirty=true,failed=false,rendered=false,completePasses=false;
    quint64 readVersion=~quint64(0);
    DirectTexture texture;
    QShader vertex,fragment,meshVertex,preciseFragment,warpFragment;
    QSize documentSize,outputSize;
    QRect changed;
    std::unique_ptr<QRhiSampler> sampler;
    std::unique_ptr<QRhiTexture> atlas,axisX,axisY;
    std::array<Target,2> linear;
    std::vector<Target> groupTargets;
    Target &surface(int index){return index<2?linear[size_t(index)]:groupTargets[size_t(index-2)];}
    Target display,scratch;
    std::unique_ptr<QRhiBuffer> meshVertices;
    quint32 meshTriangleCount=0;
    std::vector<PageMap> maps;
    std::vector<std::unique_ptr<Pass>> passes;
    std::unique_ptr<QRhiShaderResourceBindings> pipelineLayout;
    std::array<std::unique_ptr<QRhiRenderPassDescriptor>,2> pipelinePasses;
    std::array<std::unique_ptr<QRhiGraphicsPipeline>,4> pipelines;
    QHash<quint64,int> cache;
    std::array<quint64,1024> slotIds{};
    QVector<QVector<std::shared_ptr<const CompositionTileData>>> staging;
};
}
#endif
quint64 CompositionNode::reservedPayloadBytes(){
#ifdef DRAWVERSE_HAS_COMPOSITION_RHI
    return reservedTextureBytes.load();
#else
    return 0;
#endif
}
CompositionNode *CompositionNode::create(QQuickWindow *window,CompositionPacketPtr packet,std::shared_ptr<CompositionCounters> counters,std::function<void(QString)> fallback,std::function<void(CompositionPacketPtr)> adopted){
#ifdef DRAWVERSE_HAS_COMPOSITION_RHI
    static const bool initialized=(initializeCompositionResources(),true);Q_UNUSED(initialized);
    auto *node=new RhiCompositionNode(window,std::move(packet),std::move(counters),std::move(fallback),std::move(adopted));
    if(!node->initialize()){delete node;return nullptr;}return node;
#else
    Q_UNUSED(window);Q_UNUSED(packet);Q_UNUSED(counters);Q_UNUSED(adopted);fallback(QStringLiteral("CPU: Qt 6.6+ QRhi adapter unavailable"));return nullptr;
#endif
}
