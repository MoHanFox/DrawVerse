#pragma once
#include <QQuickPaintedItem>
#include <QPainterPath>
#include <QVariantList>

// Consumes only copied C ABI geometry. Static outlines avoid a full-canvas animated texture.
class SelectionOverlay : public QQuickPaintedItem {
    Q_OBJECT
    Q_PROPERTY(QVariantList steps READ steps WRITE setSteps NOTIFY geometryChanged)
    Q_PROPERTY(bool enabledSelection READ enabledSelection WRITE setEnabledSelection NOTIFY geometryChanged)
    Q_PROPERTY(QRectF documentRect READ documentRect WRITE setDocumentRect NOTIFY geometryChanged)
    Q_PROPERTY(QRectF preview READ preview WRITE setPreview NOTIFY geometryChanged)
    Q_PROPERTY(int previewKind READ previewKind WRITE setPreviewKind NOTIFY geometryChanged)
    Q_PROPERTY(qreal zoom READ zoom WRITE setZoom NOTIFY geometryChanged)
public:
    explicit SelectionOverlay(QQuickItem *parent=nullptr);
    QVariantList steps() const{return m_steps;}
    bool enabledSelection() const{return m_enabled;}
    QRectF documentRect() const{return m_documentRect;}
    QRectF preview() const{return m_preview;}
    int previewKind() const{return m_previewKind;}
    qreal zoom() const{return m_zoom;}
    void setSteps(const QVariantList &steps);
    void setEnabledSelection(bool enabled);
    void setDocumentRect(QRectF rect);
    void setPreview(QRectF rect);
    void setPreviewKind(int kind);
    void setZoom(qreal zoom);
    void paint(QPainter *painter) override;
    QPainterPath documentPath() const{return m_path;}
signals:
    void geometryChanged();
private:
    void rebuild();
    QVariantList m_steps;
    QPainterPath m_path;
    QRectF m_documentRect,m_preview;
    qreal m_zoom=1;
    int m_previewKind=0;
    bool m_enabled=false;
};
