#pragma once
#include <QVariantMap>

// Pure rules for the standard library preview, never for executing presets.
namespace BrushPreviewPolicy {
QVariantMap effectiveTip(QVariantMap tip);
}
