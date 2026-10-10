#pragma once
#include "BrushLibrary.h"
#include <QColor>
#include <QImage>
#include <QObject>
#include <QPointF>
#include <QRectF>
#include <QSize>
#include <QSet>
#include <QThread>
#include <QUrl>
#include <QVariantList>
#include <memory>

struct InputSample {
    QPointF position;
    float pressure = 1;
    float tiltX = 0, tiltY = 0, rotation = 0, tangentialPressure = 0;
    quint32 buttons = 0, capabilities = 0, tool = 3;
    quint64 timestamp = 0;
};
class BackendWorker;
struct BackendConnection;

// The only UI-facing core service. Owns no model pointers on the GUI thread.
class PaintCoreClient final : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantMap storageSettings READ storageSettings NOTIFY storageChanged)
    Q_PROPERTY(QVariantMap activeStorageSettings READ activeStorageSettings NOTIFY storageChanged)
    Q_PROPERTY(QVariantMap storageInfo READ storageInfo NOTIFY storageChanged)
    Q_PROPERTY(bool storageBusy READ storageBusy NOTIFY storageChanged)
    Q_PROPERTY(QString storageMessage READ storageMessage NOTIFY storageChanged)
    Q_PROPERTY(bool ready READ ready NOTIFY stateChanged)
    Q_PROPERTY(QVariantList historyEntries READ historyEntries NOTIFY historyChanged)
    Q_PROPERTY(bool drawing READ drawing NOTIFY stateChanged)
    Q_PROPERTY(bool closing READ closing NOTIFY stateChanged)
    Q_PROPERTY(bool modified READ modified NOTIFY stateChanged)
    Q_PROPERTY(bool fileBusy READ fileBusy NOTIFY stateChanged)
    Q_PROPERTY(QString documentName READ documentName NOTIFY stateChanged)
    Q_PROPERTY(QUrl documentUrl READ documentUrl NOTIFY stateChanged)
    Q_PROPERTY(int documentWidth READ documentWidth NOTIFY stateChanged)
    Q_PROPERTY(int documentHeight READ documentHeight NOTIFY stateChanged)
    Q_PROPERTY(int undoDepth READ undoDepth NOTIFY stateChanged)
    Q_PROPERTY(int redoDepth READ redoDepth NOTIFY stateChanged)
    Q_PROPERTY(quint64 activeLayer READ activeLayer NOTIFY stateChanged)
    Q_PROPERTY(QVariantList layers READ layers NOTIFY layersChanged)
    // Panel-level multi-selection. Not document state: never written to history or disk.
    Q_PROPERTY(QVariantList selectedLayers READ selectedLayers NOTIFY selectedLayersChanged)
    Q_PROPERTY(QVariantList collapsedGroups READ collapsedGroups NOTIFY groupExpansionChanged)
    Q_PROPERTY(QString lastError READ lastError NOTIFY errorChanged)
    Q_PROPERTY(QColor secondaryBrushColor READ secondaryBrushColor WRITE setSecondaryBrushColor NOTIFY brushChanged)
    Q_PROPERTY(QColor brushColor READ brushColor WRITE setBrushColor NOTIFY brushChanged)
    Q_PROPERTY(qreal brushRadius READ brushRadius WRITE setBrushRadius NOTIFY brushChanged)
    Q_PROPERTY(qreal brushOpacity READ brushOpacity WRITE setBrushOpacity NOTIFY brushChanged)
    Q_PROPERTY(qreal brushSpacing READ brushSpacing WRITE setBrushSpacing NOTIFY brushChanged)
    Q_PROPERTY(BrushLibrary* brushLibrary READ brushLibrary CONSTANT)
    Q_PROPERTY(QVariantList recentColors READ recentColors NOTIFY recentColorsChanged)
    Q_PROPERTY(int historyLimit READ historyLimit WRITE setHistoryLimit NOTIFY historyLimitChanged)
    Q_PROPERTY(bool eraser READ eraser WRITE setEraser NOTIFY brushChanged)
    Q_PROPERTY(bool moveTool READ moveTool WRITE setMoveTool NOTIFY brushChanged)
    Q_PROPERTY(bool layerEditBusy READ layerEditBusy NOTIFY stateChanged)
    Q_PROPERTY(bool selectionEnabled READ selectionEnabled NOTIFY selectionChanged)
    Q_PROPERTY(QVariantList selectionSteps READ selectionSteps NOTIFY selectionChanged)
    Q_PROPERTY(int selectionTool READ selectionTool WRITE setSelectionTool NOTIFY brushChanged)
