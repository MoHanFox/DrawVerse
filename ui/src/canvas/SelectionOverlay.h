#pragma once
#include <QQuickPaintedItem>
#include <QPainterPath>
#include <QVariantList>
#include <QTimer>
#include <QPointer>
#include <QWindow>

// Consumes only copied C ABI geometry and paints the selection outline as marching ants: a black
// underlay with a white dashed line whose dash phase advances, so the user can confirm the region.
// Magic-wand steps carry bounded copied alpha masks; the half-coverage boundary supplies the path.
class SelectionOverlay : public QQuickPaintedItem {
    Q_OBJECT
    Q_PROPERTY(QVariantList steps READ steps WRITE setSteps NOTIFY geometryChanged)
    Q_PROPERTY(bool enabledSelection READ enabledSelection WRITE setEnabledSelection NOTIFY geometryChanged)
    Q_PROPERTY(QRectF documentRect READ documentRect WRITE setDocumentRect NOTIFY geometryChanged)
    Q_PROPERTY(QVariantList previewPoints READ previewPoints WRITE setPreviewPoints NOTIFY geometryChanged)
    Q_PROPERTY(QRectF preview READ preview WRITE setPreview NOTIFY geometryChanged)
    Q_PROPERTY(int previewKind READ previewKind WRITE setPreviewKind NOTIFY geometryChanged)
    Q_PROPERTY(qreal zoom READ zoom WRITE setZoom NOTIFY geometryChanged)
    Q_PROPERTY(int dashPhase READ dashPhase NOTIFY dashPhaseChanged)
public:
    explicit SelectionOverlay(QQuickItem *parent=nullptr);
    QVariantList steps() const{return m_steps;}
    bool enabledSelection() const{return m_enabled;}
    QRectF documentRect() const{return m_documentRect;}
    QVariantList previewPoints() const {return m_previewPoints;}
    void setPreviewPoints(const QVariantList &points);
    QRectF preview() const{return m_preview;}
    int previewKind() const{return m_previewKind;}
    qreal zoom() const{return m_zoom;}
    int dashPhase() const{return m_phase;}
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
    void dashPhaseChanged();
protected:
    void itemChange(ItemChange change,const ItemChangeData &value) override;
    void componentComplete() override;
private:
    void rebuild();
    void syncAnimation();
    static QPainterPath previewPath(const QRectF &rect,int kind);
    QVariantList m_steps,m_previewPoints;
    QPainterPath currentPreview() const;
    QPainterPath m_path;
    QVector<QPainterPath> m_shapes;
    QRectF m_documentRect,m_preview;
    qreal m_zoom=1;
    int m_previewKind=0;
    int m_phase=0;
    bool m_enabled=false;
    QTimer m_ants;
    QPointer<QWindow> m_hostWindow;
};
