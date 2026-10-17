#pragma once
#include "BrushLibrary.h"
#include <QColor>
#include <QImage>
#include <QHash>
#include <QObject>
#include <QElapsedTimer>
#include <QPointF>
#include <QRectF>
#include <QSize>
#include <QSet>
#include <QThread>
#include <QUrl>
#include <QVariantList>
#include <memory>
#include <algorithm>
#include <functional>

struct PaintTransform;
struct PaintBrushPose;
class QTimer;
struct BrushCursorPose {bool ready=false;qreal radius=0,angle=0,roundness=1;bool secondary=false;qreal secondaryRadius=0,secondaryAngle=0,secondaryRoundness=1;};
struct InputSample {
    QPointF position;
    float pressure = 1;
    float tiltX = 0, tiltY = 0, rotation = 0, tangentialPressure = 0;
    quint32 buttons = 0, capabilities = 0, tool = 3;
    quint64 timestamp = 0;
};
class BackendWorker;
struct BackendConnection;
struct BrushCatalogConnection;
struct PaintSelectionShapeEdit;

// The only UI-facing core service. Owns no model pointers on the GUI thread.
class PaintCoreClient final : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantMap storageSettings READ storageSettings NOTIFY storageChanged)
    Q_PROPERTY(QVariantMap activeStorageSettings READ activeStorageSettings NOTIFY storageChanged)
    Q_PROPERTY(QVariantMap storageInfo READ storageInfo NOTIFY storageChanged)
    Q_PROPERTY(bool storageBusy READ storageBusy NOTIFY storageChanged)
    Q_PROPERTY(QString storageMessage READ storageMessage NOTIFY storageChanged)
    Q_PROPERTY(bool ready READ ready NOTIFY stateChanged)
    Q_PROPERTY(bool brushLibraryReady READ brushLibraryReady NOTIFY brushLibraryReadyChanged)
    Q_PROPERTY(bool wetActionBusy READ wetActionBusy NOTIFY wetActionChanged)
    Q_PROPERTY(bool brushFileBusy READ brushFileBusy NOTIFY brushFileChanged)
    Q_PROPERTY(QVariantMap brushSourceInfo READ brushSourceInfo NOTIFY brushSourceChanged)
    Q_PROPERTY(QString brushSourceMessage READ brushSourceMessage NOTIFY brushSourceChanged)
    Q_PROPERTY(bool brushSourceBusy READ brushSourceBusy NOTIFY brushSourceChanged)
    Q_PROPERTY(QString brushImportMessage READ brushImportMessage NOTIFY brushFileChanged)
    Q_PROPERTY(QString cursorTipImage READ cursorTipImage NOTIFY brushCursorImageChanged)
    Q_PROPERTY(QString cursorSecondaryTipImage READ cursorSecondaryTipImage NOTIFY brushCursorImageChanged)
    Q_PROPERTY(QString cursorImageMessage READ cursorImageMessage NOTIFY brushCursorImageChanged)
    Q_PROPERTY(QString cursorSecondaryOutlinePath READ cursorSecondaryOutlinePath NOTIFY brushOutlineChanged)
    Q_PROPERTY(bool cursorSecondaryOutlineReady READ cursorSecondaryOutlineReady NOTIFY brushOutlineChanged)
    Q_PROPERTY(bool cursorSecondaryHasImage READ cursorSecondaryHasImage NOTIFY brushOutlineChanged)
    Q_PROPERTY(QString cursorOutlinePath READ cursorOutlinePath NOTIFY brushOutlineChanged)
    Q_PROPERTY(bool cursorOutlineReady READ cursorOutlineReady NOTIFY brushOutlineChanged)
    Q_PROPERTY(bool cursorHasImage READ cursorHasImage NOTIFY brushOutlineChanged)
    Q_PROPERTY(QString brushOutlinePath READ brushOutlinePath NOTIFY brushOutlineChanged)
    Q_PROPERTY(QString brushOutlineMessage READ brushOutlineMessage NOTIFY brushOutlineChanged)
    Q_PROPERTY(bool brushOutlineReady READ brushOutlineReady NOTIFY brushOutlineChanged)
    Q_PROPERTY(QVariantList historyEntries READ historyEntries NOTIFY historyChanged)
    Q_PROPERTY(QVariantList transformInitialPoints READ transformInitialPoints NOTIFY transformChanged)
    Q_PROPERTY(int transformReference READ transformReference WRITE setTransformReference NOTIFY transformChanged)
    Q_PROPERTY(bool transformLinked READ transformLinked WRITE setTransformLinked NOTIFY transformChanged)
    Q_PROPERTY(bool transformActive READ transformActive NOTIFY stateChanged)
    Q_PROPERTY(bool transformBusy READ transformBusy NOTIFY stateChanged)
    Q_PROPERTY(QVariantList transformPoints READ transformPoints NOTIFY transformChanged)
    Q_PROPERTY(bool transformWarp READ transformWarp NOTIFY transformChanged)
    Q_PROPERTY(qreal transformSkewH READ transformSkewH NOTIFY transformChanged)
    Q_PROPERTY(qreal transformSkewV READ transformSkewV NOTIFY transformChanged)
    Q_PROPERTY(int transformInterpolation READ transformInterpolation NOTIFY transformChanged)
    Q_PROPERTY(bool activeImage READ activeImage NOTIFY stateChanged)
    Q_PROPERTY(bool liquifyActive READ liquifyActive NOTIFY stateChanged)
    Q_PROPERTY(bool liquifyBusy READ liquifyBusy NOTIFY stateChanged)
    Q_PROPERTY(int liquifyTool READ liquifyTool WRITE setLiquifyTool NOTIFY liquifyOptionsChanged)
    Q_PROPERTY(int liquifySize READ liquifySize WRITE setLiquifySize NOTIFY liquifyOptionsChanged)
    Q_PROPERTY(int liquifyStrength READ liquifyStrength WRITE setLiquifyStrength NOTIFY liquifyOptionsChanged)
    Q_PROPERTY(int liquifyDensity READ liquifyDensity WRITE setLiquifyDensity NOTIFY liquifyOptionsChanged)
    Q_PROPERTY(int liquifySpeed READ liquifySpeed WRITE setLiquifySpeed NOTIFY liquifyOptionsChanged)
    Q_PROPERTY(bool liquifyFocus READ liquifyFocus WRITE setLiquifyFocus NOTIFY liquifyOptionsChanged)
    Q_PROPERTY(bool liquifyPressure READ liquifyPressure WRITE setLiquifyPressure NOTIFY liquifyOptionsChanged)
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
    Q_PROPERTY(bool layerShortcutsAllowed READ layerShortcutsAllowed NOTIFY layerShortcutsChanged)
    Q_PROPERTY(bool textInputFocused READ textInputFocused NOTIFY layerShortcutsChanged)
    Q_PROPERTY(bool canPasteLayers READ canPasteLayers NOTIFY stateChanged)
    Q_PROPERTY(bool canMergeLayers READ canMergeLayers NOTIFY stateChanged)
    Q_PROPERTY(QString mergeLayersText READ mergeLayersText NOTIFY stateChanged)
    Q_PROPERTY(QVariantList selectedLayers READ selectedLayers NOTIFY selectedLayersChanged)
    Q_PROPERTY(QVariantList collapsedGroups READ collapsedGroups NOTIFY groupExpansionChanged)
    Q_PROPERTY(QString lastError READ lastError NOTIFY errorChanged)
    Q_PROPERTY(QColor secondaryBrushColor READ secondaryBrushColor WRITE setSecondaryBrushColor NOTIFY brushChanged)
    Q_PROPERTY(QColor brushColor READ brushColor WRITE setBrushColor NOTIFY brushChanged)
    Q_PROPERTY(int brushWriteBlend READ brushWriteBlend WRITE setBrushWriteBlend NOTIFY brushChanged)
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
    Q_PROPERTY(int selectionOperation READ selectionOperation WRITE setSelectionOperation NOTIFY selectionOptionsChanged)
    Q_PROPERTY(qreal selectionFeather READ selectionFeather WRITE setSelectionFeather NOTIFY selectionOptionsChanged)
    Q_PROPERTY(bool selectionAntialias READ selectionAntialias WRITE setSelectionAntialias NOTIFY selectionOptionsChanged)
    Q_PROPERTY(int lassoShape READ lassoShape NOTIFY brushChanged)
    Q_PROPERTY(int marqueeShape READ marqueeShape NOTIFY brushChanged)
    Q_PROPERTY(bool samplingRingEnabled READ samplingRingEnabled WRITE setSamplingRingEnabled NOTIFY samplingRingEnabledChanged)
    Q_PROPERTY(bool eyedropperTool READ eyedropperTool NOTIFY brushChanged)
    Q_PROPERTY(bool bucketTool READ bucketTool NOTIFY brushChanged)
    Q_PROPERTY(bool smudgeTool READ smudgeTool NOTIFY brushChanged)
    Q_PROPERTY(qreal smudgeStrength READ smudgeStrength WRITE setSmudgeStrength NOTIFY smudgeOptionsChanged)
    Q_PROPERTY(bool smudgePressure READ smudgePressure WRITE setSmudgePressure NOTIFY smudgeOptionsChanged)
    Q_PROPERTY(bool smudgeAllLayers READ smudgeAllLayers WRITE setSmudgeAllLayers NOTIFY smudgeOptionsChanged)
    Q_PROPERTY(bool smudgeFingerPainting READ smudgeFingerPainting WRITE setSmudgeFingerPainting NOTIFY smudgeOptionsChanged)
    Q_PROPERTY(int wandSampleSize READ wandSampleSize WRITE setWandSampleSize NOTIFY wandOptionsChanged)
    Q_PROPERTY(int wandTolerance READ wandTolerance WRITE setWandTolerance NOTIFY wandOptionsChanged)
    Q_PROPERTY(int wandOperation READ wandOperation WRITE setWandOperation NOTIFY wandOptionsChanged)
    Q_PROPERTY(bool wandAntialias READ wandAntialias WRITE setWandAntialias NOTIFY wandOptionsChanged)
    Q_PROPERTY(bool wandContiguous READ wandContiguous WRITE setWandContiguous NOTIFY wandOptionsChanged)
    Q_PROPERTY(bool wandAllLayers READ wandAllLayers WRITE setWandAllLayers NOTIFY wandOptionsChanged)
    Q_PROPERTY(qreal bucketOpacity READ bucketOpacity WRITE setBucketOpacity NOTIFY bucketOptionsChanged)
    Q_PROPERTY(int bucketTolerance READ bucketTolerance WRITE setBucketTolerance NOTIFY bucketOptionsChanged)
    Q_PROPERTY(bool bucketAntialias READ bucketAntialias WRITE setBucketAntialias NOTIFY bucketOptionsChanged)
    Q_PROPERTY(bool bucketContiguous READ bucketContiguous WRITE setBucketContiguous NOTIFY bucketOptionsChanged)
    Q_PROPERTY(bool bucketAllLayers READ bucketAllLayers WRITE setBucketAllLayers NOTIFY bucketOptionsChanged)
    Q_PROPERTY(QVariantMap layerThumbnails READ layerThumbnails NOTIFY layerThumbnailsChanged)