public:
    QVariantList historyEntries() const {return m_history;}
    explicit PaintCoreClient(QObject *parent = nullptr, const QString &settingsFile = {});
    QString settingsFile() const { return m_settingsFile; }
    ~PaintCoreClient() override;
    QVariantMap storageSettings() const { return m_storageSettings; }
    QVariantMap activeStorageSettings() const { return m_activeStorageSettings; }
    QVariantMap storageInfo() const { return m_storageInfo; }
    bool storageBusy() const { return m_storageBusy; }
    QString storageMessage() const { return m_storageMessage; }
    Q_INVOKABLE bool inspectStorage();
    Q_INVOKABLE bool saveStorageSettings(const QString &directory, int memoryMiB, int scratchGiB, int reserveMiB);
    Q_INVOKABLE QString localFolderPath(const QUrl &url) const { return url.toLocalFile(); }
    bool ready() const { return m_ready; }
    bool drawing() const { return m_drawing; }
    bool closing() const { return m_closing; }
    bool modified() const { return m_modified || m_drawing; }
    bool fileBusy() const { return m_fileBusy; }
    QString documentName() const;
    QUrl documentUrl() const { return m_documentUrl; }
    int documentWidth() const { return m_width; }
    int documentHeight() const { return m_height; }
    int undoDepth() const { return m_undo; }
    int redoDepth() const { return m_redo; }
    quint64 activeLayer() const { return m_active; }
    QVariantList layers() const { return m_layers; }
    QVariantList collapsedGroups() const;
    QString lastError() const { return m_error; }
    QColor brushColor() const { return m_color; }
    QColor secondaryBrushColor() const {return m_secondaryColor;}
    void setSecondaryBrushColor(const QColor &color);
    Q_INVOKABLE void swapBrushColors();
    qreal brushRadius() const { return m_radius; }
    qreal brushOpacity() const { return m_opacity; }
    qreal brushSpacing() const {return m_spacing;}
    BrushLibrary *brushLibrary() const {return m_brushLibrary;}
    QVariantList recentColors() const;
    bool eraser() const { return m_eraser; }
    bool moveTool() const { return m_moveTool; }
    bool layerEditBusy() const { return m_pendingLayer != 0; }
    void setMoveTool(bool enabled);
    bool selectionEnabled() const { return m_selection.value("enabled").toBool(); }
    QVariantList selectionSteps() const { return m_selection.value("steps").toList(); }
    int selectionTool() const { return m_selectionTool; }
    void setSelectionTool(int tool);
    Q_INVOKABLE bool editSelection(QRectF rectangle, int shape, int operation);
    /// Lasso path (freehand polygon) and magic-wand seed; both are resolved on the session worker.
    Q_INVOKABLE bool editSelectionPath(const QVariantList &points, int operation);
    Q_INVOKABLE bool magicWandSelection(qreal x, qreal y, int tolerance, int operation);
    Q_INVOKABLE bool selectAll();
    Q_INVOKABLE bool clearSelection();
    Q_INVOKABLE bool invertSelection();
    const QImage &frame(int view = 0) const { return m_frames[view == 1 ? 1 : 0]; }
    QRectF frameRegion(int view = 0) const { return m_regions[view == 1 ? 1 : 0]; }
    quint64 frameRevision(int view = 0) const { return m_frameRevisions[view == 1 ? 1 : 0]; }
    quint64 revision() const { return m_revision; }
    quint64 generation() const { return m_generation; }
    void requestViewport(int view, QRectF region, QSize physicalPixels, bool enabled = true);
    int historyLimit() const {return m_historyLimit;}
    void setHistoryLimit(int commands);
    Q_INVOKABLE void applyHistoryLimit();
    void setBrushColor(const QColor &color);
    void setBrushRadius(qreal radius);
    void setBrushOpacity(qreal opacity);
    void setBrushSpacing(qreal spacing);
    void setEraser(bool eraser);
    bool beginStroke(const InputSample &sample);
    void strokeTo(const InputSample &sample);
    void endStroke();
    Q_INVOKABLE void cancelStroke();
    Q_INVOKABLE void undo();
    Q_INVOKABLE void redo();
    Q_INVOKABLE void addLayer(const QString &name);
    Q_INVOKABLE void removeLayer(quint64 id);
    Q_INVOKABLE void selectLayer(quint64 id);
    /// Click selection: plain replaces, Ctrl/Cmd toggles, Shift selects the visible span.
    Q_INVOKABLE void selectLayers(quint64 id, int modifiers);
    QVariantList selectedLayers() const;
    void pruneSelectedLayers();
    /// Whole-canvas rotate/flip; `kind` is PAINT_CANVAS_TRANSFORM_*.
    Q_INVOKABLE bool transformCanvas(int kind);
    /// Eyedropper: sample the composited pixel at document coordinates. Read-only, no history.
    Q_INVOKABLE QColor sampleDocumentPixel(qreal x, qreal y) const;
    Q_INVOKABLE void setLayerProperties(quint64 id, bool visible, qreal opacity);
    Q_INVOKABLE bool setLayerFill(quint64 id, qreal fill);
    Q_INVOKABLE bool setLayerBlend(quint64 id, int blend);
    Q_INVOKABLE void previewLayerBlend(quint64 id, int blend);
    Q_INVOKABLE void clearLayerBlendPreview();
    Q_INVOKABLE bool setLayerLocks(quint64 id, int locks);
    Q_INVOKABLE bool moveLayer(quint64 id, int dx, int dy);
    Q_INVOKABLE bool addMask(quint64 id);
    Q_INVOKABLE bool setLayerClipping(quint64 id, bool enabled);
    Q_INVOKABLE bool toggleActiveClipping();
    Q_INVOKABLE bool dropLayer(quint64 id,quint64 target,int placement);
    Q_INVOKABLE void beginLayerDrag(quint64 id);
    Q_INVOKABLE QString layerDragPayload(quint64 id) const;
    Q_INVOKABLE bool acceptLayerDrop(const QString &payload,quint64 target,int placement);
    Q_INVOKABLE void addDefaultLayer();
    Q_INVOKABLE void newTransparentDocument(int width,int height);
    Q_INVOKABLE bool groupLayer(quint64 id, const QString &name);
    Q_INVOKABLE bool ungroupLayer(quint64 id);
    Q_INVOKABLE bool reparentLayer(quint64 id, quint64 parent);
    Q_INVOKABLE void toggleGroupExpanded(quint64 id);
    Q_INVOKABLE void requestLayerPreview(quint64 id);
    Q_INVOKABLE void releaseLayerPreview(quint64 id);
    Q_INVOKABLE void newDocument(int width, int height);
    Q_INVOKABLE bool openDocument(const QUrl &path);
    Q_INVOKABLE bool saveDocument(const QUrl &path, int format = 4);
    Q_INVOKABLE void cancelFile();
    Q_INVOKABLE void clearError();
    Q_INVOKABLE void shutdown();
