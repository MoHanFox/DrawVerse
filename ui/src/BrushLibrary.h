#pragma once
#include <QObject>
#include <QVariantList>
#include <QStringList>
#include <QHash>

// UI preferences only. Engine access and preview work stay in PaintCoreClient.
class BrushLibrary final : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantList presets READ presets NOTIFY presetsChanged)
    Q_PROPERTY(QString selectedId READ selectedId NOTIFY settingsChanged)
    Q_PROPERTY(QString selectedName READ selectedName NOTIFY settingsChanged)
    Q_PROPERTY(QString selectedPreview READ selectedPreview NOTIFY presetsChanged)
    Q_PROPERTY(QString previewMessage READ previewMessage NOTIFY presetsChanged)
    Q_PROPERTY(qreal radius READ radius WRITE setRadius NOTIFY settingsChanged)
    Q_PROPERTY(qreal opacity READ opacity WRITE setOpacity NOTIFY settingsChanged)
    Q_PROPERTY(qreal spacing READ spacing WRITE setSpacing NOTIFY settingsChanged)
    Q_PROPERTY(qreal hardness READ hardness WRITE setHardness NOTIFY settingsChanged)
    Q_PROPERTY(qreal roundness READ roundness WRITE setRoundness NOTIFY settingsChanged)
    Q_PROPERTY(qreal angle READ angle WRITE setAngle NOTIFY settingsChanged)
    Q_PROPERTY(qreal flow READ flow WRITE setFlow NOTIFY settingsChanged)
    Q_PROPERTY(bool pressureSize READ pressureSize WRITE setPressureSize NOTIFY settingsChanged)
    Q_PROPERTY(bool pressureFlow READ pressureFlow WRITE setPressureFlow NOTIFY settingsChanged)
    Q_PROPERTY(QVariantList recentColors READ recentColors NOTIFY recentColorsChanged)
public:
    explicit BrushLibrary(QObject *parent=nullptr);
    QVariantList presets() const;
    QString selectedId() const {return m_selected;}
    QString selectedName() const;
    QString selectedPreview() const;
    QString previewMessage() const;
    qreal radius() const;
    qreal opacity() const;
    qreal spacing() const;
    qreal hardness() const;
    qreal roundness() const;
    qreal angle() const;
    qreal flow() const;
    bool pressureSize() const;
    bool pressureFlow() const;
    QVariantMap tipSettings() const;
    QByteArray exportPackage() const;
    bool importPackage(const QByteArray &json);
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
    Q_INVOKABLE bool rename(const QString &id,const QString &name);
    Q_INVOKABLE bool remove(const QString &id);
    Q_INVOKABLE void requestPreview(const QString &id);
    Q_INVOKABLE void retryPreview();
    bool restore(const QByteArray &json);
    QByteArray snapshot() const;
    void acceptPreview(const QString &id,quint64 token,const QString &image);
signals:
    void settingsChanged();
    void presetsChanged();
    void recentColorsChanged();
    void persistRequested(QByteArray json);
    void previewRequested(QString id,quint64 token,qreal radius,qreal opacity,qreal spacing,QVariantMap tip);
private:
    struct Preset {QString id,name; qreal radius=12,opacity=1,spacing=.15; bool builtin=false;QString preview;quint64 token=1,requested=0;QString error;
        qreal hardness=1,roundness=1,angle=0,flow=1;bool pressureSize=true,pressureFlow=true;};
    int index(const QString &id) const;
    void update(int field,qreal value);
    void changed();
    QList<Preset> m_presets;
    QString m_selected="round-pressure";
    qreal m_toolRadii[2]={12,12};
    // Most recently used colours first; the colour panel shows them as its swatch row.
    QStringList m_recentColors;
    bool m_eraser=false;
    quint64 m_nextToken=16;
};
