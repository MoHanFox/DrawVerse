#include "canvas/LiquifyPreviewNode.h"
#include <QQuickWindow>
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
    Shader(){setShaderFileName(VertexStage,QStringLiteral(":/shaders/liquify.vert.qsb"));setShaderFileName(FragmentStage,QStringLiteral(":/shaders/liquify.frag.qsb"));}
    bool updateUniformData(RenderState &state,QSGMaterial *material,QSGMaterial *) override;
    void updateSampledImage(RenderState &,int binding,QSGTexture **texture,QSGMaterial *material,QSGMaterial *) override;
};
class Material final : public QSGMaterial {
public:
    std::array<QSGTexture*,3> textures{};
    bool focus=false;
    Material(){setFlag(Blending);}
    ~Material() override {for(auto *texture:textures)delete texture;}
    QSGMaterialType *type() const override {static QSGMaterialType t;return &t;}
    QSGMaterialShader *createShader(QSGRendererInterface::RenderMode) const override {return new Shader;}
    int compare(const QSGMaterial *other) const override {return this==other?0:std::less<const QSGMaterial*>{}(this,other)?-1:1;}
};
bool Shader::updateUniformData(RenderState &state,QSGMaterial *material,QSGMaterial *) {
    auto *bytes=state.uniformData();
    if(state.isMatrixDirty()){const auto matrix=state.combinedMatrix();std::memcpy(bytes->data(),matrix.constData(),64);}
    const float opacity=state.opacity(),focus=static_cast<Material*>(material)->focus?1.f:0.f;
    std::memcpy(bytes->data()+64,&opacity,4);std::memcpy(bytes->data()+68,&focus,4);
    return true;
}
void Shader::updateSampledImage(RenderState &state,int binding,QSGTexture **texture,QSGMaterial *material,QSGMaterial *) {
    if(binding>=1 && binding<=3){
        *texture=static_cast<Material*>(material)->textures[size_t(binding-1)];
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
        std::array<QSGTexture*,3> next{};
        for(size_t i=0;i<3;++i){
            next[i]=window->createTextureFromImage(frame.images[i],QQuickWindow::TextureHasAlphaChannel);
            if(!next[i]){for(auto *texture:next)delete texture;return false;}
            next[i]->setFiltering(QSGTexture::Nearest);
            next[i]->setHorizontalWrapMode(QSGTexture::ClampToEdge);next[i]->setVerticalWrapMode(QSGTexture::ClampToEdge);
        }
        for(auto *texture:m->textures)delete texture;
        m->textures=next;m_source=frame.sourceId;m_revision=0;markDirty(DirtyMaterial);
    }
    if(m->focus!=frame.focus){m->focus=frame.focus;markDirty(DirtyMaterial);}
    if(m_revision==frame.revision && m_rect==rect)return true;
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
    g->markVertexDataDirty();markDirty(DirtyGeometry);m_revision=frame.revision;m_rect=rect;
    return true;
}
