#include "brushes/BrushPreviewPolicy.h"

namespace {
void disabledModules(QVariantMap &tip) {
    for(const auto *key:{"scatterDynamics","shapeJitter","colorDynamics","texture","dualBrush","smoothing","wetMix","strokeTaper","strokeOutline",
                         "speedDynamics","tangentDynamics","distanceDynamics","tiltDynamics","rotationDynamics","directionDynamics","fadeDynamics"}) {
        const auto module=tip.value(key).toMap();
        if(!module.value("enabled").toBool())tip.remove(key);
    }
}
void standardPressure(QVariantMap &tip) {
    // Keep the synthetic stroke independent of live pen pressure settings.
    // Omitting pressureDynamics selects the engine's linear size/flow curves.
    tip["pressureSize"]=true;tip["pressureFlow"]=true;
    tip.remove("pressureDynamics");
}
void tipShape(QVariantMap &tip) {
    if(!tip.value("maskResource").toString().isEmpty() || !tip.value("resource").toString().isEmpty())tip["hardness"]=1.;
    auto device=tip.value("tipDynamics").toMap();
    if(device.isEmpty()){tip.remove("tipDynamics");return;}
    if(!device.value("tiltRoundness").toBool())device["minimumRoundness"]=.01;
    const QVariantMap defaults{{"angleControl","fixed"},{"tiltRoundness",false},{"minimumRoundness",.01}};
    if(device==defaults)tip.remove("tipDynamics");else tip["tipDynamics"]=device;
}
void randomSeeds(QVariantMap &tip) {
    struct SeedRule {const char *module;QStringList strengths;};
    const SeedRule rules[]={
        {"shapeJitter",{"size","angle","roundness"}},
        {"colorDynamics",{"foregroundBackground","hue","saturation","value"}},
        {"scatterDynamics",{"amount","countJitter"}}
    };
    for(const auto &rule:rules) {
        auto module=tip.value(rule.module).toMap();if(module.isEmpty())continue;
        bool active=false;for(const auto &strength:rule.strengths)active|=module.value(strength).toDouble()!=0.;
        if(!active){module["seed"]=QStringLiteral("0000000000000000");tip[rule.module]=module;}
    }
}
void brushRules(QVariantMap &tip) {
    disabledModules(tip);standardPressure(tip);tipShape(tip);randomSeeds(tip);
}
void secondaryBrush(QVariantMap &tip) {
    auto dual=tip.value("dualBrush").toMap();if(dual.isEmpty())return;
    brushRules(dual);tip["dualBrush"]=dual;
}
void outlineWidth(QVariantMap &tip) {
    auto outline=tip.value("strokeOutline").toMap();if(outline.isEmpty())return;
    auto width=outline.value("width").toMap();
    width[width.value("relative").toBool()?"pixels":"percent"]=0.;outline["width"]=width;
    auto source=outline.value("source").toMap();
    if(source.value("model").toString()=="existing-brush") {
        auto brush=source.value("brush").toMap();brushRules(brush);source["brush"]=brush;
    }
    outline["source"]=source;tip["strokeOutline"]=outline;
}
void standardSample(QVariantMap &tip) {
    tip["flow"]=1.;tip["presetRadius"]=24.;tip["writeBlend"]=0;
    // The standard preview submits a moving stroke, without stationary ticks.
    tip.remove("airbrush");
}
struct Rule {const char *name;void (*apply)(QVariantMap &);};
const Rule rules[]={
    {"disabled-modules",disabledModules},
    {"standard-pressure",standardPressure},
    {"tip-shape",tipShape},
    {"inactive-random-seeds",randomSeeds},
    {"secondary-brush",secondaryBrush},
    {"outline-width-and-source",outlineWidth},
    {"standard-sample",standardSample}
};
}
QVariantMap BrushPreviewPolicy::effectiveTip(QVariantMap tip) {
    for(const auto &rule:rules)rule.apply(tip);
    return tip;
}
