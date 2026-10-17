#pragma once
#include <QObject>
#include <QColor>
#include <QJsonObject>
#include <QHash>
#include <QVariantList>
#include <QStringList>

// UI preferences only. Engine access and preview work stay in PaintCoreClient.
class BrushLibrary final : public QObject {
    Q_OBJECT
    Q_PROPERTY(int writeBlend READ writeBlend WRITE setWriteBlend NOTIFY settingsChanged)
    Q_PROPERTY(QVariantList presets READ presets NOTIFY presetsChanged)
    Q_PROPERTY(QString selectedId READ selectedId NOTIFY settingsChanged)
    Q_PROPERTY(QString selectedName READ selectedName NOTIFY settingsChanged)
    Q_PROPERTY(QString selectedPreview READ selectedPreview NOTIFY presetsChanged)
    Q_PROPERTY(QString previewMessage READ previewMessage NOTIFY presetsChanged)
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
    Q_PROPERTY(bool cursorImage READ cursorImage WRITE setCursorImage NOTIFY cursorChanged)
    Q_PROPERTY(int cursorImageOpacity READ cursorImageOpacity WRITE setCursorImageOpacity NOTIFY cursorChanged)
    Q_PROPERTY(bool cursorComposite READ cursorComposite WRITE setCursorComposite NOTIFY cursorChanged)
    Q_PROPERTY(bool cursorSimple READ cursorSimple WRITE setCursorSimple NOTIFY cursorChanged)
    Q_PROPERTY(QString cursorColor READ cursorColor WRITE setCursorColor NOTIFY cursorChanged)
    Q_PROPERTY(QString cursorSecondaryColor READ cursorSecondaryColor WRITE setCursorSecondaryColor NOTIFY cursorChanged)
    Q_PROPERTY(QVariantList recentColors READ recentColors NOTIFY recentColorsChanged)
public:
    bool cursorOutline() const {return m_cursorOutline;}
    bool cursorCenter() const {return m_cursorCenter;}
    bool cursorHoverOnly() const {return m_cursorHoverOnly;}
    int cursorWidth() const {return m_cursorWidth;}
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
    QString selectedId() const {return m_selected;}
    QString selectedName() const;
    QString selectedPreview() const;
    QString previewMessage() const;
    QVariantMap wetMix() const;
    Q_PROPERTY(QVariantMap strokeTaper READ strokeTaper NOTIFY settingsChanged)
    QVariantMap strokeTaper() const;
    Q_INVOKABLE bool setStrokeTaper(QVariantMap data);
    Q_INVOKABLE bool setWetMix(QVariantMap data);
    QVariantMap smoothing() const;
    Q_INVOKABLE bool setSmoothing(QVariantMap data);
    QVariantMap pressureDynamics() const;
    QVariantMap texture() const;
    QVariantMap dualBrush() const;
    Q_INVOKABLE bool setDualBrush(QVariantMap data);
    Q_INVOKABLE bool setDualPressureControl(const QString &target,const QVariantList &points,qreal minimum);
    Q_INVOKABLE void resetDualPressureControl(const QString &target);
    bool setDualResource(const QString &id,const QString &resource);
    Q_INVOKABLE void useProceduralDual() {setDualResource(m_selected,{});}
    Q_INVOKABLE bool setTexture(QVariantMap data);
    bool setTextureResource(const QString &id,const QString &resource);
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
    Q_INVOKABLE bool setPressureControl(const QString &target,const QVariantList &points,qreal minimum);
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
    bool importPackage(const QByteArray &json);
    bool importAbr(const QByteArray &json);
    void setHardness(qreal value);
    void setRoundness(qreal value);
    void setAngle(qreal value);
    void setFlow(qreal value);
    void setPressureSize(bool value);
    void setPressureFlow(bool value);
    QVariantList recentColors() const;
    void setRadius(qreal value);
    void setEraser(bool eraser);
    void setOpacity(qreal value);
    void setSpacing(qreal value);
    Q_INVOKABLE bool select(const QString &id);
    Q_INVOKABLE bool useColor(const QString &color);
    Q_INVOKABLE QString saveCopy(const QString &name);
    Q_INVOKABLE QString createOutlinePreset(bool fixedWidth);
    Q_INVOKABLE bool rename(const QString &id,const QString &name);
    Q_INVOKABLE bool remove(const QString &id);
    Q_INVOKABLE bool resetPreset();
    Q_INVOKABLE void requestPreview(const QString &id);
    Q_INVOKABLE void retryPreview();
    bool restore(const QByteArray &json);
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
    void recentColorsChanged();
    void persistRequested(QByteArray json);
    void previewRequested(QString id,quint64 token,qreal radius,qreal opacity,qreal spacing,QVariantMap tip);
private:
    void cursorPreferenceChanged();
    void setTipDynamics(QVariantMap data);
    bool m_cursorOutline=true,m_cursorCenter=false,m_cursorHoverOnly=false;
    int m_cursorWidth=1,m_cursorImageOpacity=25;
    bool m_cursorImage=false;
    bool m_cursorSimple=false,m_cursorComposite=false;
    QString m_cursorColor="#000000",m_cursorSecondaryColor="#ffffff";
    struct Preset {QString id,name; qreal radius=12,opacity=1,spacing=.15; bool builtin=false;QString preview;quint64 token=1,requested=0;QString error;
        qreal hardness=1,roundness=1,angle=0,flow=1;bool pressureSize=true,pressureFlow=true;QString maskResource,sourceResource;QVariantMap pressure,device,scatter,shape,color,texture,dual,smoothing,wet,taper;};
    static QJsonObject presetRow(const Preset &preset,int &version);
    int index(const QString &id) const;
    void update(int field,qreal value);
    void changed();
    QList<Preset> m_presets;
    QHash<QString,Preset> m_resetPresets;
    QString m_selected="round-pressure";
    // Most recently used colours first; the colour panel shows them as its swatch row.
    QStringList m_recentColors;
    int m_writeBlend=0;
    bool m_eraser=false;
    QColor m_foregroundColor{"#2ea99d"},m_backgroundColor=Qt::white;
    quint64 m_nextToken=16;
};
