#pragma once

#include <QByteArray>
#include <QtGlobal>
#include <cmath>

// Call once before QGuiApplication so every native window shares the same scale.
inline void configureUiScale() {
    bool valid = false;
    const double external = qgetenv("QT_SCALE_FACTOR").toDouble(&valid);
    const double base = valid && std::isfinite(external) && external > 0 ? external : 1.;
    const double scaled = base * 1.1;
    qputenv("QT_SCALE_FACTOR", QByteArray::number(std::isfinite(scaled) ? scaled : 1.1, 'g', 17));
}
