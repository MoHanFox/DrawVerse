#pragma once
#include <QQuickPaintedItem>

// A flat translucent surface. It never samples or caches window contents.
class MenuSurface : public QQuickPaintedItem {
    Q_OBJECT
    Q_PROPERTY(qreal radius READ radius WRITE setRadius NOTIFY materialChanged)
    Q_PROPERTY(QColor tint READ tint WRITE setTint NOTIFY materialChanged)
    Q_PROPERTY(bool topCornersOnly READ topCornersOnly WRITE setTopCornersOnly NOTIFY materialChanged)
public:
    explicit MenuSurface(QQuickItem *parent=nullptr);
    qreal radius() const { return m_radius; }
    QColor tint() const { return m_tint; }
    bool topCornersOnly() const { return m_topCornersOnly; }
    void setRadius(qreal value);
    void setTint(QColor value);
    void setTopCornersOnly(bool value);
    void paint(QPainter *painter) override;
signals:
    void materialChanged();
private:
    qreal m_radius=10;
    QColor m_tint{QStringLiteral("#bf1c1e21")};
    bool m_topCornersOnly=false;
};