public:
    void setBrushPreferencePersistenceEnabled(bool enabled) {m_brushPreferencePersistenceEnabled=enabled;}
    void persistBrushPreferences(const QByteArray &state);
    bool brushFileBusy() const {return m_brushFileBusy;}
    QString brushImportMessage() const {return m_brushImportMessage;}
    QString cursorTipImage() const {return m_cursorImages.value(m_drawing && !m_liquifyActive?m_strokeOutlineResource:m_outlineResource);}
    QString cursorSecondaryTipImage() const {if(m_drawing && !m_strokeSecondImageEligible)return {};return m_cursorImages.value(m_drawing && !m_liquifyActive?m_strokeSecondOutlineResource:m_secondOutlineResource);}
    QString cursorImageMessage() const {return m_cursorImageErrors.value(m_drawing && !m_liquifyActive?m_strokeOutlineResource:m_outlineResource);}
    QString cursorOutlinePath() const {return m_drawing && !m_liquifyActive?m_strokeOutlinePath:m_brushOutlinePath;}
    bool cursorOutlineReady() const {return m_drawing && !m_liquifyActive?m_strokeOutlineReady:m_brushOutlineReady;}
    bool cursorHasImage() const {return !(m_drawing && !m_liquifyActive?m_strokeOutlineResource:m_brushLibrary->maskResource()).isEmpty();}
    QString cursorSecondaryOutlinePath() const {return m_drawing && !m_liquifyActive?m_strokeSecondOutlinePath:m_secondOutlinePath;}
    bool cursorSecondaryOutlineReady() const {return m_drawing && !m_liquifyActive?m_strokeSecondOutlineReady:m_secondOutlineReady;}
    bool cursorSecondaryHasImage() const {return !(m_drawing && !m_liquifyActive?m_strokeSecondOutlineResource:m_secondOutlineResource).isEmpty();}
    BrushCursorPose baseCursorPose() const;
    BrushCursorPose brushCursorPose(const InputSample &sample,QPointF direction,bool directionKnown,bool pressureKnown);
    QString brushOutlinePath() const {return m_brushOutlinePath;}
    QString brushOutlineMessage() const {return !m_brushLibrary->cursorComposite() || m_secondOutlineMessage.isEmpty()?m_brushOutlineMessage:m_brushOutlineMessage.isEmpty()?m_secondOutlineMessage:m_brushOutlineMessage+QStringLiteral("\n")+m_secondOutlineMessage;}
    bool brushOutlineReady() const {return m_brushOutlineReady;}
    bool brushLibraryReady() const {return m_brushLibraryReady;}
    QVariantMap brushSourceInfo() const {return m_brushSourceInfo;}
    QString brushSourceMessage() const {return m_brushSourceMessage;}
    bool brushSourceBusy() const {return m_brushSourceBusy;}
    bool wetActionBusy() const {return m_wetActionBusy;}
    Q_INVOKABLE bool cleanWetBrush();
    Q_INVOKABLE bool loadWetBrush();
    bool wetBrushAction(bool reload);
    Q_INVOKABLE bool inspectBrushSource();
    Q_INVOKABLE bool importBrushes(const QUrl &path);
    Q_INVOKABLE bool importBrushTip(const QUrl &path,int interpretation);
    Q_INVOKABLE bool importBrushTexture(const QUrl &path,int interpretation);
    Q_INVOKABLE bool importDualBrushTip(const QUrl &path,int interpretation);
    Q_INVOKABLE bool exportBrush(const QUrl &path,bool includeSource=false,const QString &expectedPreset={});
    Q_INVOKABLE bool exportAllBrushes(const QUrl &path,bool includeSource=false);
    QVariantList historyEntries() const {return m_history;}
    Q_INVOKABLE qreal documentHistoryScrollPosition() const {return m_documentHistoryScroll;}
    Q_INVOKABLE void rememberDocumentHistoryScroll(qreal y) {if(!m_historyLiquify)m_documentHistoryScroll=std::max(qreal(0),y);}
    explicit PaintCoreClient(QObject *parent = nullptr, const QString &settingsFile = {},std::shared_ptr<BrushCatalogConnection> brushCatalog = {});
    std::shared_ptr<BrushCatalogConnection> brushCatalogConnection() const {return m_brushCatalogConnection;}
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
    bool modified() const { return m_modified || (m_drawing && !m_liquifyActive); }
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
    int brushWriteBlend() const {return m_brushLibrary->writeBlend();}
    void setBrushWriteBlend(int mode);
    qreal brushRadius() const { return m_radius; }
    qreal brushOpacity() const { return m_opacity; }
    qreal brushSpacing() const {return m_spacing;}
    BrushLibrary *brushLibrary() const {return m_brushLibrary;}
    QVariantList recentColors() const;
    bool eraser() const { return m_eraser; }
    /// Tool-slot activation: exactly one painting/selection tool stays active. QML handlers must not
    /// clear the other tools themselves, because a statement block inside a single-line binding does
    /// not parse.
    Q_INVOKABLE void activateBrush();
    Q_INVOKABLE void activateEraser();
    Q_INVOKABLE void activateMoveTool();
    Q_INVOKABLE void activateEyedropper();
    Q_INVOKABLE void activateBucket();
    Q_INVOKABLE void activateSmudge();
    bool smudgeTool() const {return m_smudgeTool;}
    qreal smudgeStrength() const {return m_smudgeStrength;}
    bool smudgePressure() const {return m_smudgePressure;}
    bool smudgeAllLayers() const {return m_smudgeAllLayers;}
    bool smudgeFingerPainting() const {return m_smudgeFingerPainting;}
    void setSmudgeStrength(qreal value);
    void setSmudgePressure(bool value);
    void setSmudgeAllLayers(bool value);
    void setSmudgeFingerPainting(bool value);
    bool bucketTool() const {return m_bucketTool;}
    qreal bucketOpacity() const {return m_bucketOpacity;}
    int bucketTolerance() const {return m_bucketTolerance;}
    bool bucketAntialias() const {return m_bucketAntialias;}
    bool bucketContiguous() const {return m_bucketContiguous;}
    bool bucketAllLayers() const {return m_bucketAllLayers;}
    void setBucketOpacity(qreal value);
    void setBucketTolerance(int value);
    void setBucketAntialias(bool value);
    void setBucketContiguous(bool value);
    void setBucketAllLayers(bool value);
    bool bucketFill(QPointF position);
    bool samplingRingEnabled() const {return m_samplingRingEnabled;}
    void setSamplingRingEnabled(bool enabled);
    bool eyedropperTool() const {return m_eyedropperTool;}
    /// Marquee keeps the last rectangle/ellipse choice; the lasso and the wand are their own tools.
    Q_INVOKABLE QVariantList pressureCurveSamples(const QVariantList &points,bool smooth) const;
    Q_INVOKABLE void activateMarquee();
    Q_INVOKABLE void activateLasso();
    bool moveTool() const { return m_moveTool; }
    bool layerEditBusy() const { return m_pendingLayer != 0 || m_pendingCopy != 0; }
    void setMoveTool(bool enabled);
    bool selectionEnabled() const { return m_selection.value("enabled").toBool(); }
    QVariantList selectionSteps() const { return m_selection.value("steps").toList(); }
    int selectionTool() const { return m_selectionTool; }
    Q_INVOKABLE void activateWand() {setSelectionTool(5);}
    bool selectWand(QPointF position, int operation);
    int wandSampleSize() const {return m_wandSampleSize;}
    int wandTolerance() const {return m_wandTolerance;}
    int wandOperation() const {return m_wandOperation;}
    void setWandOperation(int value);
    bool wandAntialias() const {return m_wandAntialias;}
    bool wandContiguous() const {return m_wandContiguous;}
    bool wandAllLayers() const {return m_wandAllLayers;}
    void setWandSampleSize(int value);
    void setWandTolerance(int value);
    void setWandAntialias(bool value);
    void setWandContiguous(bool value);
    void setWandAllLayers(bool value);
    int selectionOperation() const {return m_selectionOperation;}
    qreal selectionFeather() const {return m_selectionFeather;}
    bool selectionAntialias() const {return m_selectionAntialias;}
    int lassoShape() const {return m_lassoShape;}
    void setSelectionOperation(int operation);
    void setSelectionFeather(qreal feather);
    void setSelectionAntialias(bool enabled);
    bool editSelectionPath(const QVector<QPointF> &points, int operation, bool polygonal);
    int marqueeShape() const { return m_marqueeShape; }
    QVariantMap layerThumbnails() const { return m_layerThumbnails; }
    void setSelectionTool(int tool);
    Q_INVOKABLE bool editSelection(QRectF rectangle, int shape, int operation);
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
    QVariantList transformInitialPoints() const {return m_transformInitialPoints;}
    int transformReference() const {return m_transformReference;}
    void setTransformReference(int v){if(v>=0 && v<=8 && v!=m_transformReference){m_transformReference=v;emit transformChanged();}}
    bool transformLinked() const {return m_transformLinked;}
    void setTransformLinked(bool v){if(v!=m_transformLinked){m_transformLinked=v;emit transformChanged();}}
    bool transformActive() const {return m_transformActive;}
    bool transformBusy() const {return m_pendingTransform>m_completedSequence;}
    QVariantList transformPoints() const {return m_transformPoints;}
    bool transformWarp() const {return m_transformWarp;}
    qreal transformSkewH() const {return m_transformSkewH;}
    qreal transformSkewV() const {return m_transformSkewV;}
    int transformInterpolation() const {return m_transformInterpolation;}
    bool activeImage() const {for(const auto &v:m_layers){const auto l=v.toMap();if(l.value("id").toULongLong()==m_active)return l.value("image").toBool();}return false;}
    Q_INVOKABLE bool placeImage(const QUrl &path);
    Q_INVOKABLE bool beginTransform();
    Q_INVOKABLE bool updateTransform(const QVariantList &points,bool warp,int interpolation, qreal skewH=qQNaN(), qreal skewV=qQNaN());
    Q_INVOKABLE bool checkpointTransform();
    Q_INVOKABLE bool finishTransform(bool commit);
    Q_INVOKABLE bool rasterizeImage();
    bool liquifyActive() const {return m_liquifyActive;}
    bool liquifyBusy() const {return m_pendingLiquify!=0 || layerEditBusy();}
    int liquifyTool() const {return m_liquifyTool;}
    int liquifySize() const {return m_liquifySize;}
    int liquifyStrength() const {return m_liquifyStrength;}
    int liquifyDensity() const {return m_liquifyDensity;}
    int liquifySpeed() const {return m_liquifySpeed;}
    bool liquifyFocus() const {return m_liquifyFocus;}
    void setLiquifyFocus(bool value);
    bool liquifyPressure() const {return m_liquifyPressure;}
    void setLiquifyTool(int value);
    void setLiquifySize(int value);
    void setLiquifyStrength(int value);
    void setLiquifyDensity(int value);
    void setLiquifySpeed(int value);
    void setLiquifyPressure(bool value);
    Q_INVOKABLE bool beginLiquify();
    Q_INVOKABLE bool resetLiquify();
    Q_INVOKABLE bool finishLiquify(bool commit);
    void liquifyTick();
    bool beginStroke(const InputSample &sample);
    void strokeTo(const InputSample &sample);
    void airbrushTick();
    void endStroke();
    Q_INVOKABLE void cancelStroke();
    Q_INVOKABLE void undo();
    Q_INVOKABLE void redo();
    Q_INVOKABLE void addLayer(const QString &name);
    Q_INVOKABLE void removeLayer(quint64 id);
    Q_INVOKABLE void requestSelectedLayerRemoval();
    Q_INVOKABLE bool confirmSelectedLayerRemoval(int choice);
    Q_INVOKABLE bool dropSelectedLayers(quint64 target, int placement);
    Q_INVOKABLE QVariantList layerDragIds(const QString &payload) const;
    Q_INVOKABLE void selectLayer(quint64 id);
    /// Click selection: plain replaces, Ctrl/Cmd toggles, Shift selects the visible span.
    Q_INVOKABLE void selectLayers(quint64 id, int modifiers);
    QVariantList selectedLayers() const;
    void pruneSelectedLayers();
    /// Eyedropper: sample the composited pixel at document coordinates. Read-only, no history.
    Q_INVOKABLE QColor sampleDocumentPixel(qreal x, qreal y) const;
    Q_INVOKABLE void setLayerProperties(quint64 id, bool visible, qreal opacity);
    Q_INVOKABLE bool setLayerFill(quint64 id, qreal fill);
    Q_INVOKABLE bool setLayerBlend(quint64 id, int blend);
    Q_INVOKABLE void previewLayerBlend(quint64 id, int blend);
    Q_INVOKABLE void clearLayerBlendPreview();
    Q_INVOKABLE bool setLayerLocks(quint64 id, int locks);
    Q_INVOKABLE bool moveLayer(quint64 id, int dx, int dy);
    bool pickLayer(QPointF point, int dx = 0, int dy = 0);
    Q_INVOKABLE bool addMask(quint64 id);
    Q_INVOKABLE bool setLayerClipping(quint64 id, bool enabled);
    Q_INVOKABLE bool toggleActiveClipping();
    Q_INVOKABLE bool dropLayer(quint64 id,quint64 target,int placement);
    Q_INVOKABLE void beginLayerDrag(quint64 id);
    Q_INVOKABLE void beginBrushDrag(const QString &payload);
    Q_INVOKABLE QString layerDragPayload(quint64 id) const;
    Q_INVOKABLE bool acceptLayerDrop(const QString &payload,quint64 target,int placement);
    Q_INVOKABLE void addDefaultLayer();
    Q_INVOKABLE void newTransparentDocument(int width,int height);
    bool layerShortcutsAllowed() const;
    bool textInputFocused() const;
    bool canPasteLayers() const;
    bool canMergeLayers() const;
    QString mergeLayersText() const;
    Q_INVOKABLE bool copySelectedLayers();
    Q_INVOKABLE bool pasteLayers();
    Q_INVOKABLE bool duplicateSelectedLayers();
    Q_INVOKABLE bool mergeSelectedLayers();
    Q_INVOKABLE bool groupLayer(quint64 id, const QString &name);
    Q_INVOKABLE bool groupSelectedLayers(const QString &name);
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
    void brushCursorPreferencesChanged();
    void brushCursorImageChanged();
    void brushOutlineChanged();
    void brushLibraryReadyChanged();
    void wetActionChanged();
    void wetBrushFinished(bool success);
    void brushFileChanged();
    void brushSourceChanged();
    void brushFileFinished(bool success);
    void liquifyOptionsChanged();
    void historyChanged();
    void storageChanged();
    void storageFinished(bool success);
    void stateChanged();
    void layerShortcutsChanged();
    void transformChanged();
    void historyScopeChanging(bool liquify);
    void layersChanged();
    void layerThumbnailsChanged();
    void selectedLayersChanged();
    void groupDeletionRequested();
    void groupExpansionChanged();
    void brushChanged();
    void samplingRingEnabledChanged();
    void recentColorsChanged();
    void historyLimitChanged();
    void selectionChanged();
    void selectionOptionsChanged();
    void bucketOptionsChanged();
    void smudgeOptionsChanged();
    void wandOptionsChanged();
    void errorChanged();
    void frameChanged();
    void stopped();
    void commandCompleted(bool onWorkerThread);
    void fileFinished(bool success);
