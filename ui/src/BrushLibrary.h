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
    void previewRequested(QString id,quint64 token,qreal radius,qreal opacity,qreal spacing);
private:
    struct Preset {QString id,name; qreal radius=12,opacity=1,spacing=.15; bool builtin=false;QString preview;quint64 token=1,requested=0;QString error;};
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
