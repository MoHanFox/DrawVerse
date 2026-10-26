#include "canvas/LiquifyPreviewNode.h"
#include "canvas/ResidentPreviewTextures.h"
#include <QQuickWindow>
#include "platform/InteractionTrace.h"
#include <QSGMaterial>
#include <QSGMaterialShader>
#include <QSGTexture>
#include <cstring>
#include <functional>

namespace {
struct Vertex {float x,y,u,v,du,dv;};
const QSGGeometry::Attribute attributes[]={
    QSGGeometry::Attribute::create(0,2,QSGGeometry::FloatType,true),
    QSGGeometry::Attribute::create(1,2,QSGGeometry::FloatType),
    QSGGeometry::Attribute::create(2,2,QSGGeometry::FloatType)};
const QSGGeometry::AttributeSet attributeSet={3,sizeof(Vertex),attributes};

class Material;
class Shader final : public QSGMaterialShader {
public:
    Shader(){setFlags(UpdatesGraphicsPipelineState);setShaderFileName(VertexStage,QStringLiteral(":/shaders/liquify.vert.qsb"));setShaderFileName(FragmentStage,QStringLiteral(":/shaders/liquify.frag.qsb"));}
    bool updateUniformData(RenderState &state,QSGMaterial *material,QSGMaterial *) override;
    bool updateGraphicsPipelineState(RenderState &,GraphicsPipelineState *,QSGMaterial *,QSGMaterial *) override;
    void updateSampledImage(RenderState &,int binding,QSGTexture **texture,QSGMaterial *material,QSGMaterial *) override;
};
class Material final : public QSGMaterial {
public:
    std::array<std::shared_ptr<ResidentPreviewTextures>,2> sources{};
    std::array<float,4> identity{};
    bool selected=false;
    bool focus=false,forward=false;
    Material(){setFlag(Blending);}
    QSGMaterialType *type() const override {static QSGMaterialType t;return &t;}
    QSGMaterialShader *createShader(QSGRendererInterface::RenderMode) const override {return new Shader;}
    int compare(const QSGMaterial *other) const override {return this==other?0:std::less<const QSGMaterial*>{}(this,other)?-1:1;}
};
bool Shader::updateGraphicsPipelineState(RenderState &,GraphicsPipelineState *pipeline,QSGMaterial *material,QSGMaterial *) {
    if(!static_cast<Material*>(material)->forward)return false;
    // Stay in Qt's ordered transparent pass (no opaque depth ownership), but replace RGBA.
    // Folding one layer must not accumulate its opacity or let the first sheet keep depth.
    pipeline->srcColor=GraphicsPipelineState::One;pipeline->dstColor=GraphicsPipelineState::Zero;
    pipeline->separateBlendFactors=false;return true;
}
bool Shader::updateUniformData(RenderState &state,QSGMaterial *material,QSGMaterial *) {
    auto *bytes=state.uniformData();
    if(state.isMatrixDirty()){const auto matrix=state.combinedMatrix();std::memcpy(bytes->data(),matrix.constData(),64);}
    const float opacity=state.opacity(),focus=static_cast<Material*>(material)->focus?1.f:0.f;
    std::memcpy(bytes->data()+64,&opacity,4);std::memcpy(bytes->data()+68,&focus,4);
    const auto *m=static_cast<Material*>(material);const float selected=m->selected?1.f:0.f;
    for(const auto &source:m->sources)if(source)source->commit(state);
    std::memcpy(bytes->data()+72,&selected,4);std::memcpy(bytes->data()+80,m->identity.data(),16);
    return true;
}
void Shader::updateSampledImage(RenderState &state,int binding,QSGTexture **texture,QSGMaterial *material,QSGMaterial *) {
    if(binding>=1 && binding<=4){
        *texture=static_cast<Material*>(material)->sources[0]->images[size_t(binding-1)];
        if(*texture)(*texture)->commitTextureOperations(state.rhi(),state.resourceUpdateBatch());
    }
}
}

LiquifyPreviewNode::LiquifyPreviewNode() {
    auto *g=new QSGGeometry(attributeSet,0,0,QSGGeometry::UnsignedIntType);
    g->setDrawingMode(QSGGeometry::DrawTriangles);g->setVertexDataPattern(QSGGeometry::DynamicPattern);
    setGeometry(g);setFlag(OwnsGeometry);setMaterial(new Material);setFlag(OwnsMaterial);
}
bool LiquifyPreviewNode::sync(QQuickWindow *window,const LiquifyPreviewFrame &frame,const QRectF &rect) {
    auto *m=static_cast<Material*>(material());
    if(m_source!=frame.sourceId){
        auto current=ResidentPreviewTextures::acquire(window,frame);
        if(!current)return false;
        auto prepared=frame.prepared?ResidentPreviewTextures::acquire(window,*frame.prepared):nullptr;
        if(frame.prepared && !prepared)return false;
        m->sources={std::move(current),std::move(prepared)};
        m->selected=!frame.selection.isNull();m->identity=frame.identity;m_source=frame.sourceId;m_revision=0;markDirty(DirtyMaterial);
    }
    if(m->forward!=frame.forward){m->forward=frame.forward;markDirty(DirtyMaterial);}
    const bool sourceOnly=frame.focus || frame.forward;
    if(m->focus!=sourceOnly){m->focus=sourceOnly;markDirty(DirtyMaterial);}
    if(m_revision==frame.revision && m_rect==rect && m_presentationDelta==frame.presentationDelta)return true;
    InteractionTrace::Scope trace("move.render.adoptGeometry");
    if(trace.active)trace.event.value=double(frame.revision);
    InteractionTrace::point("move.render.source",double(frame.sourceId));
    auto *g=geometry();
    const auto count=int(frame.vertices.size());
    const auto indices=int((frame.columns-1)*(frame.rows-1)*6);
    const bool resize=g->vertexCount()!=count || g->indexCount()!=indices;
    if(resize)g->allocate(count,indices);
    auto *vertices=static_cast<Vertex*>(g->vertexData());
    const auto width=float(frame.documentSize.width()),height=float(frame.documentSize.height());
    for(int i=0;i<count;++i){const auto &v=frame.vertices[i];vertices[i]={float(rect.x()+v[0]/width*rect.width()),float(rect.y()+v[1]/height*rect.height()),v[2],v[3],v[0]/width,v[1]/height};}
    if(resize){
        auto *index=g->indexDataAsUInt();
        for(quint32 y=0;y+1<frame.rows;++y)for(quint32 x=0;x+1<frame.columns;++x){
            const quint32 a=y*frame.columns+x,b=a+1,c=a+frame.columns,d=c+1;
            *index++=a;*index++=b;*index++=d;*index++=a;*index++=d;*index++=c;
        }
        g->markIndexDataDirty();
    }
    g->markVertexDataDirty();markDirty(DirtyGeometry);m_revision=frame.revision;m_rect=rect;m_presentationDelta=frame.presentationDelta;
    return true;
}