private:
    bool m_brushPreferencePersistenceEnabled=true;
    bool m_wetActionBusy=false;
    bool m_brushFileBusy=false;
    QString m_brushImportMessage;
    QString m_brushSourceIdentity,m_brushSourceMessage;
    QVariantMap m_brushSourceInfo;
    quint64 m_brushSourceToken=0;
    bool m_brushSourceBusy=false;
    PaintBrushPose *m_hoverPose=nullptr,*m_strokePose=nullptr;
    QByteArray m_poseExpectedSource;
    quint64 m_hoverPosePreparation=0;QString m_hoverPoseId;float m_hoverPoseRadius=0;
    BrushCursorPose m_liveBasePose{false,12,0,1},m_strokeBasePose;bool m_strokePoseBound=false,m_strokeOutlineReady=false;
    QString m_strokeOutlineResource,m_strokeOutlinePath;
    QString m_outlineResource,m_brushOutlinePath,m_brushOutlineMessage;
    bool m_brushOutlineReady=false;
    bool m_brushLibraryReady=false;
    QString m_pendingBrushImport;
    QString m_secondOutlineResource,m_secondOutlinePath,m_secondOutlineMessage,m_strokeSecondOutlineResource,m_strokeSecondOutlinePath;
    bool m_secondOutlineReady=false,m_strokeSecondOutlineReady=false;
    void requestBrushOutlines(bool updateImages=true);
    void requestBrushCursorImages();
    QHash<QString,QString> m_cursorImages,m_cursorImageErrors;
    QStringList m_cursorImageDemand;
    bool m_strokeSecondImageEligible=false;
    bool submit(int type, const InputSample &sample = {}, quint64 id = 0,
                const QString &text = {}, qreal amount = 0, bool flag = false);
    void showError(const QString &error);
    bool submitFile(const QUrl &path, int kind, int format);
    bool setAppearance(quint64 id, const QString &field, const QVariant &value);
    bool submitGroup(quint32 kind, quint64 id, quint64 parent, const QString &name = {});
    bool submitSelectionShape(const PaintSelectionShapeEdit &request);
    bool submitSelection(quint32 action, QRectF rectangle = {}, int shape = 0, int operation = 0);
    QSet<quint64> m_collapsedGroups;
    QVariantMap m_storageSettings, m_activeStorageSettings, m_storageInfo;
    QString m_storageMessage;
    bool m_storageBusy = true;
    QString m_settingsFile;
    QThread m_thread;
    BackendWorker *m_worker = nullptr;
    std::shared_ptr<BackendConnection> m_connection;
    std::shared_ptr<BrushCatalogConnection> m_brushCatalogConnection;
    bool synchronizeBrushCatalog(const QByteArray &state);
    bool m_ready = false, m_drawing = false, m_closing = false, m_modified = false, m_dropStroke = false;
    bool m_fileBusy = false, m_fileOpening = false, m_backendFailed = false;
    QUrl m_documentUrl, m_sourceUrl, m_fileUrl;
    quint64 m_fileId = 0;
    int m_width = 960, m_height = 640, m_undo = 0, m_redo = 0;
    quint64 m_active = 1;
    quint64 m_generation = 0, m_revision = 0, m_pendingNew = 0, m_pendingMutation = 0;
    int m_nextDefaultLayer = 1;
    bool submitLayerAction(quint32 kind, const QByteArray &bytes = {});
    quint64 m_pendingCopy = 0;
    quint64 m_pendingLayer = 0;
    /// Sequence of an in-flight lasso/wand selection edit; non-zero keeps layer edits busy.

    QVariantList m_layers;
    QVariantMap m_layerThumbnails;
    QList<quint64> m_selectedLayers;
    QList<quint64> m_deleteLayers;
    quint64 m_deleteGeneration = 0, m_deleteRevision = 0;
    bool removeLayers(const QList<quint64> &ids, bool keepChildren);
    bool dropLayers(const QList<quint64> &ids, quint64 target, int placement);
    QString m_error;
    QColor m_secondaryColor=Qt::white;
    QColor m_color{"#2ea99d"};
    // Colour of the stroke in progress: recent colours only advance on a committed stroke.
    QColor m_strokeColor;
    qreal m_radius = 12, m_opacity = 1;
    // Per-document history command limit; persisted as a preference and applied to new sessions.
    void persistLiquifyOptions();
    bool submitLiquifyControl(quint32 kind);
    bool deferTransformAction(std::function<void()> action);
    std::function<void()> m_afterTransform;
    bool submitTransform(quint32 kind,const PaintTransform *geometry=nullptr);
    bool m_transformActive=false,m_transformWarp=false,m_transformPublished=false,m_transformLinked=true;
    QVariantList m_transformInitialPoints;
    qreal m_transformSkewH=0,m_transformSkewV=0;
    int m_transformInterpolation=2,m_transformReference=4;
    quint64 m_pendingTransform=0,m_lastTransform=0;
    QVariantList m_transformPoints;
    bool submitLiquifyStrokeMarker(quint32 kind);
    bool submitLiquifyDab(const InputSample &from,const InputSample &to,int elapsedMs);
    bool m_liquifyFocus=false,m_liquifyActive=false,m_liquifyPressure=false;
    int m_liquifyTool=0,m_liquifySize=200,m_liquifyStrength=50,m_liquifyDensity=100,m_liquifySpeed=100;
    quint64 m_completedSequence=0;
    quint64 m_pendingLiquify=0,m_lastLiquifyDab=0;
    InputSample m_liquifyLast;
    QElapsedTimer m_liquifyClock;
    QTimer *m_airbrushTimer=nullptr;
    InputSample m_airbrushSample;
    quint64 m_airbrushTimestamp=0;
    quint64 m_airbrushIntervalNs=0;
    bool m_airbrushEnabled=false;
    int m_historyLimit = 100;
    qreal m_spacing = .15;
    BrushLibrary *m_brushLibrary = nullptr;
    bool m_eraser = false;
    /// Last marquee shape (1 rectangle, 2 ellipse) so the slot does not reset when re-activated.
    int m_marqueeShape = 1, m_lassoShape = 3;
    int m_selectionOperation = 0;
    qreal m_selectionFeather = 0;
    bool m_selectionAntialias = true;
    bool m_moveTool = false;
    bool m_eyedropperTool = false;
    bool m_samplingRingEnabled = true;
    bool m_bucketTool = false;
    bool m_smudgeTool=false,m_smudgePressure=true,m_smudgeAllLayers=false,m_smudgeFingerPainting=false;
    qreal m_smudgeStrength=.5;
    int m_wandSampleSize=1,m_wandTolerance=10;
    int m_wandOperation=1;
    bool m_wandAntialias=true,m_wandContiguous=true,m_wandAllLayers=false;
    qreal m_bucketOpacity = 1;
    int m_bucketTolerance = 0;
    bool m_bucketAntialias = true, m_bucketContiguous = true, m_bucketAllLayers = false;
    int m_selectionTool = 0;
    QVariantMap m_selection;
    QVariantList m_history;
    bool m_historyLiquify=false;
    qreal m_documentHistoryScroll=0;
    QImage m_frames[2];
    QRectF m_regions[2], m_requestedRegions[2];
    QSize m_requestedPixels[2];
    quint64 m_requests[2]{}, m_frameRevisions[2]{};
    bool m_viewEnabled[2]{};
    quint64 m_blendPreviewLayer = 0;
    int m_blendPreviewMode = 0;
};
