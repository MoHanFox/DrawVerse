#pragma once
#include <QQuickPaintedItem>
#include <QQuickWindow>
#include <QImage>
#include <QPointer>

class FrostedSurface : public QQuickPaintedItem {
    Q_OBJECT
    Q_PROPERTY(qreal radius READ radius WRITE setRadius NOTIFY materialChanged)
    Q_PROPERTY(QColor tint READ tint WRITE setTint NOTIFY materialChanged)
    Q_PROPERTY(bool topCornersOnly READ topCornersOnly WRITE setTopCornersOnly NOTIFY materialChanged)
    Q_PROPERTY(bool hasBackdrop READ hasBackdrop NOTIFY backdropChanged)
public:
    explicit FrostedSurface(QQuickItem *parent=nullptr);
    qreal radius() const { return m_radius; }
    QColor tint() const { return m_tint; }
    bool topCornersOnly() const { return m_topCornersOnly; }
    bool hasBackdrop() const { return !m_backdrop.isNull(); }
    void setRadius(qreal value);
    void setTint(QColor value);
    void setTopCornersOnly(bool value);
    Q_INVOKABLE void capture(QQuickWindow *source);
    Q_INVOKABLE void relocate();
    Q_INVOKABLE void clear();
    void paint(QPainter *painter) override;
    static QImage blurredBackdrop(const QImage &image);
signals:
    void materialChanged();
    void backdropChanged();
private:
    qreal m_radius=10;
    QColor m_tint{QStringLiteral("#bf1c1e21")};
    bool m_topCornersOnly=false;
    QImage m_backdrop;
    QPointer<QQuickWindow> m_source;
    QSize m_sourceSize;
    QPointF m_origin;
};
