#pragma once
#include <QQuickPaintedItem>
#include <QImage>

// A bounded, cached UI color picker; it never reads document pixels or paint frames.
class ColorWheelItem : public QQuickPaintedItem {
    Q_OBJECT
    Q_PROPERTY(QColor color READ color WRITE setColor NOTIFY colorChanged)
    Q_PROPERTY(qreal hue READ hue NOTIFY colorChanged)
    Q_PROPERTY(qreal saturation READ saturation NOTIFY colorChanged)
    Q_PROPERTY(qreal value READ value NOTIFY colorChanged)
public:
    explicit ColorWheelItem(QQuickItem *parent = nullptr);
    QColor color() const { return m_color; }
    void setColor(const QColor &color);
    qreal hue() const { return m_hue; }
    qreal saturation() const { return m_color.hsvSaturationF(); }
    qreal value() const { return m_color.valueF(); }
    Q_INVOKABLE void pickHsv(qreal hue,qreal saturation,qreal value);
    void paint(QPainter *painter) override;
signals:
    void colorChanged();
    void picked(const QColor &color);
protected:
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
private:
    void pick(QPointF point);
    void rebuildTriangle();
    QColor m_color{Qt::red};
    qreal m_hue = 0;
    qreal m_cachedHue = -1;
    QImage m_triangle;
    bool m_ring = false;
};
