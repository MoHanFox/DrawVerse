// G0 only. Opt-in executable: no production renderer or document ownership.
#include <QtTest>
#include <QQuickItem>
#include <QQuickWindow>
#include <QSGSimpleTextureNode>
#include <QSGRendererInterface>
#include <QScreen>
#include <QJsonDocument>
#include <QFile>
#include <QScopeGuard>
#include <rhi/qrhi.h>
#include <array>
#include <mutex>
#include <memory>
#include <cmath>
#include <cstring>

namespace {
constexpr int side=64, count=side*side;
using Pixel=std::array<float,4>;
struct Fixture {
    std::array<QByteArray,3> input;
    std::vector<Pixel> linear;
    QImage display{side,side,QImage::Format_RGBA8888_Premultiplied};
    Fixture() {
        std::array<std::vector<Pixel>,3> pixels;
        for(auto &p:pixels)p.resize(count);
        linear.resize(count);
        for(int y=0;y<side;++y)for(int x=0;x<side;++x){
            const int i=y*side+x;
            const float a=float((x+3*y)%17)/16, b=float((2*x+y)%13)/12;
            const float hidden=float((x+y)%9)/8;
            pixels[0][i]={a*x/63,a*y/63,a*.21f,a};
            pixels[1][i]={b*.04f,b*.17f,b*(63-x)/63,b};
            // Match core masks: stored alpha is the hidden amount.
            pixels[2][i]={0,0,0,hidden};
            const float coverage=1-hidden*.75f;
            const float alpha=a*coverage;
            Pixel p;
            for(int c=0;c<4;++c)p[c]=pixels[0][i][c]*coverage+pixels[1][i][c]*(1-alpha);
            linear[i]=p;
            auto *out=display.scanLine(y)+4*x;
            for(int c=0;c<3;++c){
                const float v=p[3]>0?std::clamp(p[c]/p[3],0.f,1.f):0;
                const float s=v<=.0031308f?12.92f*v:1.055f*std::pow(v,1.f/2.4f)-.055f;
                out[c]=uchar(std::lround(s*p[3]*255));
            }
            out[3]=uchar(std::lround(p[3]*255));
        }
        for(int j=0;j<3;++j)input[j]=QByteArray(reinterpret_cast<const char*>(pixels[j].data()),count*sizeof(Pixel));
    }
};
struct Evidence {
    std::mutex mutex;
    QString error,device;
    quintptr rhi=0;
    int created=0,destroyed=0,uploads=0,passes=0,readbacks=0,readDone=0,frames=0;
    std::array<QByteArray,2> read;
    QJsonObject snapshot() {
        std::lock_guard lock(mutex);
        return {{"error",error},{"device",device},{"rhi",QString::number(rhi,16)},
            {"created",created},{"destroyed",destroyed},{"upload_batches",uploads},
            {"passes",passes},{"test_readbacks",readbacks},{"read_done",readDone},{"frames",frames}};
    }
};
class Texture final : public QSGTexture {
public:
    QRhiTexture *value=nullptr;
    qint64 comparisonKey() const override {return qint64(quintptr(value));}
    QRhiTexture *rhiTexture() const override {return value;}
    QSize textureSize() const override {return {side,side};}
    bool hasAlphaChannel() const override {return true;}
    bool hasMipmaps() const override {return false;}
};
class Node final : public QSGSimpleTextureNode {
public:
    Node(QQuickWindow *w,const Fixture &f,std::shared_ptr<Evidence> e):window(w),fixture(f),evidence(std::move(e)) {
        setFiltering(QSGTexture::Nearest);
        connection=QObject::connect(w,&QQuickWindow::beforeRendering,w,[this]{record();},Qt::DirectConnection);
    }
    ~Node() override {
        QObject::disconnect(connection);
        if(initialized){std::lock_guard lock(evidence->mutex);++evidence->destroyed;}
    }
    bool initialize() {
        auto *ri=window->rendererInterface();
        rhi=static_cast<QRhi*>(ri->getResource(window,QSGRendererInterface::RhiResource));
        if(!rhi || !rhi->isTextureFormatSupported(QRhiTexture::RGBA32F,QRhiTexture::RenderTarget))return fail("RGBA32F render target unavailable");
        for(int j=0;j<3;++j){
            input[j].reset(rhi->newTexture(QRhiTexture::RGBA32F,{side,side}));
            if(!input[j]->create())return fail("input allocation failed");
            output[j].reset(rhi->newTexture(j==2?QRhiTexture::RGBA8:QRhiTexture::RGBA32F,{side,side},1,
                QRhiTexture::RenderTarget|QRhiTexture::UsedAsTransferSource));
            if(!output[j]->create())return fail("target allocation failed");
            target[j].reset(rhi->newTextureRenderTarget(QRhiTextureRenderTargetDescription(QRhiColorAttachment(output[j].get()))));
            rp[j].reset(target[j]->newCompatibleRenderPassDescriptor());target[j]->setRenderPassDescriptor(rp[j].get());
            if(!target[j]->create())return fail("render target creation failed");
        }
        sampler.reset(rhi->newSampler(QRhiSampler::Nearest,QRhiSampler::Nearest,QRhiSampler::None,QRhiSampler::ClampToEdge,QRhiSampler::ClampToEdge));
        if(!sampler->create())return fail("sampler allocation failed");
        const auto shader=[](const char *name){QFile f(name);if(!f.open(QIODevice::ReadOnly))return QShader{};return QShader::fromSerialized(f.readAll());};
        const auto vs=shader(":/g0/g0.vert.qsb"),fs=shader(":/g0/g0.frag.qsb");
        if(!vs.isValid() || !fs.isValid())return fail("shader missing");
        for(int j=0;j<3;++j){
            uniform[j].reset(rhi->newBuffer(QRhiBuffer::Dynamic,QRhiBuffer::UniformBuffer,16));
            if(!uniform[j]->create())return fail("uniform allocation failed");
            bindings[j].reset(rhi->newShaderResourceBindings());
            bindings[j]->setBindings({
                QRhiShaderResourceBinding::uniformBuffer(0,QRhiShaderResourceBinding::FragmentStage,uniform[j].get()),
                QRhiShaderResourceBinding::sampledTexture(1,QRhiShaderResourceBinding::FragmentStage,j==0?input[0].get():output[j-1].get(),sampler.get()),
                QRhiShaderResourceBinding::sampledTexture(2,QRhiShaderResourceBinding::FragmentStage,j==0?input[2].get():input[1].get(),sampler.get())});
            if(!bindings[j]->create())return fail("bindings allocation failed");
            pipeline[j].reset(rhi->newGraphicsPipeline());
            pipeline[j]->setShaderStages({{QRhiShaderStage::Vertex,vs},{QRhiShaderStage::Fragment,fs}});
            pipeline[j]->setShaderResourceBindings(bindings[j].get());pipeline[j]->setRenderPassDescriptor(rp[j].get());
            if(!pipeline[j]->create())return fail("pipeline allocation failed");
        }
        texture.value=output[2].get();setTexture(&texture);
        if(rhi->isYUpInFramebuffer())setTextureCoordinatesTransform(QSGSimpleTextureNode::MirrorVertically);
        initialized=true;
        std::lock_guard lock(evidence->mutex);++evidence->created;evidence->rhi=quintptr(rhi);evidence->device=QString::fromUtf8(rhi->driverInfo().deviceName);
        return true;
    }
    bool requestRead=false;
private:
    bool fail(QString message){std::lock_guard lock(evidence->mutex);evidence->error=std::move(message);return false;}
    void record() {
        if(!initialized)return;
        auto *ri=window->rendererInterface();
        if(ri->getResource(window,QSGRendererInterface::RhiResource)!=rhi){fail("device epoch changed; reject old resources");return;}
        auto *cb=static_cast<QRhiCommandBuffer*>(ri->getResource(window,QSGRendererInterface::RhiRedirectCommandBuffer));
        if(!cb){auto *sc=static_cast<QRhiSwapChain*>(ri->getResource(window,QSGRendererInterface::RhiSwapchainResource));if(sc)cb=sc->currentFrameCommandBuffer();}
        if(!cb){fail("no window command buffer");return;}
        if(!uploaded){
            auto *batch=rhi->nextResourceUpdateBatch();
            for(int j=0;j<3;++j){
                batch->uploadTexture(input[j].get(),QRhiTextureUploadDescription({QRhiTextureUploadEntry(0,0,QRhiTextureSubresourceUploadDescription(fixture.input[j]))}));
                const std::array<float,4> p{float(j),.75f,0,0};batch->updateDynamicBuffer(uniform[j].get(),0,16,p.data());
            }
            cb->resourceUpdate(batch);
            for(int j=0;j<3;++j){
                cb->beginPass(target[j].get(),Qt::transparent,{1,0});
                cb->setGraphicsPipeline(pipeline[j].get());cb->setViewport({0,0,float(side),float(side)});
                cb->setShaderResources(bindings[j].get());cb->draw(3);cb->endPass();
            }
            uploaded=true;std::lock_guard lock(evidence->mutex);++evidence->uploads;evidence->passes+=3;
        }
        if(requestRead && !readIssued){
            auto *batch=rhi->nextResourceUpdateBatch();
            for(int j=0;j<2;++j){
                // Test-only asynchronous readback. No texture is created from it.
                // The completion owns its result, independent of node lifetime.
                // Move the callable to a local before returning to break that
                // ownership cycle only after all result access has finished.
                auto result=std::make_shared<QRhiReadbackResult>();
                result->completed=[e=evidence,j,result]{
                    {std::lock_guard lock(e->mutex);e->read[j]=result->data;++e->readDone;}
                    auto completed=std::move(result->completed);
                };
                batch->readBackTexture(QRhiReadbackDescription(output[j+1].get()),result.get());
            }
            cb->resourceUpdate(batch);readIssued=true;
            std::lock_guard lock(evidence->mutex);evidence->readbacks+=2;
        }
        std::lock_guard lock(evidence->mutex);++evidence->frames;
    }
    QQuickWindow *window;
    const Fixture &fixture;
    std::shared_ptr<Evidence> evidence;
    QMetaObject::Connection connection;
    QRhi *rhi=nullptr;
    bool initialized=false,uploaded=false,readIssued=false;
    std::array<std::unique_ptr<QRhiTexture>,3> input,output;
    std::array<std::unique_ptr<QRhiRenderPassDescriptor>,3> rp;
    std::array<std::unique_ptr<QRhiTextureRenderTarget>,3> target;
    std::unique_ptr<QRhiSampler> sampler;
    std::array<std::unique_ptr<QRhiBuffer>,3> uniform;
    std::array<std::unique_ptr<QRhiShaderResourceBindings>,3> bindings;
    std::array<std::unique_ptr<QRhiGraphicsPipeline>,3> pipeline;
    Texture texture;
};
class Item : public QQuickItem {
public:
    Fixture fixture;
    std::shared_ptr<Evidence> evidence=std::make_shared<Evidence>();
    bool read=false;
    explicit Item(QQuickItem *parent):QQuickItem(parent){setFlag(ItemHasContents);setSize({side,side});}
    QSGNode *updatePaintNode(QSGNode *old,UpdatePaintNodeData *) override {
        if(window()->rendererInterface()->graphicsApi()==QSGRendererInterface::Software){
            auto *node=static_cast<QSGSimpleTextureNode*>(old);
            if(!node){node=new QSGSimpleTextureNode;node->setTexture(window()->createTextureFromImage(fixture.display));node->setOwnsTexture(true);node->setFiltering(QSGTexture::Nearest);}
            node->setRect(boundingRect());return node;
        }
        auto *node=static_cast<Node*>(old);
        if(!node){node=new Node(window(),fixture,evidence);if(!node->initialize()){delete node;return nullptr;}}
        node->setRect(boundingRect());node->requestRead=read;return node;
    }
};
}
class GpuCompositionProbe final : public QObject {
    Q_OBJECT
private slots:
    void sameWindowMultipass() {
        QJsonObject report{{"qt",qVersion()},{"passed",false},{"texture_payload_bytes",344064},{"upload_bytes",196608},
            {"normal_path_readbacks",0},{"precision","RGBA32F intermediates, RGBA8 display"}};
        const auto output=qEnvironmentVariable("DRAWVERSE_G0_REPORT");
        const auto save=qScopeGuard([&]{QFile f(output);if(f.open(QIODevice::WriteOnly))f.write(QJsonDocument(report).toJson());});
        QVERIFY(!output.isEmpty());
        // Native Windows imposes a minimum decorated window width. Use a
        // larger integer multiple instead of assuming resize(64,64) is exact.
        QQuickWindow window;window.resize(256,256);window.setColor(Qt::black);
        window.setPersistentSceneGraph(false);window.setPersistentGraphics(false);
        auto *item=new Item(window.contentItem());item->setSize({256,256});auto e=item->evidence;
        window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));
        report["api"]=int(window.rendererInterface()->graphicsApi());report["dpr"]=window.devicePixelRatio();report["refresh_hz"]=window.screen()->refreshRate();
        const bool software=window.rendererInterface()->graphicsApi()==QSGRendererInterface::Software;
        report["cpu_fallback"]=software;
        if(!software){
            QCOMPARE(window.rendererInterface()->graphicsApi(),QSGRendererInterface::Direct3D11);
            QTRY_VERIFY(e->snapshot()["passes"].toInt()==3 || !e->snapshot()["error"].toString().isEmpty());
            QVERIFY2(e->snapshot()["error"].toString().isEmpty(),qPrintable(e->snapshot()["error"].toString()));
            for(int j=0;j<8;++j){const auto before=e->snapshot()["frames"].toInt();window.update();QTRY_VERIFY(e->snapshot()["frames"].toInt()>before);}
            QCOMPARE(e->snapshot()["upload_batches"].toInt(),1);QCOMPARE(e->snapshot()["passes"].toInt(),3);QCOMPARE(e->snapshot()["test_readbacks"].toInt(),0);
            report["direct_display_before_readback"]=e->snapshot();
            item->read=true;item->update();
            // Ensure command completion can progress, with a finite deadline.
            QTimer pump;connect(&pump,&QTimer::timeout,&window,&QQuickWindow::update);pump.start(16);
            QTRY_COMPARE_WITH_TIMEOUT(e->snapshot()["read_done"].toInt(),2,10000);pump.stop();
            double maxLinear=0;int maxByte=0;QJsonArray histogram;std::array<int,3> errors{};
            {
                std::lock_guard lock(e->mutex);QCOMPARE(e->read[0].size(),qsizetype(count*sizeof(Pixel)));QCOMPARE(e->read[1].size(),qsizetype(count*4));
                for(int i=0;i<count;++i){Pixel pixel;std::memcpy(pixel.data(),e->read[0].constData()+i*sizeof(Pixel),sizeof(Pixel));
                    for(int c=0;c<4;++c){QVERIFY(std::isfinite(pixel[c]));maxLinear=std::max(maxLinear,double(std::abs(pixel[c]-item->fixture.linear[i][c])));
                        const int delta=std::abs(int(uchar(e->read[1][i*4+c]))-int(item->fixture.display.constBits()[i*4+c]));maxByte=std::max(maxByte,delta);++errors[std::min(delta,2)];}
                }
            }
            for(auto v:errors)histogram.append(v);report["max_linear_error"]=maxLinear;report["max_rgba8_error"]=maxByte;report["rgba8_error_histogram_0_1_other"]=histogram;
            {
                std::lock_guard lock(e->mutex);
                for(int j=0;j<2;++j){QFile f(output+(j==0?".gpu-linear.bin":".gpu-display.bin"));QVERIFY(f.open(QIODevice::WriteOnly));QCOMPARE(f.write(e->read[j]),e->read[j].size());}
            }
            QFile reference(output+".cpu-linear.bin");QVERIFY(reference.open(QIODevice::WriteOnly));
            QCOMPARE(reference.write(reinterpret_cast<const char*>(item->fixture.linear.data()),count*sizeof(Pixel)),qint64(count*sizeof(Pixel)));
            QVERIFY(item->fixture.display.save(output+".cpu-display.png"));
            QVERIFY(maxLinear<=2e-6);QVERIFY(maxByte<=1);
        }
        // Independent real window correctness: raw linear passes above have no
        // screen blending. Here compare premultiplied RGB over opaque black.
        auto image=window.grabWindow().convertToFormat(QImage::Format_RGBA8888);QVERIFY(!image.isNull());
        QVERIFY(image.save(output+".png"));
        int maxWindow=0;
        for(int y=0;y<image.height();++y)for(int x=0;x<image.width();++x){
            const auto *expected=item->fixture.display.constScanLine(y*side/image.height())+(x*side/image.width())*4;
            const auto *actual=image.constScanLine(y)+x*4;
            for(int c=0;c<3;++c)maxWindow=std::max(maxWindow,std::abs(int(actual[c])-int(expected[c])));
        }
        report["max_window_rgb_error"]=maxWindow;QVERIFY(maxWindow<=1);
        if(!software){
            QQuickWindow other;other.resize(256,256);auto *second=new Item(other.contentItem());second->setSize({256,256});other.show();QVERIFY(QTest::qWaitForWindowExposed(&other));
            QTRY_COMPARE(second->evidence->snapshot()["passes"].toInt(),3);
            QVERIFY(second->evidence->snapshot()["rhi"]!=e->snapshot()["rhi"]);report["second_window"]=second->evidence->snapshot();
            window.hide();window.releaseResources();QTRY_COMPARE(e->snapshot()["destroyed"].toInt(),1);
            item->read=false;window.show();QVERIFY(QTest::qWaitForWindowExposed(&window));QTRY_COMPARE(e->snapshot()["created"].toInt(),2);
            report["after_rebuild"]=e->snapshot();QCOMPARE(e->snapshot()["upload_batches"].toInt(),2);
        }
        report["passed"]=true;
    }
};
QTEST_MAIN(GpuCompositionProbe)
#include "gpu_composition_probe.moc"
