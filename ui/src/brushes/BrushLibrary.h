#pragma once
#include <QObject>
#include <QColor>
#include <QJsonObject>
#include <QJsonArray>
#include <QHash>
#include <QSet>
#include <QVariantList>
#include <QStringList>
#include <QTimer>
#include <algorithm>
#include <array>

// UI preferences only. Engine access and preview work stay in PaintCoreClient.
class BrushLibrary final : public QObject {
    Q_OBJECT
    Q_PROPERTY(int writeBlend READ writeBlend WRITE setWriteBlend NOTIFY settingsChanged)
    Q_PROPERTY(int toolIsolation READ toolIsolation WRITE setToolIsolation NOTIFY settingsChanged)
    Q_PROPERTY(QVariantList presets READ presets NOTIFY presetsChanged)
    Q_PROPERTY(QVariantMap organization READ organization NOTIFY presetsChanged)
    Q_PROPERTY(QVariantList selectedBrushItems READ selectedBrushItems NOTIFY selectionChanged)
    Q_PROPERTY(bool multiBrushSelection READ multiBrushSelection NOTIFY selectionChanged)
    Q_PROPERTY(bool brushSelectionBlocksPainting READ brushSelectionBlocksPainting NOTIFY selectionChanged)
    Q_PROPERTY(bool brushSelectionHasSources READ brushSelectionHasSources NOTIFY selectionChanged)
    Q_PROPERTY(QString selectedId READ selectedId NOTIFY settingsChanged)
    Q_PROPERTY(QString selectedName READ selectedName NOTIFY settingsChanged)
    Q_PROPERTY(QString selectedPreview READ selectedPreview NOTIFY selectedPreviewChanged)
    Q_PROPERTY(QString previewMessage READ previewMessage NOTIFY selectedPreviewChanged)
    Q_PROPERTY(QVariantMap texture READ texture NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap dualBrush READ dualBrush NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap colorDynamics READ colorDynamics NOTIFY settingsChanged)
    Q_PROPERTY(bool colorDynamicsEnabled READ colorDynamicsEnabled WRITE setColorDynamicsEnabled NOTIFY settingsChanged)
    Q_PROPERTY(bool colorPerTip READ colorPerTip WRITE setColorPerTip NOTIFY settingsChanged)
    Q_PROPERTY(qreal foregroundBackgroundJitter READ foregroundBackgroundJitter WRITE setForegroundBackgroundJitter NOTIFY settingsChanged)
    Q_PROPERTY(qreal hueJitter READ hueJitter WRITE setHueJitter NOTIFY settingsChanged)
    Q_PROPERTY(qreal saturationJitter READ saturationJitter WRITE setSaturationJitter NOTIFY settingsChanged)
    Q_PROPERTY(qreal valueJitter READ valueJitter WRITE setValueJitter NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap shapeJitter READ shapeJitter NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap transferJitter READ transferJitter NOTIFY settingsChanged)
    Q_PROPERTY(bool shapeJitterEnabled READ shapeJitterEnabled WRITE setShapeJitterEnabled NOTIFY settingsChanged)
    Q_PROPERTY(qreal shapeSizeJitter READ shapeSizeJitter WRITE setShapeSizeJitter NOTIFY settingsChanged)
    Q_PROPERTY(qreal shapeMinimumSize READ shapeMinimumSize WRITE setShapeMinimumSize NOTIFY settingsChanged)
    Q_PROPERTY(qreal shapeAngleJitter READ shapeAngleJitter WRITE setShapeAngleJitter NOTIFY settingsChanged)
    Q_PROPERTY(qreal shapeRoundnessJitter READ shapeRoundnessJitter WRITE setShapeRoundnessJitter NOTIFY settingsChanged)
    Q_PROPERTY(qreal shapeMinimumRoundness READ shapeMinimumRoundness WRITE setShapeMinimumRoundness NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap scatterDynamics READ scatterDynamics NOTIFY settingsChanged)
    Q_PROPERTY(bool scatterEnabled READ scatterEnabled WRITE setScatterEnabled NOTIFY settingsChanged)
    Q_PROPERTY(bool scatterBothAxes READ scatterBothAxes WRITE setScatterBothAxes NOTIFY settingsChanged)
    Q_PROPERTY(qreal scatterAmount READ scatterAmount WRITE setScatterAmount NOTIFY settingsChanged)
    Q_PROPERTY(int scatterCount READ scatterCount WRITE setScatterCount NOTIFY settingsChanged)
    Q_PROPERTY(qreal scatterCountJitter READ scatterCountJitter WRITE setScatterCountJitter NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap tipDynamics READ tipDynamics NOTIFY settingsChanged)
    Q_PROPERTY(QString angleControl READ angleControl WRITE setAngleControl NOTIFY settingsChanged)
    Q_PROPERTY(bool tiltRoundness READ tiltRoundness WRITE setTiltRoundness NOTIFY settingsChanged)
    Q_PROPERTY(qreal minimumRoundness READ minimumRoundness WRITE setMinimumRoundness NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap wetMix READ wetMix NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap smoothing READ smoothing NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap pressureDynamics READ pressureDynamics NOTIFY settingsChanged)
    Q_PROPERTY(QString sourceResource READ sourceResource NOTIFY settingsChanged)
    Q_PROPERTY(QString maskResource READ maskResource NOTIFY settingsChanged)
    Q_PROPERTY(qreal radius READ radius WRITE setRadius NOTIFY settingsChanged)
    Q_PROPERTY(qreal opacity READ opacity WRITE setOpacity NOTIFY settingsChanged)
    Q_PROPERTY(qreal spacing READ spacing WRITE setSpacing NOTIFY settingsChanged)
    Q_PROPERTY(qreal hardness READ hardness WRITE setHardness NOTIFY settingsChanged)
    Q_PROPERTY(qreal roundness READ roundness WRITE setRoundness NOTIFY settingsChanged)
    Q_PROPERTY(qreal angle READ angle WRITE setAngle NOTIFY settingsChanged)
    Q_PROPERTY(qreal flow READ flow WRITE setFlow NOTIFY settingsChanged)
    Q_PROPERTY(bool pressureSize READ pressureSize WRITE setPressureSize NOTIFY settingsChanged)
    Q_PROPERTY(bool pressureOpacity READ pressureOpacity WRITE setPressureOpacity NOTIFY settingsChanged)
    Q_PROPERTY(bool pressureFlow READ pressureFlow WRITE setPressureFlow NOTIFY settingsChanged)
    Q_PROPERTY(bool cursorOutline READ cursorOutline WRITE setCursorOutline NOTIFY cursorChanged)
    Q_PROPERTY(bool cursorCenter READ cursorCenter WRITE setCursorCenter NOTIFY cursorChanged)
    Q_PROPERTY(bool cursorHoverOnly READ cursorHoverOnly WRITE setCursorHoverOnly NOTIFY cursorChanged)
    Q_PROPERTY(int cursorWidth READ cursorWidth WRITE setCursorWidth NOTIFY cursorChanged)
    Q_PROPERTY(int cursorBrightness READ cursorBrightness WRITE setCursorBrightness NOTIFY cursorChanged)
    Q_PROPERTY(bool cursorImage READ cursorImage WRITE setCursorImage NOTIFY cursorChanged)
    Q_PROPERTY(int cursorImageOpacity READ cursorImageOpacity WRITE setCursorImageOpacity NOTIFY cursorChanged)
    Q_PROPERTY(bool cursorComposite READ cursorComposite WRITE setCursorComposite NOTIFY cursorChanged)
    Q_PROPERTY(bool cursorSimple READ cursorSimple WRITE setCursorSimple NOTIFY cursorChanged)
    Q_PROPERTY(QString cursorColor READ cursorColor WRITE setCursorColor NOTIFY cursorChanged)
    Q_PROPERTY(QString cursorSecondaryColor READ cursorSecondaryColor WRITE setCursorSecondaryColor NOTIFY cursorChanged)
    Q_PROPERTY(QVariantList recentColors READ recentColors NOTIFY recentColorsChanged)
public:
    QVariantMap organization() const {return m_organization.toVariantMap();}
    bool cursorOutline() const {return m_cursorOutline;}
    bool cursorCenter() const {return m_cursorCenter;}
    bool cursorHoverOnly() const {return m_cursorHoverOnly;}
    int cursorWidth() const {return m_cursorWidth;}
    int cursorBrightness() const {return std::max(1,qRound(QColor(m_cursorColor).valueF()*100));}
    void setCursorBrightness(int value);
    bool cursorImage() const {return m_cursorImage;}
    int cursorImageOpacity() const {return m_cursorImageOpacity;}
    void setCursorImage(bool value);
    void setCursorImageOpacity(int value);
    bool cursorComposite() const {return m_cursorComposite;}
    void setCursorComposite(bool value);
    bool cursorSimple() const {return m_cursorSimple;}
    QString cursorColor() const {return m_cursorColor;}
    QString cursorSecondaryColor() const {return m_cursorSecondaryColor;}
    void setCursorSimple(bool value);
    void setCursorColor(const QString &value);
    void setCursorSecondaryColor(const QString &value);
    void setCursorOutline(bool value);
    void setCursorCenter(bool value);
    void setCursorHoverOnly(bool value);
    void setCursorWidth(int value);
    explicit BrushLibrary(QObject *parent=nullptr);
    int writeBlend() const {return m_writeBlend;}
    void setWriteBlend(int mode);
    QVariantList presets() const;
    // A method result is a QML value sequence; property-backed iteration re-reads the getter.
    Q_INVOKABLE QVariantList presetRows() const {return presets();}
    Q_INVOKABLE QVariantMap presetInfo(const QString &id) const;
    bool persistencePending() const {return m_persistTimer.isActive();}
    void flushPersistence();
    QString selectedId() const {return m_selected;}
    QVariantList selectedBrushItems() const;
    bool multiBrushSelection() const;
    bool brushSelectionBlocksPainting() const;
    bool brushSelectionHasSources() const;
    QString selectedName() const;
    QString selectedPreview() const;
    QString previewMessage() const;
    QVariantMap wetMix() const;
    Q_PROPERTY(QVariantMap strokeTaper READ strokeTaper NOTIFY settingsChanged)
    QVariantMap strokeTaper() const;
    Q_PROPERTY(QVariantMap strokeOutline READ strokeOutline NOTIFY settingsChanged)
    QVariantMap strokeOutline() const;
    Q_INVOKABLE bool setStrokeOutline(QVariantMap data);
    Q_INVOKABLE bool usePresetAsOutline(const QString &id);
    Q_INVOKABLE bool useTextureAsOutline(const QString &resource);
    Q_PROPERTY(bool dualExisting READ dualExisting WRITE setDualExisting NOTIFY settingsChanged)
    Q_PROPERTY(QString dualSourceName READ dualSourceName NOTIFY settingsChanged)
    bool dualExisting() const;
    QString dualSourceName() const;
    void setDualExisting(bool existing);
    Q_INVOKABLE bool usePresetAsSecondary(const QString &id);
    Q_PROPERTY(QVariantMap airbrush READ airbrush NOTIFY settingsChanged)
    QVariantMap airbrush() const;
    Q_PROPERTY(QVariantMap speedDynamics READ speedDynamics NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap tangentDynamics READ tangentDynamics NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap distanceDynamics READ distanceDynamics NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap tiltDynamics READ tiltDynamics NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap rotationDynamics READ rotationDynamics NOTIFY settingsChanged)
    Q_PROPERTY(QVariantMap directionDynamics READ directionDynamics NOTIFY settingsChanged)
    QVariantMap speedDynamics() const;
    Q_PROPERTY(bool speedDynamicsEnabled READ speedDynamicsEnabled NOTIFY settingsChanged)
    bool speedDynamicsEnabled() const;
    Q_PROPERTY(qreal speedDynamicsMaxSpeed READ speedDynamicsMaxSpeed NOTIFY settingsChanged)
    qreal speedDynamicsMaxSpeed() const;
    QVariantMap speedSize() const;
    Q_PROPERTY(QVariantList speedSizeCurve READ speedSizeCurve NOTIFY settingsChanged)
    QVariantList speedSizeCurve() const;
    Q_PROPERTY(qreal speedSizeMinimum READ speedSizeMinimum NOTIFY settingsChanged)
    qreal speedSizeMinimum() const;
    QVariantMap speedFlow() const;
    Q_PROPERTY(QVariantList speedFlowCurve READ speedFlowCurve NOTIFY settingsChanged)
    QVariantList speedFlowCurve() const;
    Q_PROPERTY(qreal speedFlowMinimum READ speedFlowMinimum NOTIFY settingsChanged)
    qreal speedFlowMinimum() const;
    QVariantMap speedOpacity() const;
    Q_PROPERTY(QVariantList speedOpacityCurve READ speedOpacityCurve NOTIFY settingsChanged)
    QVariantList speedOpacityCurve() const;
    Q_PROPERTY(qreal speedOpacityMinimum READ speedOpacityMinimum NOTIFY settingsChanged)
    qreal speedOpacityMinimum() const;
    QVariantMap tangentDynamics() const;
    Q_PROPERTY(bool tangentDynamicsEnabled READ tangentDynamicsEnabled NOTIFY settingsChanged)
    bool tangentDynamicsEnabled() const;
    Q_PROPERTY(QVariantList tangentSizeCurve READ tangentSizeCurve NOTIFY settingsChanged)
    QVariantList tangentSizeCurve() const;
    Q_PROPERTY(qreal tangentSizeMinimum READ tangentSizeMinimum NOTIFY settingsChanged)
    qreal tangentSizeMinimum() const;
    Q_PROPERTY(QVariantList tangentFlowCurve READ tangentFlowCurve NOTIFY settingsChanged)
    QVariantList tangentFlowCurve() const;
    Q_PROPERTY(qreal tangentFlowMinimum READ tangentFlowMinimum NOTIFY settingsChanged)
    qreal tangentFlowMinimum() const;
    Q_PROPERTY(QVariantList tangentOpacityCurve READ tangentOpacityCurve NOTIFY settingsChanged)
    QVariantList tangentOpacityCurve() const;
    Q_PROPERTY(qreal tangentOpacityMinimum READ tangentOpacityMinimum NOTIFY settingsChanged)
    qreal tangentOpacityMinimum() const;
    QVariantMap distanceDynamics() const;
    Q_PROPERTY(bool distanceDynamicsEnabled READ distanceDynamicsEnabled NOTIFY settingsChanged)
    bool distanceDynamicsEnabled() const;
    Q_PROPERTY(qreal distanceDynamicsMaxDistance READ distanceDynamicsMaxDistance NOTIFY settingsChanged)
    qreal distanceDynamicsMaxDistance() const;
    Q_PROPERTY(QVariantList distanceSizeCurve READ distanceSizeCurve NOTIFY settingsChanged)
    QVariantList distanceSizeCurve() const;
    Q_PROPERTY(qreal distanceSizeMinimum READ distanceSizeMinimum NOTIFY settingsChanged)
    qreal distanceSizeMinimum() const;
    Q_PROPERTY(QVariantList distanceFlowCurve READ distanceFlowCurve NOTIFY settingsChanged)
    QVariantList distanceFlowCurve() const;
    Q_PROPERTY(qreal distanceFlowMinimum READ distanceFlowMinimum NOTIFY settingsChanged)
    qreal distanceFlowMinimum() const;
    Q_PROPERTY(QVariantList distanceOpacityCurve READ distanceOpacityCurve NOTIFY settingsChanged)
    QVariantList distanceOpacityCurve() const;
    Q_PROPERTY(qreal distanceOpacityMinimum READ distanceOpacityMinimum NOTIFY settingsChanged)
    qreal distanceOpacityMinimum() const;
    QVariantMap tiltDynamics() const;
    Q_PROPERTY(bool tiltDynamicsEnabled READ tiltDynamicsEnabled NOTIFY settingsChanged)
    bool tiltDynamicsEnabled() const;
    Q_PROPERTY(QVariantList tiltSizeCurve READ tiltSizeCurve NOTIFY settingsChanged)
    QVariantList tiltSizeCurve() const;
    Q_PROPERTY(qreal tiltSizeMinimum READ tiltSizeMinimum NOTIFY settingsChanged)
    qreal tiltSizeMinimum() const;
    Q_PROPERTY(QVariantList tiltFlowCurve READ tiltFlowCurve NOTIFY settingsChanged)
    QVariantList tiltFlowCurve() const;
    Q_PROPERTY(qreal tiltFlowMinimum READ tiltFlowMinimum NOTIFY settingsChanged)
    qreal tiltFlowMinimum() const;
    Q_PROPERTY(QVariantList tiltOpacityCurve READ tiltOpacityCurve NOTIFY settingsChanged)
    QVariantList tiltOpacityCurve() const;
    Q_PROPERTY(qreal tiltOpacityMinimum READ tiltOpacityMinimum NOTIFY settingsChanged)
    qreal tiltOpacityMinimum() const;
    QVariantMap rotationDynamics() const;
    Q_PROPERTY(bool rotationDynamicsEnabled READ rotationDynamicsEnabled NOTIFY settingsChanged)
    bool rotationDynamicsEnabled() const;
    Q_PROPERTY(QVariantList rotationSizeCurve READ rotationSizeCurve NOTIFY settingsChanged)
    QVariantList rotationSizeCurve() const;
    Q_PROPERTY(qreal rotationSizeMinimum READ rotationSizeMinimum NOTIFY settingsChanged)
    qreal rotationSizeMinimum() const;
    Q_PROPERTY(QVariantList rotationFlowCurve READ rotationFlowCurve NOTIFY settingsChanged)
    QVariantList rotationFlowCurve() const;
    Q_PROPERTY(qreal rotationFlowMinimum READ rotationFlowMinimum NOTIFY settingsChanged)
    qreal rotationFlowMinimum() const;
    Q_PROPERTY(QVariantList rotationOpacityCurve READ rotationOpacityCurve NOTIFY settingsChanged)
    QVariantList rotationOpacityCurve() const;
    Q_PROPERTY(qreal rotationOpacityMinimum READ rotationOpacityMinimum NOTIFY settingsChanged)
    qreal rotationOpacityMinimum() const;
    QVariantMap directionDynamics() const;
    Q_PROPERTY(bool directionDynamicsEnabled READ directionDynamicsEnabled NOTIFY settingsChanged)
    bool directionDynamicsEnabled() const;
    Q_PROPERTY(QVariantList directionSizeCurve READ directionSizeCurve NOTIFY settingsChanged)
    QVariantList directionSizeCurve() const;
    Q_PROPERTY(qreal directionSizeMinimum READ directionSizeMinimum NOTIFY settingsChanged)
    qreal directionSizeMinimum() const;
    Q_PROPERTY(QVariantList directionFlowCurve READ directionFlowCurve NOTIFY settingsChanged)
    QVariantList directionFlowCurve() const;
    Q_PROPERTY(qreal directionFlowMinimum READ directionFlowMinimum NOTIFY settingsChanged)
    qreal directionFlowMinimum() const;
    Q_PROPERTY(QVariantList directionOpacityCurve READ directionOpacityCurve NOTIFY settingsChanged)
    QVariantList directionOpacityCurve() const;
    Q_PROPERTY(qreal directionOpacityMinimum READ directionOpacityMinimum NOTIFY settingsChanged)
    qreal directionOpacityMinimum() const;
    Q_INVOKABLE bool setStrokeTaper(QVariantMap data);
    Q_INVOKABLE bool setAirbrush(QVariantMap data);
    Q_INVOKABLE bool setSpeedDynamics(QVariantMap data);
    Q_INVOKABLE bool setSpeedDynamicsEnabled(bool enabled);
    Q_INVOKABLE bool setSpeedDynamicsMaxSpeed(qreal maximum);
    Q_INVOKABLE bool setSpeedControl(const QString &target, QVariantList points, qreal minimum, const QVariantList &nodeModes = {});
    Q_INVOKABLE bool resetSpeedControl(const QString &target);
    Q_INVOKABLE bool setTangentDynamics(QVariantMap data);
    Q_INVOKABLE bool setTangentDynamicsEnabled(bool enabled);
    Q_INVOKABLE bool setTangentControl(const QString &target, QVariantList points, qreal minimum, const QVariantList &nodeModes = {});
    Q_INVOKABLE bool resetTangentControl(const QString &target);
    Q_INVOKABLE bool setDistanceDynamics(QVariantMap data);
    Q_INVOKABLE bool setDistanceDynamicsEnabled(bool enabled);
    Q_INVOKABLE bool setDistanceDynamicsMaxDistance(qreal maximum);
    Q_INVOKABLE bool setDistanceControl(const QString &target, QVariantList points, qreal minimum, const QVariantList &nodeModes = {});
    Q_INVOKABLE bool resetDistanceControl(const QString &target);
    Q_INVOKABLE bool setTiltDynamics(QVariantMap data);
    Q_INVOKABLE bool setTiltDynamicsEnabled(bool enabled);
    Q_INVOKABLE bool setTiltControl(const QString &target, QVariantList points, qreal minimum, const QVariantList &nodeModes = {});
    Q_INVOKABLE bool resetTiltControl(const QString &target);
    Q_INVOKABLE bool setRotationDynamics(QVariantMap data);
    Q_INVOKABLE bool setRotationDynamicsEnabled(bool enabled);
    Q_INVOKABLE bool setRotationControl(const QString &target, QVariantList points, qreal minimum, const QVariantList &nodeModes = {});
    Q_INVOKABLE bool resetRotationControl(const QString &target);
    Q_INVOKABLE bool setDirectionDynamics(QVariantMap data);
    Q_INVOKABLE bool setDirectionDynamicsEnabled(bool enabled);
    Q_INVOKABLE bool setDirectionControl(const QString &target, QVariantList points, qreal minimum, const QVariantList &nodeModes = {});
    Q_INVOKABLE bool resetDirectionControl(const QString &target);
    Q_PROPERTY(QVariantMap fadeDynamics READ fadeDynamics NOTIFY settingsChanged)
    QVariantMap fadeDynamics() const;
    Q_PROPERTY(bool fadeDynamicsEnabled READ fadeDynamicsEnabled NOTIFY settingsChanged)
    bool fadeDynamicsEnabled() const;
    Q_PROPERTY(int fadeSteps READ fadeSteps NOTIFY settingsChanged)
    int fadeSteps() const;
    Q_PROPERTY(QVariantList fadeSizeCurve READ fadeSizeCurve NOTIFY settingsChanged)
    QVariantList fadeSizeCurve() const;
    Q_PROPERTY(qreal fadeSizeMinimum READ fadeSizeMinimum NOTIFY settingsChanged)
    qreal fadeSizeMinimum() const;
    Q_PROPERTY(QVariantList fadeFlowCurve READ fadeFlowCurve NOTIFY settingsChanged)
    QVariantList fadeFlowCurve() const;
    Q_PROPERTY(qreal fadeFlowMinimum READ fadeFlowMinimum NOTIFY settingsChanged)
    qreal fadeFlowMinimum() const;
    Q_PROPERTY(QVariantList fadeOpacityCurve READ fadeOpacityCurve NOTIFY settingsChanged)
    QVariantList fadeOpacityCurve() const;
    Q_PROPERTY(qreal fadeOpacityMinimum READ fadeOpacityMinimum NOTIFY settingsChanged)
    qreal fadeOpacityMinimum() const;
    Q_INVOKABLE bool setFadeDynamics(QVariantMap data);
    Q_INVOKABLE bool setFadeDynamicsEnabled(bool enabled);
    Q_INVOKABLE bool setFadeSteps(int steps);
    Q_INVOKABLE bool setFadeControl(const QString &target, QVariantList points, qreal minimum, const QVariantList &nodeModes = {});
    Q_INVOKABLE bool resetFadeControl(const QString &target);
    Q_PROPERTY(QVariantMap dynamicsSourceMatrix READ dynamicsSourceMatrix NOTIFY settingsChanged)
    QVariantMap dynamicsSourceMatrix() const;
    Q_PROPERTY(QString dynamicsSizeSource READ dynamicsSizeSource NOTIFY settingsChanged)
    QString dynamicsSizeSource() const;
    Q_PROPERTY(QString dynamicsFlowSource READ dynamicsFlowSource NOTIFY settingsChanged)
    QString dynamicsFlowSource() const;
    Q_PROPERTY(QString dynamicsOpacitySource READ dynamicsOpacitySource NOTIFY settingsChanged)
    QString dynamicsOpacitySource() const;
    Q_INVOKABLE bool setDynamicsSource(const QString &target, const QString &source);
    Q_INVOKABLE bool setWetMix(QVariantMap data);
    QVariantMap smoothing() const;
    Q_INVOKABLE bool setSmoothing(QVariantMap data);
    QVariantMap pressureDynamics() const;
    QVariantMap texture() const;
    QVariantMap dualBrush() const;
    Q_INVOKABLE bool setDualBrush(QVariantMap data);
    Q_INVOKABLE bool setDualPressureControl(const QString &target,const QVariantList &points,qreal minimum, const QVariantList &nodeModes = {});
    Q_INVOKABLE void resetDualPressureControl(const QString &target);
    bool setDualResource(const QString &id,const QString &resource);
    Q_INVOKABLE void useProceduralDual() {setDualResource(m_selected,{});}
    Q_INVOKABLE bool setTexture(QVariantMap data);
    bool setTextureResource(const QString &id,const QString &resource);
    bool setOutlineTextureResource(const QString &id,const QString &resource,bool selfBrush=false);
    Q_INVOKABLE void removeTexture() {setTextureResource(m_selected,{});}
    QVariantMap colorDynamics() const;
    Q_INVOKABLE bool setColorDynamics(QVariantMap data);
    bool colorDynamicsEnabled() const;
    void setColorDynamicsEnabled(bool value);
    bool colorPerTip() const;
    void setColorPerTip(bool value);
    qreal foregroundBackgroundJitter() const;
    void setForegroundBackgroundJitter(qreal value);
    qreal hueJitter() const;
    void setHueJitter(qreal value);
    qreal saturationJitter() const;
    void setSaturationJitter(qreal value);
    qreal valueJitter() const;
    void setValueJitter(qreal value);
    QVariantMap shapeJitter() const;
    QVariantMap transferJitter() const;
    Q_INVOKABLE bool setTransferJitter(QVariantMap data);
    Q_INVOKABLE bool setShapeJitter(QVariantMap data);
    bool shapeJitterEnabled() const;
    void setShapeJitterEnabled(bool value);
    qreal shapeSizeJitter() const;
    void setShapeSizeJitter(qreal value);
    qreal shapeMinimumSize() const;
    void setShapeMinimumSize(qreal value);
    qreal shapeAngleJitter() const;
    void setShapeAngleJitter(qreal value);
    qreal shapeRoundnessJitter() const;
    void setShapeRoundnessJitter(qreal value);
    qreal shapeMinimumRoundness() const;
    void setShapeMinimumRoundness(qreal value);
    QVariantMap scatterDynamics() const;
    bool scatterEnabled() const;
    bool scatterBothAxes() const;
    qreal scatterAmount() const;
    int scatterCount() const;
    qreal scatterCountJitter() const;
    void setScatterEnabled(bool enabled);
    void setScatterBothAxes(bool enabled);
    void setScatterAmount(qreal amount);
    void setScatterCount(int count);
    void setScatterCountJitter(qreal jitter);
    Q_INVOKABLE bool setScatterDynamics(QVariantMap data);
    QVariantMap tipDynamics() const;
    QString angleControl() const;
    bool tiltRoundness() const;
    qreal minimumRoundness() const;
    void setAngleControl(const QString &control);
    void setTiltRoundness(bool enabled);
    void setMinimumRoundness(qreal minimum);
    Q_INVOKABLE bool setPressureControl(const QString &target,const QVariantList &points,qreal minimum, const QVariantList &nodeModes = {});
    Q_INVOKABLE void resetPressureControl(const QString &target);
    QString sourceResource() const;
    QString maskResource() const;
    bool setMaskResource(const QString &id,const QString &resource);
    Q_INVOKABLE void useProceduralTip() {setMaskResource(m_selected,{});}
    qreal radius() const;
    qreal opacity() const;
    qreal spacing() const;
    qreal hardness() const;
    qreal roundness() const;
    qreal angle() const;
    qreal flow() const;
    bool pressureSize() const;
    bool pressureFlow() const;
    bool pressureOpacity() const;
    void setPressureOpacity(bool enabled);
    QVariantMap tipSettings() const;
    QByteArray exportPackage(bool includeSource=false) const;
    QByteArray exportAllPackage(bool includeSource=false) const;
    QByteArray exportSelectionPackage(bool includeSource=false) const;
    bool importPackage(const QByteArray &json,const QString &groupName={});
    bool importAbr(const QByteArray &json,const QString &groupName={});
    static int outlineStateVersion(const QVariantMap &outline);
    static int textureStateVersion(const QVariantMap &texture);
    void setHardness(qreal value);
    void setRoundness(qreal value);
    void setAngle(qreal value);
    void setFlow(qreal value);
    void setPressureSize(bool value);
    void setPressureFlow(bool value);
    QVariantList recentColors() const;
    void setRadius(qreal value);
    void setEraser(bool eraser);
    int toolIsolation() const {return m_toolIsolation;}
    int toolContext() const {return m_toolContext;}
    void setToolIsolation(int mode);
    void setToolContext(int tool,bool persist=true);
    void setOpacity(qreal value);
    void setSpacing(qreal value);
    Q_INVOKABLE bool select(const QString &id);
    Q_INVOKABLE bool selectBrushItems(const QVariantList &items,const QString &activePreset={});
    Q_INVOKABLE void clearBrushSelection();
    Q_INVOKABLE bool groupSelectedItems(const QString &name);
    Q_INVOKABLE bool moveSelectedItemsToGroupAt(const QString &parentId,int index);
    Q_INVOKABLE bool useColor(const QString &color);
    Q_INVOKABLE QString saveCopy(const QString &name);
    Q_INVOKABLE QString createOutlinePreset(bool fixedWidth);
    Q_INVOKABLE QString createGroup(const QString &name,const QString &parentId = {});
    Q_INVOKABLE bool renameGroup(const QString &id,const QString &name);
    Q_INVOKABLE bool movePresetToGroup(const QString &presetId,const QString &parentId = {});
    Q_INVOKABLE bool moveGroupToGroup(const QString &groupId,const QString &parentId = {});
    Q_INVOKABLE bool movePresetToGroupAt(const QString &presetId,const QString &parentId,int index);
    Q_INVOKABLE bool moveGroupToGroupAt(const QString &groupId,const QString &parentId,int index);
    Q_INVOKABLE bool ungroup(const QString &id);
    Q_INVOKABLE bool removeGroup(const QString &id,bool cascade = false);
    Q_INVOKABLE bool rename(const QString &id,const QString &name);
    Q_INVOKABLE bool remove(const QString &id);
    Q_INVOKABLE bool resetPreset();
    Q_INVOKABLE bool restoreSourceBaseline(const QVariantMap &info);
    Q_INVOKABLE void requestPreview(const QString &id);
    Q_INVOKABLE void cancelPreview(const QString &id);
    Q_INVOKABLE void retryPreview();
    bool restore(const QByteArray &json,bool preserveSelection=false);
    QByteArray snapshot() const;
    QByteArray catalogSnapshot() const;
    bool applyNativeCatalog(const QByteArray &json);
    QColor foregroundColor() const {return m_foregroundColor;}
    QColor backgroundColor() const {return m_backgroundColor;}
    void setActiveColors(const QColor &foreground,const QColor &background);
    void acceptPreview(const QString &id,quint64 token,const QString &image);
signals:
    void activeColorsChanged();
    void cursorChanged();
    void settingsChanged();
    void presetsChanged();
    void presetChanged(QString id);
    void selectedPreviewChanged();
    void previewChanged(QString id,QString image);
    void selectionChanged();
    void recentColorsChanged();
    void persistRequested(QByteArray json);
    void previewRequested(QString id,quint64 token,qreal radius,qreal opacity,qreal spacing,QVariantMap tip);
    void previewCancelled(QString id,quint64 token);
private:
    void cursorPreferenceChanged();
    void setTipDynamics(QVariantMap data);
    bool m_cursorOutline=true,m_cursorCenter=false,m_cursorHoverOnly=false;
    int m_cursorWidth=0,m_cursorImageOpacity=25;
    bool m_cursorImage=false;
    bool m_cursorSimple=false,m_cursorComposite=false;
    QString m_cursorColor="#000000",m_cursorSecondaryColor="#ffffff";
    struct Preset {QString id,name; qreal radius=12,opacity=1,spacing=.15; bool builtin=false;QString preview;quint64 token=1,requested=0;QString error;
        qreal hardness=1,roundness=1,angle=0,flow=1;bool pressureSize=true,pressureFlow=true;QString maskResource,sourceResource;QVariantMap pressure,device,scatter,shape,transfer,color,texture,dual,smoothing,wet,taper,airbrush,speed,tangent,distance,tilt,rotation,direction,matrix,fade,outline;bool dualExisting=false;QString dualSourceName;};
    static QJsonObject presetRow(const Preset &preset,int &version);
    struct ToolProfile {QHash<QString,Preset> presets;QString selected="round-pressure";int writeBlend=0;};
    QJsonObject toolIsolationState() const;
    static bool parseToolIsolation(const QJsonValue &value,int &mode,int &context,std::array<ToolProfile,3> &profiles);
    void captureToolProfile();
    int m_toolIsolation=0,m_toolContext=0;
    std::array<ToolProfile,3> m_toolProfiles;
    static QJsonObject exportPreset(const Preset &preset,bool includeSource,const QString &id);
    QVariantMap tangentControl(const QString &target) const;
    QVariantMap distanceControl(const QString &target) const;
    QVariantMap tiltControl(const QString &target) const;
    QVariantMap rotationControl(const QString &target) const;
    QVariantMap directionControl(const QString &target) const;
    int index(const QString &id) const;
    void update(int field,qreal value);
    void changed(bool structure = false, const QString &id = {});
    void queuePersistence(bool catalog=true);
    QTimer m_persistTimer;
    mutable QByteArray m_snapshotCache,m_catalogCache;
    bool applyOrganization(const QJsonObject &candidate);
    QList<Preset> m_presets;
    QJsonObject m_organization;
    QHash<QString,Preset> m_resetPresets;
    QString m_selected="round-pressure";
    QJsonArray m_selectedItems;
    // Most recently used colours first; the colour panel shows them as its swatch row.
    QStringList m_recentColors;
    int m_writeBlend=0;
    QColor m_foregroundColor{"#2ea99d"},m_backgroundColor=Qt::white;
    quint64 m_nextToken=16;
};