signals:
    void historyChanged();
    void storageChanged();
    void storageFinished(bool success);
    void stateChanged();
    void layersChanged();
    void selectedLayersChanged();
    void groupExpansionChanged();
    void brushChanged();
    void recentColorsChanged();
    void historyLimitChanged();
    void selectionChanged();
    void errorChanged();
    void frameChanged();
    void stopped();
    void commandCompleted(bool onWorkerThread);
    void fileFinished(bool success);
private:
    bool submit(int type, const InputSample &sample = {}, quint64 id = 0,
                const QString &text = {}, qreal amount = 0, bool flag = false);
    void showError(const QString &error);
    bool submitFile(const QUrl &path, int kind, int format);
    bool setAppearance(quint64 id, const QString &field, const QVariant &value);
    bool submitGroup(quint32 kind, quint64 id, quint64 parent, const QString &name = {});
    bool submitSelection(quint32 action, QRectF rectangle = {}, int shape = 0, int operation = 0);
    QSet<quint64> m_collapsedGroups;
    QVariantMap m_storageSettings, m_activeStorageSettings, m_storageInfo;
    QString m_storageMessage;
    bool m_storageBusy = true;
    QString m_settingsFile;
    QThread m_thread;
    BackendWorker *m_worker = nullptr;
    std::shared_ptr<BackendConnection> m_connection;
    bool m_ready = false, m_drawing = false, m_closing = false, m_modified = false, m_dropStroke = false;
    bool m_fileBusy = false, m_fileOpening = false, m_backendFailed = false;
    QUrl m_documentUrl, m_sourceUrl, m_fileUrl;
    quint64 m_fileId = 0;
    int m_width = 960, m_height = 640, m_undo = 0, m_redo = 0;
    quint64 m_active = 1;
    quint64 m_generation = 0, m_revision = 0, m_pendingNew = 0, m_pendingMutation = 0;
    int m_nextDefaultLayer = 1;
    quint64 m_pendingLayer = 0;
    /// Sequence of an in-flight lasso/wand selection edit; non-zero keeps layer edits busy.

    QVariantList m_layers;
    QList<quint64> m_selectedLayers;
    QString m_error;
    QColor m_secondaryColor=Qt::white;
    QColor m_color{"#2ea99d"};
    // Colour of the stroke in progress: recent colours only advance on a committed stroke.
    QColor m_strokeColor;
    qreal m_radius = 12, m_opacity = 1;
    // Per-document history command limit; persisted as a preference and applied to new sessions.
    int m_historyLimit = 100;
    qreal m_spacing = .15;
    BrushLibrary *m_brushLibrary = nullptr;
    bool m_eraser = false;
    bool m_moveTool = false;
    int m_selectionTool = 0;
    QVariantMap m_selection;
    QVariantList m_history;
    QImage m_frames[2];
    QRectF m_regions[2], m_requestedRegions[2];
    QSize m_requestedPixels[2];
    quint64 m_requests[2]{}, m_frameRevisions[2]{};
    bool m_viewEnabled[2]{};
    quint64 m_blendPreviewLayer = 0;
    int m_blendPreviewMode = 0;
};
