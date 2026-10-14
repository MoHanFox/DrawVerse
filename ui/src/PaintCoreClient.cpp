#include "PaintCoreClient.h"
#include "paint_api.h"
#include <QMutex>
#include <QMutexLocker>
#include <QTimer>
#include <QFileInfo>
#include <QSettings>
#include <QDir>
#include <QBuffer>
#include <QElapsedTimer>
#include <QHash>
#include <QDrag>
#include <QMimeData>
#include <QGuiApplication>
#include <QClipboard>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <algorithm>
#include <atomic>
#include <vector>
#include <cmath>
#include <utility>

Q_DECLARE_METATYPE(PaintSessionInfo)
Q_DECLARE_METATYPE(PaintFileJobInfo)
namespace {
enum Operation { New = 1, Begin, Move, End, Cancel, Undo, Redo, Add, Remove, Select, Properties, NewWhite, AddMask };
template<class T> T dto() { T value{}; value.struct_size = sizeof(T); return value; }
float linear(float v) { return v <= .04045f ? v / 12.92f : std::pow((v + .055f) / 1.055f, 2.4f); }
QString immediateError(PaintStatus status) {
    uint64_t size = 0; paint_error_message(nullptr, 0, &size);
    QByteArray message(static_cast<qsizetype>(size), '\0');
    paint_error_message(reinterpret_cast<uint8_t *>(message.data()), size, &size);
    return QStringLiteral("核心错误 %1：%2").arg(status).arg(QString::fromUtf8(message));
}
PaintPoint point(const InputSample &s) {
    auto p = dto<PaintPoint>();
    p.x = s.position.x(); p.y = s.position.y(); p.pressure = s.pressure;
    p.tilt_x = s.tiltX; p.tilt_y = s.tiltY; p.rotation = s.rotation;
    p.tangential_pressure = s.tangentialPressure; p.buttons = s.buttons;
    p.capabilities = s.capabilities; p.tool = s.tool; p.timestamp_ns = s.timestamp;
    return p;
}
}
// GUI calls through this bridge only enqueue small commands or view requests.
// The worker owns the handles and clears the bridge before joining Rust workers.
struct BackendConnection {
    QMutex mutex;
    PaintCore *core = nullptr;
    PaintSession *session = nullptr;
    std::atomic_bool framePending[2]{};
    std::atomic_bool metadataPending{false};
    std::atomic<quint64> fileJob{0};
};
struct BrushCatalogConnection {
    QMutex mutex;
    PaintBrushLibrary *library=nullptr;
    quint64 preparation=0;
    int workers=0;
    QByteArray source,preferences;
};
class BackendWorker final : public QObject {
    Q_OBJECT
public:
    explicit BackendWorker(std::shared_ptr<BackendConnection> connection, QString settingsFile,std::shared_ptr<BrushCatalogConnection> catalog) : m_catalog(std::move(catalog)),m_settingsFile(std::move(settingsFile)), m_connection(std::move(connection)) {
        m_timer = new QTimer(this); m_timer->setInterval(16);
        connect(m_timer, &QTimer::timeout, this, &BackendWorker::poll);
    }
    void boot() {
        auto version = dto<PaintVersion>();
        if (!check(paint_core_version(&version))) return;
        if (version.major != PAINT_ABI_MAJOR || version.minor < 42) {
            emit failure(QStringLiteral("笔刷系统需要核心 ABI 1.42 或兼容后续版本")); return;
        }
        auto settings = preferences();
        {QMutexLocker lock(&m_catalog->mutex);
            if(!m_catalog->library){
                if(!check(paint_brush_library_create(0,&m_catalog->library)))return;
                const auto directory=QFileInfo(settings->fileName()).dir().filePath("brush-resources").toUtf8();
                if(!check(paint_brush_library_bind_resources(m_catalog->library,reinterpret_cast<const uint8_t*>(directory.constData()),quint64(directory.size())))){paint_brush_library_destroy(&m_catalog->library);return;}
            }
            ++m_catalog->workers;m_catalogLease=true;
        }
        emit brushPreferences(settings->value("brushes/state").toByteArray());
        emit liquifyPreferences({{"tool",settings->value("liquify/tool",0)},{"size",settings->value("liquify/size",200)},{"strength",settings->value("liquify/strength",50)},{"density",settings->value("liquify/density",100)},{"speed",settings->value("liquify/speed",100)},{"pressure",settings->value("liquify/pressure",false)}});
        m_historyLimit = std::clamp(settings->value("history/limit", 100).toInt(), 10, 1000);
        m_saved = QVariantMap{{"directory", settings->value("storage/directory", "")},
            {"memoryMiB", settings->value("storage/memoryMiB", 256)},
            {"scratchGiB", settings->value("storage/scratchGiB", 8)},
            {"reserveMiB", settings->value("storage/reserveMiB", 512)}};
        QByteArray path; const auto options = storageOptions(m_saved, path);
        const auto created = paint_core_create_with_storage(&options, &m_core);
        if (created != PAINT_OK) {
            const auto message = immediateError(created);
            emit storageState(m_saved, {}, {}, message); emit storageDone(false);
            emit failure(message); return;
        }
        m_active = m_saved;
        inspectStorage();
        auto caps = dto<PaintCapabilities>();
        if (!check(paint_core_capabilities(m_core, &caps))) return;
        constexpr uint64_t required = PAINT_FEATURE_ASYNC_SESSION | PAINT_FEATURE_CPU_VIEWPORT | PAINT_FEATURE_FILE_IO | PAINT_FEATURE_STORAGE_SETTINGS | PAINT_FEATURE_LAYER_APPEARANCE | PAINT_FEATURE_LAYER_GROUPS | PAINT_FEATURE_MASKS | PAINT_FEATURE_CLIPPING;
        if ((caps.features & (required | PAINT_FEATURE_SELECTION)) != (required | PAINT_FEATURE_SELECTION)) { emit failure(QStringLiteral("当前核心缺少异步视口或选区能力")); return; }
        auto desc = dto<PaintDocumentDesc>(); desc.width = 960; desc.height = 640;
        desc.working_space = PAINT_WORKING_LINEAR_SRGB; desc.pixel_format = PAINT_STORAGE_RGBA32F_PREMULTIPLIED;
        if (!check(paint_session_create(m_core, &desc, &m_session))) return;
        { QMutexLocker lock(&m_connection->mutex); m_connection->core = m_core; m_connection->session = m_session; }
        // The persisted history limit applies from the first document of this run.
        if (m_historyLimit > 0 && m_historyLimit != 100) {
            quint64 sequence = 0;
            if (!check(paint_session_set_history_limit(m_core, m_session, quint32(m_historyLimit), &sequence))) return;
        }
        auto initial=dto<PaintCommand>();initial.kind=PAINT_COMMAND_NEW_WHITE_DOCUMENT;initial.width=960;initial.height=640;
        if(!check(paint_session_submit(m_core,m_session,&initial,&m_bootSequence))) return;
        m_previewClock.start(); poll(); m_timer->start();
    }
    // Disk queries and QSettings synchronization stay on the backend thread.
    void inspectStorage() { evaluateStorage(m_saved, false); }
    void saveStorage(const QVariantMap &values) { evaluateStorage(values, true); }
    void saveBrushPreferences(const QByteArray &json){m_pendingBrushPreferences=json;pollBrushState();}
    void saveLiquifyOptions(const QVariantMap &values) {auto settings=preferences();for(auto it=values.cbegin();it!=values.cend();++it)settings->setValue("liquify/"+it.key(),it.value());settings->sync();if(settings->status()!=QSettings::NoError)emit failure(QStringLiteral("无法保存液化设置，请检查配置目录权限"));}
    void storeHistoryLimit(int commands) {
        auto settings=preferences();settings->setValue("history/limit",commands);settings->sync();
        if(settings->status()!=QSettings::NoError)emit failure(QStringLiteral("无法保存历史条数上限，请检查配置目录权限"));
    }
    void applyHistoryLimit() {
        if(m_stopped || !m_session)return;
        quint64 sequence=0;
        if(!check(paint_session_set_history_limit(m_core,m_session,quint32(m_historyLimit),&sequence)))
            emit failure(QStringLiteral("无法应用历史条数上限"));
    }
    void queueBrushPreview(const QString &id,quint64 token,qreal radius,qreal opacity,qreal spacing,QVariantMap tip) {        m_brushQueue.removeIf([&](const QVariant &v){return v.toMap().value("id")==id;});
        m_brushQueue.append(QVariantMap{{"id",id},{"token",token},{"radius",radius},{"opacity",opacity},{"spacing",spacing},{"tip",tip}});
    }
    void referencePreview(quint64 id, bool add) {
        if(add) {
            if(m_previewReferences.contains(id)) ++m_previewReferences[id];
            else if(m_previewTargets.size()<4096) { m_previewReferences[id]=1; m_previewTargets.append(id); }
        } else if(m_previewReferences.contains(id)) {
            if(--m_previewReferences[id]<=0) { m_previewReferences.remove(id); m_previewTargets.removeAll(id); }
        }
        m_previewIndex=0;
    }
    void stop() {
        if (m_stopped) return;
        m_stopped = true; m_timer->stop();
        while(!m_pendingBrushPreferences.isEmpty()){pollBrushState();if(!m_pendingBrushPreferences.isEmpty())QThread::msleep(1);}
        { QMutexLocker lock(&m_connection->mutex); m_connection->session = nullptr; m_connection->core = nullptr; }
        // Release the GUI lock before cancellation and thread joins.
        if(m_brushSession) check(paint_session_destroy(m_core,&m_brushSession));
        if(m_previewCatalog)check(paint_brush_library_destroy(&m_previewCatalog));
        if (m_session) check(paint_session_destroy(m_core, &m_session));
        if (m_core) check(paint_core_destroy(&m_core));
        {QMutexLocker lock(&m_catalog->mutex);if(m_catalogLease && --m_catalog->workers==0){check(paint_brush_library_destroy(&m_catalog->library));m_catalog->preparation=0;m_catalog->source.clear();}m_catalogLease=false;}
        emit finished();
    }
    void watchLayerCopy(quint64 sequence) {m_layerCopySequence=sequence;}
    void queueBrushOutline(const QString &identity) {m_pendingOutline=identity;}
    void pollBrushOutline() {
        if(m_pendingOutline.isEmpty())return;
        const auto identity=std::exchange(m_pendingOutline,{});const auto bytes=identity.toLatin1();
        PaintBrushLibrary *library=nullptr;{QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
        QByteArray path(4*1024*1024,'\0');uint64_t required=0;
        const auto status=paint_brush_library_tip_outline(library,reinterpret_cast<const uint8_t*>(bytes.constData()),quint64(bytes.size()),reinterpret_cast<uint8_t*>(path.data()),quint64(path.size()),&required);
        if(status==PAINT_OK)path.resize(qsizetype(required));
        emit brushOutlineReady(identity,status==PAINT_OK?QString::fromLatin1(path):QString{},status==PAINT_OK?QString{}:immediateError(status));
    }
    void inspectBrushSource(QString identity,quint64 token) {
        const auto bytes=identity.toLatin1();
        PaintBrushLibrary *library=nullptr;{QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
        QByteArray record(2*1024*1024,'\0');uint64_t required=0;
        const auto status=paint_brush_library_source_info(library,reinterpret_cast<const uint8_t*>(bytes.constData()),quint64(bytes.size()),reinterpret_cast<uint8_t*>(record.data()),quint64(record.size()),&required);
        if(status==PAINT_OK)record.resize(qsizetype(required));
        emit brushSourceReady(identity,token,status==PAINT_OK?record:QByteArray{},status==PAINT_OK?QString{}:immediateError(status));
    }
    void importBrushTip(QString path,QString target,int interpretation,bool texture=false) {
        PaintBrushLibrary *library=nullptr;
        {QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
        const auto encoded=path.toUtf8();QByteArray identity(64,'\0');uint64_t required=0;uint32_t depth=0;
        const auto status=paint_brush_library_import_tip_file(library,reinterpret_cast<const uint8_t*>(encoded.constData()),quint64(encoded.size()),quint32(interpretation),reinterpret_cast<uint8_t*>(identity.data()),64,&required,&depth);
        emit brushTipReady(target,status==PAINT_OK?QString::fromLatin1(identity):QString{},depth,status==PAINT_OK?QString{}:immediateError(status),texture);
    }
    void brushFile(QString path,QByteArray package,bool exporting) {
        if(m_stopped){emit brushFileReady({},QStringLiteral("后端已关闭"),exporting,false);return;}
        const bool abr=!exporting && path.endsWith(".abr",Qt::CaseInsensitive);
        const auto encoded=path.toUtf8();PaintStatus status;
        PaintBrushLibrary *library=nullptr;{QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
        if(exporting)status=paint_brush_library_package_write(library,reinterpret_cast<const uint8_t*>(encoded.constData()),quint64(encoded.size()),reinterpret_cast<const uint8_t*>(package.constData()),quint64(package.size()));
        else {
            package.resize(2*1024*1024);uint64_t required=0;
            status=abr?paint_brush_library_abr_read(library,reinterpret_cast<const uint8_t*>(encoded.constData()),quint64(encoded.size()),reinterpret_cast<uint8_t*>(package.data()),quint64(package.size()),&required):paint_brush_library_package_read(library,reinterpret_cast<const uint8_t*>(encoded.constData()),quint64(encoded.size()),reinterpret_cast<uint8_t*>(package.data()),quint64(package.size()),&required);
            if(status==PAINT_OK)package.resize(qsizetype(required));
        }
        emit brushFileReady(status==PAINT_OK?package:QByteArray{},status==PAINT_OK?QString{}:immediateError(status),exporting,abr);
    }
signals:
    void brushSourceReady(QString identity,quint64 token,QByteArray record,QString error);
    void brushOutlineReady(QString identity,QString path,QString error);
    void brushTipReady(QString target,QString resource,int depth,QString error,bool texture);
    void nativeBrushCatalog(quint64 preparation,QByteArray state,QString error);
    void brushFileReady(QByteArray package,QString error,bool exporting,bool abr);
    void layerClipboardReady(QByteArray bytes);
    void liquifyPreferences(QVariantMap values);
    void brushPreferences(QByteArray json);
    void brushPreviewReady(QString id,quint64 token,QString image);
    void storageState(QVariantMap saved, QVariantMap active, QVariantMap info, QString message);
    void storageDone(bool success);
    void metadata(PaintSessionInfo info, QVariantList layers, QVariantMap selection, QVariantList history,QVariantMap transform);
    void thumbnailReady(quint64 generation, quint64 revision, quint64 layer, QString image);
    void pixels(int view, QImage image, QRectF region, quint64 generation, quint64 revision, quint64 request);
    void failure(QString message);
    void completed(bool workerThread);
    void finished();
    void fileResult(PaintFileJobInfo info, QString message);
public:
    std::unique_ptr<QSettings> preferences() const {
        if (!m_settingsFile.isEmpty()) return std::make_unique<QSettings>(m_settingsFile, QSettings::IniFormat);
        return std::make_unique<QSettings>(QSettings::IniFormat, QSettings::UserScope, "DrawVerse", "DrawVerse");
    }
    static PaintStorageOptions storageOptions(const QVariantMap &values, QByteArray &path) {
        path = values.value("directory").toString().toUtf8();
        auto options = dto<PaintStorageOptions>(); options.path = reinterpret_cast<const uint8_t *>(path.constData());
        options.path_length = static_cast<uint64_t>(path.size());
        // Reject negative/overflowing values before multiplying; Rust validates the exact byte bounds.
        const auto memory = values.value("memoryMiB").toLongLong();
        const auto scratch = values.value("scratchGiB").toLongLong();
        const auto reserve = values.value("reserveMiB").toLongLong();
        if (memory < 1 || memory > 1024 || scratch < 1 || scratch > 64 || reserve < 0 || reserve > 1048576) return options;
        options.resident_bytes = static_cast<uint64_t>(memory) * 1024 * 1024;
        options.scratch_bytes = static_cast<uint64_t>(scratch) * 1024 * 1024 * 1024;
        options.min_free_bytes = static_cast<uint64_t>(reserve) * 1024 * 1024;
        return options;
    }
    void evaluateStorage(const QVariantMap &values, bool save) {
        if (m_stopped) { emit storageDone(false); return; }
        QByteArray path; const auto options = storageOptions(values, path);
        auto info = dto<PaintStorageInfo>();
        const auto status = paint_storage_inspect(&options, &info);
        if (status != PAINT_OK) {
            emit storageState(m_saved, m_active, {}, immediateError(status)); emit storageDone(false); return;
        }
        if (save) {
            auto settings = preferences();
            for (auto it=values.cbegin(); it!=values.cend(); ++it) settings->setValue("storage/"+it.key(),it.value());
            settings->sync();
            if (settings->status()!=QSettings::NoError) {
                emit storageState(m_saved,m_active,{},QStringLiteral("无法保存设置，请检查配置目录权限和可用空间"));
                emit storageDone(false); return;
            }
            m_saved = values;
        }
        const QVariantMap report{{"availableGiB",static_cast<double>(info.available_bytes)/(1024.*1024.*1024.)},
            {"removedRuns",info.removed_runs},{"removedMiB",static_cast<double>(info.removed_bytes)/(1024.*1024.)},
            {"systemDirectory",QDir::tempPath()}};
        emit storageState(m_saved,m_active,report,save ? QStringLiteral("设置已保存，重启 DrawVerse 后生效") : QString());
        emit storageDone(true);
    }
    bool check(PaintStatus status) {
        if (status == PAINT_OK) return true;
        emit failure(immediateError(status)); return false;
    }
    bool publish(const PaintSessionInfo &info) {
        if (m_connection->metadataPending.load()) return false;
        QVariantList layers;
        if (info.document_generation != m_previewGeneration) { m_thumbnails.clear(); m_thumbnailRevisions.clear(); m_previewGeneration = info.document_generation; }
        // Exact publication IDs prevent mixing layers from different edits.
        for (uint32_t i = 0; i < info.layer_count; ++i) {
            auto layer = dto<PaintLayerInfo>();
            auto status = paint_session_layer_info(m_core, m_session, info.publication, i, &layer);
            if (status == PAINT_BUSY) return false;
            if (!check(status)) return false;
            QByteArray name(static_cast<qsizetype>(layer.name_length), '\0'); uint64_t required = 0;
            status = paint_session_layer_name(m_core, m_session, info.publication, layer.layer_id,
                reinterpret_cast<uint8_t *>(name.data()), layer.name_length, &required);
            if (status == PAINT_BUSY) return false;
            if (!check(status)) return false;
            auto appearance = dto<PaintLayerAppearance>();
            status = paint_session_layer_appearance(m_core,m_session,info.publication,layer.layer_id,&appearance);
            if (status == PAINT_BUSY) return false;
            if (!check(status)) return false;
            auto hierarchy=dto<PaintLayerHierarchy>();
            status=paint_session_layer_hierarchy(m_core,m_session,info.publication,layer.layer_id,&hierarchy);
            if(status==PAINT_BUSY) return false;
            if(!check(status)) return false;
            auto clipping=dto<PaintLayerClipping>();
            status=paint_session_layer_clipping(m_core,m_session,info.publication,layer.layer_id,&clipping);
            if(status==PAINT_BUSY) return false;
            if(!check(status)) return false;
            uint32_t image=0;status=paint_session_layer_is_image(m_core,m_session,info.publication,layer.layer_id,&image);if(status==PAINT_BUSY)return false;if(!check(status))return false;
            layers.prepend(QVariantMap{{"id", QVariant::fromValue<qulonglong>(layer.layer_id)},
                {"image",image!=0},{"name", QString::fromUtf8(name)}, {"visible", layer.visible != 0}, {"opacity", layer.opacity},
                {"fill",appearance.fill},{"blendMode",appearance.blend_mode},{"locks",appearance.locks},
                {"mask",hierarchy.kind==PAINT_LAYER_MASK},{"group",hierarchy.kind==PAINT_LAYER_GROUP},{"parent",QVariant::fromValue<qulonglong>(hierarchy.parent_id)},{"depth",hierarchy.depth},{"effectiveLocks",hierarchy.effective_locks},
                {"offsetX",appearance.offset_x},{"offsetY",appearance.offset_y},{"dissolveSeed",appearance.dissolve_seed},
                {"thumbnail",m_thumbnails.value(layer.layer_id)}});
            auto entry=layers.first().toMap();entry.insert("clipped",clipping.enabled!=0);
            entry.insert("clipBase",QVariant::fromValue<qulonglong>(clipping.base_layer_id));layers[0]=entry;
        }
        auto summary=dto<PaintSelectionInfo>();
        auto status=paint_session_selection_info(m_core,m_session,info.publication,&summary);
        if(status==PAINT_BUSY) return false;
        if(!check(status)) return false;
        QVariantList steps;
        for(uint32_t index=0;index<summary.step_count;++index) {
            auto step=dto<PaintSelectionStep>();
            status=paint_session_selection_step(m_core,m_session,info.publication,index,&step);
            if(status==PAINT_BUSY) return false;
            if(!check(status)) return false;
            auto details=dto<PaintSelectionDetails>();QVector<PaintSelectionPoint> path(step.shape==PAINT_SELECTION_POLYGON?4096:0);
            status=paint_session_selection_details(m_core,m_session,info.publication,index,&details,path.data(),static_cast<uint32_t>(path.size()));
            if(status==PAINT_BUSY)return false;if(!check(status))return false;
            QVariantList points;for(uint32_t i=0;i<details.point_count;++i)points.append(QPointF(path[i].x,path[i].y));
            QByteArray mask;
            if(step.shape==PAINT_SELECTION_MASK) {
                uint32_t length=0;status=paint_session_selection_mask(m_core,m_session,info.publication,index,nullptr,0,&length);
                if(status==PAINT_BUSY)return false;if(status!=PAINT_BUFFER_TOO_SMALL && !check(status))return false;
                mask.resize(length);status=paint_session_selection_mask(m_core,m_session,info.publication,index,reinterpret_cast<uint8_t*>(mask.data()),length,&length);
                if(status==PAINT_BUSY)return false;if(!check(status))return false;
            }
            steps.append(QVariantMap{{"mask",mask},{"points",points},{"feather",details.feather},{"antialias",step.antialias!=0},{"operation",step.operation},{"shape",step.shape},{"x",step.x},{"y",step.y},{"width",step.width},{"height",step.height}});
        }
        QVariantList history;
        // One entry per HistoryAction discriminant, in order. A missing entry used to fail the whole
        // publish ("无法识别的历史操作"), which silently froze every later UI update, so the list must
        // cover every kind the core can report and the check below treats the rest as unknown text.
        const QStringList names{QStringLiteral("初始状态"),QStringLiteral("画笔"),QStringLiteral("橡皮擦"),QStringLiteral("全选"),QStringLiteral("矩形选区"),QStringLiteral("取消选区"),QStringLiteral("反选"),QStringLiteral("新建图层"),QStringLiteral("删除图层"),QStringLiteral("图层属性"),QStringLiteral("混合模式"),QStringLiteral("图层填充"),QStringLiteral("图层锁定"),QStringLiteral("移动图层"),QStringLiteral("新建图层组"),QStringLiteral("取消图层组"),QStringLiteral("图层归组"),QStringLiteral("添加蒙版"),QStringLiteral("剪贴蒙版"),QString(),QStringLiteral("椭圆选区"),QStringLiteral("套索"),QStringLiteral("魔棒"),QStringLiteral("释放剪贴蒙版"),QStringLiteral("多边形套索"),QStringLiteral("油漆桶"),QStringLiteral("涂抹"),QStringLiteral("复制图层"),QStringLiteral("粘贴图层"),QStringLiteral("合并图层"),QStringLiteral("液化"),QStringLiteral("液化 · 推拉"),QStringLiteral("液化 · 膨大"),QStringLiteral("液化 · 扭曲"),QStringLiteral("液化 · 重建"),QStringLiteral("液化 · 重置"),QStringLiteral("置入图片"),QStringLiteral("自由变换"),QStringLiteral("栅格化图片")};
        const QStringList icons{"page","brush","eraser","rectangleSelection","rectangleSelection","rectangleSelection","rectangleSelection","plus","trash","layers","layers","layers","layers","move","folder","folder","folder","mask","layers","page","ellipseSelection","lassoSelection","wand","layers","polygonSelection","bucket","smudge","layers","layers","layers","liquify","liquifyPush","liquifyBloat","liquifyTwirl","liquifyReconstruct","undo","page","move","layers"};
        for(uint32_t depth=0;depth<=info.undo_depth+info.redo_depth;++depth) {
            auto entry=dto<PaintHistoryEntry>();auto historyStatus=paint_session_history_entry(m_core,m_session,info.publication,depth,&entry);
            if(historyStatus==PAINT_BUSY)return false;if(!check(historyStatus))return false;
            if(entry.kind==PAINT_HISTORY_TRUNCATED)continue;
            // An unknown kind is shown as generically as possible instead of aborting the publication:
            // dropping it would leave the canvas, layer list and selection stale for good.
            const bool known=entry.kind<static_cast<uint32_t>(names.size());
            history.append(QVariantMap{{"depth",entry.depth},{"kind",entry.kind},
                {"title",entry.kind==PAINT_HISTORY_INITIAL && (info.flags&PAINT_SESSION_LIQUIFY)?QStringLiteral("液化起始状态"):entry.kind==PAINT_HISTORY_INITIAL && (info.flags&PAINT_SESSION_TRANSFORM)?QStringLiteral("变换起始状态"):entry.kind==PAINT_HISTORY_RELEASE_CLIPPING?QStringLiteral("释放剪贴蒙版"):known?names[entry.kind]:QStringLiteral("未知操作")},
                {"icon",entry.kind==PAINT_HISTORY_RELEASE_CLIPPING?QStringLiteral("layers"):known?icons[entry.kind]:QStringLiteral("page")}});
        }
        auto transform=dto<PaintTransform>();status=paint_session_transform_info(m_core,m_session,info.publication,&transform);if(status==PAINT_BUSY)return false;if(!check(status))return false;QVariantList transformPoints;for(int i=0;i<16;++i)transformPoints.append(QPointF(transform.points[i*2],transform.points[i*2+1]));
        m_connection->metadataPending.store(true);
        emit metadata(info, layers,{{"enabled",summary.enabled!=0},{"steps",steps}},history,{{"points",transformPoints},{"warp",transform.warp!=0},{"interpolation",transform.interpolation},{"skewH",transform.skew_h},{"skewV",transform.skew_v}}); m_publication = info.publication;
        return true;
    }
    void poll() {
        if (!m_session || m_stopped) return;
        pollBrushState();
        pollBrushOutline();
        auto info = dto<PaintSessionInfo>();
        if (!check(paint_session_info(m_core, m_session, &info))) return;
        if(info.completed_sequence<m_bootSequence) return;
        if ((info.publication != m_publication || info.flags != m_flags) && !publish(info)) return;
        if(m_layerCopySequence && info.completed_sequence>=m_layerCopySequence) {
            uint64_t size=0;
            auto status=paint_session_read_layer_clipboard(m_core,m_session,m_layerCopySequence,nullptr,0,&size);
            QByteArray copied;
            if(status==PAINT_BUFFER_TOO_SMALL && size<=256ull*1024*1024) {
                QByteArray bytes(static_cast<qsizetype>(size),'\0');
                if(paint_session_read_layer_clipboard(m_core,m_session,m_layerCopySequence,reinterpret_cast<uint8_t*>(bytes.data()),size,&size)==PAINT_OK)copied=std::move(bytes);
            }
            m_layerCopySequence=0;emit layerClipboardReady(copied);
        }
        m_flags = info.flags;
        pollPreviews(info);
        if(info.stroke_active && !m_brushJob.isEmpty())m_brushClock.restart();
        if(!info.stroke_active && m_previewClock.elapsed()-m_quietSince>=200) pollBrushPreview();
        if (info.completed_sequence != m_completed) {
            m_completed = info.completed_sequence; emit completed(QThread::currentThread() == thread());
        }
        if (info.last_error_status != PAINT_OK &&
            (info.last_error_sequence != m_errorSequence || info.last_error_status != m_errorStatus)) {
            uint64_t size = 0;
            auto status = paint_session_error_message(m_core, m_session, info.last_error_sequence, nullptr, 0, &size);
            if (status == PAINT_BUFFER_TOO_SMALL || status == PAINT_OK) {
                QByteArray message(static_cast<qsizetype>(size), '\0');
                if (paint_session_error_message(m_core, m_session, info.last_error_sequence,
                    reinterpret_cast<uint8_t *>(message.data()), size, &size) == PAINT_OK) {
                    emit failure(QStringLiteral("任务错误 %1：%2").arg(info.last_error_status).arg(QString::fromUtf8(message)));
                    m_errorSequence = info.last_error_sequence;
                    m_errorStatus = info.last_error_status;
                }
            }
        }
        if (info.flags & PAINT_SESSION_FAILED) { m_timer->stop(); return; }
        const auto fileJob = m_connection->fileJob.load();
        if (fileJob && fileJob != m_deliveredFile) {
            auto file = dto<PaintFileJobInfo>();
            if (check(paint_session_file_info(m_core, m_session, fileJob, &file)) && file.state >= PAINT_FILE_SUCCEEDED) {
                // The actor publishes document metadata immediately after the file result.
                // Deliver metadata first so Save/Discard decisions see the current document.
                if (file.state == PAINT_FILE_SUCCEEDED &&
                    ((file.kind == PAINT_FILE_OPEN && info.document_generation != file.result_generation) ||
                     (file.kind == PAINT_FILE_SAVE && file.format == PAINT_FILE_OPENRASTER &&
                      info.document_generation == file.source_generation && info.revision == file.source_revision &&
                      (info.flags & PAINT_SESSION_MODIFIED)))) return;
                uint64_t size = 0;
                auto status = paint_session_file_message(m_core, m_session, fileJob, nullptr, 0, &size);
                if (status == PAINT_OK || status == PAINT_BUFFER_TOO_SMALL) {
                    QByteArray message(static_cast<qsizetype>(size), '\0');
                    if (check(paint_session_file_message(m_core, m_session, fileJob,
                        reinterpret_cast<uint8_t *>(message.data()), size, &size))) {
                        m_deliveredFile = fileJob; emit fileResult(file, QString::fromUtf8(message));
                    }
                }
            }
        }
        for (uint32_t view = 0; view < 2; ++view) {
            // At most one queued QImage per view when the GUI is busy.
            if (m_connection->framePending[view].load()) continue;
            auto frame = dto<PaintFrameInfo>();
            auto status = paint_session_frame_info(m_core, m_session, view, &frame);
            if (status == PAINT_BUSY) continue;
            if (!check(status)) return;
            if (frame.frame_id == m_frames[view]) continue;
            QImage image(static_cast<int>(frame.pixel_width), static_cast<int>(frame.pixel_height), QImage::Format_RGBA8888_Premultiplied);
            if (image.isNull()) { emit failure(QStringLiteral("无法分配视口显示缓存")); return; }
            auto tile = dto<PaintTile>(); tile.format = PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED;
            tile.data = image.bits(); tile.capacity = static_cast<uint64_t>(image.sizeInBytes());
            tile.stride = static_cast<uint64_t>(image.bytesPerLine());
            status = paint_session_read_frame(m_core, m_session, view, frame.request_id, frame.frame_id, &tile);
            if (status == PAINT_BUSY) continue;
            if (!check(status)) return;
            m_frames[view] = frame.frame_id;
            m_connection->framePending[view].store(true);
            emit pixels(static_cast<int>(view), image, QRectF(frame.x, frame.y, frame.width, frame.height),
                frame.document_generation, frame.revision, frame.request_id);
        }
    }
    // One background preview at a time, only after a quiet document interval.
    // Slot 2 is bounded by the core and is cancelled whenever painting resumes.
    void disablePreview(const PaintSessionInfo &info) {
        auto view=dto<PaintViewport>(); view.view_id=2; view.document_generation=info.document_generation;
        uint64_t ignored=0; paint_session_set_viewport(m_core,m_session,&view,&ignored); m_previewRequest=0;
    }
    void pollPreviews(const PaintSessionInfo &info) {
        const auto now=m_previewClock.elapsed();
        if(info.document_generation!=m_seenGeneration || info.revision!=m_seenRevision || info.stroke_active) {
            m_seenGeneration=info.document_generation; m_seenRevision=info.revision; m_quietSince=now;
            m_previewIndex=0;
            if(m_previewRequest) disablePreview(info);
            return;
        }
        if(now-m_quietSince<200) return;
        if(m_previewRequest) {
            auto frame=dto<PaintFrameInfo>();
            const auto status=paint_session_frame_info(m_core,m_session,2,&frame);
            if(status==PAINT_BUSY) return;
            if(status!=PAINT_OK) { disablePreview(info); ++m_previewIndex; return; }
            if(frame.request_id!=m_previewRequest || frame.document_generation!=info.document_generation || frame.revision!=info.revision) return;
            QImage image(static_cast<int>(frame.pixel_width),static_cast<int>(frame.pixel_height),QImage::Format_RGBA8888_Premultiplied);
            if(image.isNull()) { disablePreview(info); ++m_previewIndex; return; }
            auto tile=dto<PaintTile>(); tile.format=PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED;
            tile.data=image.bits(); tile.capacity=static_cast<uint64_t>(image.sizeInBytes()); tile.stride=static_cast<uint64_t>(image.bytesPerLine());
            if(paint_session_read_frame(m_core,m_session,2,frame.request_id,frame.frame_id,&tile)!=PAINT_OK) return;
            QByteArray png; QBuffer buffer(&png); buffer.open(QIODevice::WriteOnly);
            if(image.save(&buffer,"PNG")) {
                m_thumbnails[m_previewLayer]="data:image/png;base64,"+QString::fromLatin1(png.toBase64());
                m_thumbnailRevisions[m_previewLayer]=info.revision;
                while(m_thumbnails.size()>128) {
                    auto victim=m_thumbnails.begin();
                    while(victim!=m_thumbnails.end() && m_previewReferences.contains(victim.key())) ++victim;
                    if(victim==m_thumbnails.end()) break;
                    m_thumbnailRevisions.remove(victim.key()); m_thumbnails.erase(victim);
                }
                // A preview changes no document metadata. Updating the entire layer list here
                // rebuilt every visible delegate, which re-requested all previews on every image.
                emit thumbnailReady(info.document_generation,info.revision,m_previewLayer,m_thumbnails[m_previewLayer]);
            }
            disablePreview(info); ++m_previewIndex;
        }
        while(m_previewIndex<m_previewTargets.size() && m_thumbnailRevisions.contains(m_previewTargets[m_previewIndex]) &&
              m_thumbnailRevisions.value(m_previewTargets[m_previewIndex])==info.revision) ++m_previewIndex;
        if(m_previewIndex>=m_previewTargets.size()) return;
        const auto layerId=m_previewTargets[m_previewIndex];
        auto appearance=dto<PaintLayerAppearance>();
        if(paint_session_layer_appearance(m_core,m_session,info.publication,layerId,&appearance)!=PAINT_OK) { ++m_previewIndex; return; }
        auto view=dto<PaintViewport>(); view.view_id=2; view.enabled=1; view.document_generation=info.document_generation;
        view.width=info.width; view.height=info.height;
        const double scale=48./std::max(info.width,info.height);
        view.pixel_width=std::max(1u,static_cast<uint32_t>(std::round(info.width*scale)));
        view.pixel_height=std::max(1u,static_cast<uint32_t>(std::round(info.height*scale)));
        if(paint_session_set_layer_preview(m_core,m_session,layerId,&view,&m_previewRequest)==PAINT_OK) m_previewLayer=layerId;
    }
    // Preview documents run in their own Rust actor; no mutation of the user's document.
    void pollBrushState() {
        PaintBrushLibrary *library=nullptr;quint64 preparation=0;QByteArray wanted;
        {QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;preparation=m_catalog->preparation;wanted=m_catalog->preferences;}
        if(!library || !preparation){m_pendingBrushPreferences.clear();return;}
        if(m_pendingBrushPreferences.isEmpty() && preparation==m_reportedBrushPreparation)return;
        QByteArray canonical(2*1024*1024,'\0');uint64_t required=0;
        const auto status=paint_brush_library_prepared_state(library,preparation,reinterpret_cast<uint8_t*>(canonical.data()),quint64(canonical.size()),&required);
        if(status==PAINT_BUSY)return;
        QString error;
        if(status!=PAINT_OK){error=immediateError(status);if(paint_brush_library_current_state(library,reinterpret_cast<uint8_t*>(canonical.data()),quint64(canonical.size()),&required)!=PAINT_OK)canonical.clear();}
        if(!canonical.isEmpty())canonical.resize(qsizetype(required));
        if(preparation!=m_reportedBrushPreparation){m_reportedBrushPreparation=preparation;emit nativeBrushCatalog(preparation,canonical,error);}
        if(!m_pendingBrushPreferences.isEmpty()){
            const auto pending=std::exchange(m_pendingBrushPreferences,{});
            if(pending!=wanted || !error.isEmpty())return;
            auto root=QJsonDocument::fromJson(pending).object();const auto native=QJsonDocument::fromJson(canonical).object();root["presets"]=native.value("presets");root["selected"]=native.value("selected");root["version"]=native.value("version");
            auto settings=preferences();settings->setValue("brushes/state",QJsonDocument(root).toJson(QJsonDocument::Compact));settings->sync();if(settings->status()!=QSettings::NoError)emit failure(QStringLiteral("无法保存画笔设置，请检查配置目录权限"));
        }
    }
    void pollBrushPreview() {
        if(m_brushJob.isEmpty()) {
            if(m_brushQueue.isEmpty())return;
            m_brushJob=m_brushQueue.takeFirst().toMap();m_brushRequest=0;m_brushClock.restart();
            if(!m_brushSession){auto desc=dto<PaintDocumentDesc>();desc.width=240;desc.height=60;desc.working_space=PAINT_WORKING_LINEAR_SRGB;desc.pixel_format=PAINT_STORAGE_RGBA32F_PREMULTIPLIED;if(paint_session_create(m_core,&desc,&m_brushSession)!=PAINT_OK){finishBrushPreview({});return;}}
            auto command=dto<PaintCommand>();command.kind=PAINT_COMMAND_NEW_DOCUMENT;command.width=240;command.height=60;
            if(paint_session_submit(m_core,m_brushSession,&command,&m_brushSequence)!=PAINT_OK){finishBrushPreview({});return;}
            command=dto<PaintCommand>();command.kind=PAINT_COMMAND_BEGIN_STROKE;command.stroke=dto<PaintStrokeDesc>();command.stroke.mode=PAINT_MODE_PAINT;
            command.stroke.radius=static_cast<float>(std::clamp(m_brushJob.value("radius").toDouble()*.35,2.,18.));command.stroke.opacity=m_brushJob.value("opacity").toFloat();command.stroke.spacing=m_brushJob.value("spacing").toFloat();
            for(auto &v:command.stroke.linear_rgba)v=1;
            if(!m_previewCatalog){
                if(paint_brush_library_create(0,&m_previewCatalog)!=PAINT_OK){finishBrushPreview({});return;}
                PaintBrushLibrary *library=nullptr;{QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
                if(paint_brush_library_share_resources(m_previewCatalog,library)!=PAINT_OK){paint_brush_library_destroy(&m_previewCatalog);finishBrushPreview({});return;}
            }
            const QByteArray presetId="round-pressure";
            BrushLibrary draft;auto state=QJsonDocument::fromJson(draft.catalogSnapshot()).object();auto rows=state.value("presets").toArray();auto row=rows[0].toObject();const auto tip=m_brushJob.value("tip").toMap();
            row["radius"]=24.;row["opacity"]=command.stroke.opacity;row["spacing"]=command.stroke.spacing;
            for(const auto &key:{"hardness","roundness","angle","flow","pressureSize","pressureFlow"})row[key]=QJsonValue::fromVariant(tip.value(key));
            if(!tip.value("maskResource").toString().isEmpty()){row["maskResource"]=tip.value("maskResource").toString();state["version"]=3;}
            if(!tip.value("pressureDynamics").toMap().isEmpty()){row["pressureDynamics"]=QJsonObject::fromVariantMap(tip.value("pressureDynamics").toMap());state["version"]=tip.value("pressureDynamics").toMap().contains("opacity")?6:5;}
            if(!tip.value("tipDynamics").toMap().isEmpty()){row["tipDynamics"]=QJsonObject::fromVariantMap(tip.value("tipDynamics").toMap());state["version"]=7;}
            if(!tip.value("scatterDynamics").toMap().isEmpty()){row["scatterDynamics"]=QJsonObject::fromVariantMap(tip.value("scatterDynamics").toMap());state["version"]=8;}
            if(!tip.value("shapeJitter").toMap().isEmpty()){row["shapeJitter"]=QJsonObject::fromVariantMap(tip.value("shapeJitter").toMap());state["version"]=9;}
            if(!tip.value("colorDynamics").toMap().isEmpty()){row["colorDynamics"]=QJsonObject::fromVariantMap(tip.value("colorDynamics").toMap());state["version"]=10;}
            if(!tip.value("texture").toMap().isEmpty()){row["texture"]=QJsonObject::fromVariantMap(tip.value("texture").toMap());state["version"]=11;}
            rows[0]=row;state["presets"]=rows;const auto json=QJsonDocument(state).toJson(QJsonDocument::Compact);uint64_t preparation=0;
            if(paint_brush_library_prepare_state(m_previewCatalog,reinterpret_cast<const uint8_t*>(json.constData()),quint64(json.size()),&preparation)!=PAINT_OK){finishBrushPreview({});return;}
            auto request=dto<PaintBrushToolDesc>();request.radius=command.stroke.radius;request.strength=1;for(auto &v:request.linear_rgba)v=1;
            auto background=dto<PaintBrushBackgroundDesc>();for(auto &v:background.linear_rgba)v=1;
            if(!tip.value("colorDynamics").toMap().isEmpty()){request.linear_rgba[0]=1;request.linear_rgba[1]=request.linear_rgba[2]=0;background.linear_rgba[0]=background.linear_rgba[1]=0;background.linear_rgba[2]=1;}
            for(int i=0;i<=32;++i){const double t=i/32.;InputSample sample;sample.position={20+200*t,30+8*std::sin(t*6.283185307)};sample.pressure=static_cast<float>(.25+.75*std::sin(t*3.141592654));sample.tool=PAINT_TOOL_PEN;if(!tip.value("tipDynamics").toMap().isEmpty()){sample.capabilities=PAINT_INPUT_PRESSURE|PAINT_INPUT_TILT|PAINT_INPUT_ROTATION;sample.rotation=static_cast<float>(-60+120*t);sample.tiltX=static_cast<float>(30+30*std::sin(t*3.141592654));sample.tiltY=20;}command.point=point(sample);command.kind=PAINT_COMMAND_STROKE_TO;
                const auto status=i==0?paint_session_begin_prepared_preset_with_background(m_core,m_brushSession,m_previewCatalog,preparation,reinterpret_cast<const uint8_t*>(presetId.constData()),quint64(presetId.size()),&request,&background,&command.point,&m_brushSequence):paint_session_submit(m_core,m_brushSession,&command,&m_brushSequence);
                if(status!=PAINT_OK){finishBrushPreview({});return;}}
            command=dto<PaintCommand>();command.kind=PAINT_COMMAND_END_STROKE;if(paint_session_submit(m_core,m_brushSession,&command,&m_brushSequence)!=PAINT_OK){finishBrushPreview({});return;}
            return;
        }
        if(m_brushClock.elapsed()>3000){finishBrushPreview({});return;}
        auto info=dto<PaintSessionInfo>();if(paint_session_info(m_core,m_brushSession,&info)!=PAINT_OK){finishBrushPreview({});return;}
        if(info.completed_sequence<m_brushSequence)return;
        if(info.last_error_status!=PAINT_OK){finishBrushPreview({});return;}
        if(!m_brushRequest){auto view=dto<PaintViewport>();view.view_id=0;view.enabled=1;view.width=240;view.height=60;view.pixel_width=240;view.pixel_height=60;view.document_generation=info.document_generation;if(paint_session_set_viewport(m_core,m_brushSession,&view,&m_brushRequest)!=PAINT_OK)finishBrushPreview({});return;}
        auto frame=dto<PaintFrameInfo>();const auto status=paint_session_frame_info(m_core,m_brushSession,0,&frame);if(status==PAINT_BUSY)return;
        if(status!=PAINT_OK){finishBrushPreview({});return;}
        if(frame.request_id!=m_brushRequest || frame.document_generation!=info.document_generation || frame.revision!=info.revision)return;
        QImage image(240,60,QImage::Format_RGBA8888_Premultiplied);auto tile=dto<PaintTile>();tile.format=PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED;tile.data=image.bits();tile.capacity=image.sizeInBytes();tile.stride=image.bytesPerLine();
        if(paint_session_read_frame(m_core,m_brushSession,0,frame.request_id,frame.frame_id,&tile)!=PAINT_OK){finishBrushPreview({});return;}
        QByteArray png;QBuffer output(&png);output.open(QIODevice::WriteOnly);if(!image.save(&output,"PNG")){finishBrushPreview({});return;}finishBrushPreview(QStringLiteral("data:image/png;base64,")+QString::fromLatin1(png.toBase64()));
    }
    void finishBrushPreview(const QString &image) {
        emit brushPreviewReady(m_brushJob.value("id").toString(),m_brushJob.value("token").toULongLong(),image);
        if(m_brushSession){auto view=dto<PaintViewport>();view.view_id=0;uint64_t ignored=0;paint_session_set_viewport(m_core,m_brushSession,&view,&ignored);}
        if(image.isEmpty() && m_brushSession)paint_session_destroy(m_core,&m_brushSession);
        m_brushJob.clear();m_brushRequest=0;
    }
    PaintSession *m_brushSession=nullptr;
    PaintBrushLibrary *m_previewCatalog=nullptr;
    std::shared_ptr<BrushCatalogConnection> m_catalog;
    bool m_catalogLease=false;
    QByteArray m_pendingBrushPreferences;
    quint64 m_reportedBrushPreparation=0;
    QString m_pendingOutline;
    QVariantList m_brushQueue;
    QVariantMap m_brushJob;
    uint64_t m_brushSequence=0,m_brushRequest=0;
    QElapsedTimer m_brushClock;
    QHash<quint64,QString> m_thumbnails;
    QHash<quint64,quint64> m_thumbnailRevisions;
    QHash<quint64,int> m_previewReferences;
    QList<quint64> m_previewTargets;
    QElapsedTimer m_previewClock;
    qint64 m_quietSince=0;
    quint64 m_previewGeneration=0,m_seenGeneration=0,m_seenRevision=0,m_previewRequest=0,m_previewLayer=0;
    qsizetype m_previewIndex=0;
    uint64_t m_bootSequence=0;
    QString m_settingsFile;
    QVariantMap m_saved, m_active;
    // Persisted history command limit, loaded with the other preferences on boot.
    int m_historyLimit = 100;
    std::shared_ptr<BackendConnection> m_connection;
    QTimer *m_timer;
    PaintCore *m_core = nullptr;
    PaintSession *m_session = nullptr;
    quint64 m_layerCopySequence=0;
    quint64 m_publication = 0, m_completed = 0, m_errorSequence = 0, m_frames[2]{};
    quint64 m_deliveredFile = 0;
    uint32_t m_flags = 0;
    PaintStatus m_errorStatus = PAINT_OK;
    bool m_stopped = false;
};

PaintCoreClient::PaintCoreClient(QObject *parent, const QString &settingsFile,std::shared_ptr<BrushCatalogConnection> brushCatalog) : QObject(parent), m_settingsFile(settingsFile), m_connection(std::make_shared<BackendConnection>()),m_brushCatalogConnection(brushCatalog?std::move(brushCatalog):std::make_shared<BrushCatalogConnection>()) {
    connect(QGuiApplication::clipboard(),&QClipboard::dataChanged,this,&PaintCoreClient::stateChanged);
    // Focus changes can occur inside a QML enabled binding. Notify only the
    // shortcuts that depend on focus, without reentering document/busy bindings.
    connect(qGuiApp,&QGuiApplication::focusObjectChanged,this,[this]{emit layerShortcutsChanged();});
    connect(this,&PaintCoreClient::stateChanged,this,&PaintCoreClient::layerShortcutsChanged);
    m_brushLibrary=new BrushLibrary(this);
    qRegisterMetaType<PaintSessionInfo>();
    qRegisterMetaType<PaintFileJobInfo>();
    m_worker = new BackendWorker(m_connection, settingsFile,m_brushCatalogConnection); m_worker->moveToThread(&m_thread);
    connect(m_worker,&BackendWorker::liquifyPreferences,this,[this](const QVariantMap &values){
        m_liquifyTool=std::clamp(values.value("tool").toInt(),0,3);m_liquifySize=std::clamp(values.value("size").toInt(),1,2000);m_liquifyStrength=std::clamp(values.value("strength").toInt(),1,100);m_liquifyDensity=std::clamp(values.value("density").toInt(),1,100);m_liquifySpeed=std::clamp(values.value("speed").toInt(),1,100);m_liquifyPressure=values.value("pressure").toBool();emit liquifyOptionsChanged();
    });
    connect(m_worker,&BackendWorker::layerClipboardReady,this,[this](const QByteArray &bytes){
        if(m_closing)return;
        m_pendingCopy=0;
        if(!bytes.isEmpty()){auto *mime=new QMimeData;mime->setData("application/x-drawverse-layers-v1",bytes);QGuiApplication::clipboard()->setMimeData(mime);}
        emit stateChanged();
    });
    connect(m_worker,&BackendWorker::brushPreferences,m_brushLibrary,[this](const QByteArray &json){
        QByteArray source=json;{QMutexLocker lock(&m_brushCatalogConnection->mutex);if(!m_brushCatalogConnection->preferences.isEmpty())source=m_brushCatalogConnection->preferences;}
        if(!source.isEmpty() && !m_brushLibrary->restore(source))showError(QStringLiteral("无法恢复笔刷设置，原配置已保留"));
        synchronizeBrushCatalog(m_brushLibrary->snapshot());
    });
    connect(m_worker,&BackendWorker::nativeBrushCatalog,this,[this](quint64 preparation,const QByteArray &state,const QString &error){
        {QMutexLocker lock(&m_brushCatalogConnection->mutex);if(preparation!=m_brushCatalogConnection->preparation)return;}
        if(!state.isEmpty() && m_brushLibrary->applyNativeCatalog(state)){QMutexLocker lock(&m_brushCatalogConnection->mutex);if(preparation==m_brushCatalogConnection->preparation)m_brushCatalogConnection->source=error.isEmpty()?m_brushLibrary->catalogSnapshot():QByteArray{};}
        if(!error.isEmpty())showError(error);
    });
    connect(m_worker,&BackendWorker::brushPreviewReady,m_brushLibrary,&BrushLibrary::acceptPreview);
    connect(m_worker,&BackendWorker::brushOutlineReady,this,[this](const QString &identity,const QString &path,const QString &error){
        if(m_drawing && identity==m_strokeOutlineResource){m_strokeOutlinePath=path;m_strokeOutlineReady=error.isEmpty();emit brushOutlineChanged();}
        if(identity!=m_outlineResource || identity!=m_brushLibrary->maskResource())return;
        m_brushOutlinePath=path;m_brushOutlineReady=error.isEmpty();m_brushOutlineMessage=error.isEmpty()?QString{}:QStringLiteral("图片轮廓暂不可用，显示简化轮廓：")+error;emit brushOutlineChanged();
    });
    connect(m_worker,&BackendWorker::brushSourceReady,this,[this](const QString &identity,quint64 token,const QByteArray &record,const QString &error){
        if(m_closing || token!=m_brushSourceToken || identity!=m_brushLibrary->sourceResource())return;
        m_brushSourceBusy=false;m_brushSourceInfo=error.isEmpty()?QJsonDocument::fromJson(record).object().toVariantMap():QVariantMap{};
        m_brushSourceMessage=error;emit brushSourceChanged();
    });
    connect(m_brushLibrary,&BrushLibrary::settingsChanged,this,[this]{
        const auto identity=m_brushLibrary->sourceResource();if(identity==m_brushSourceIdentity)return;
        m_brushSourceIdentity=identity;++m_brushSourceToken;m_brushSourceInfo.clear();m_brushSourceMessage.clear();m_brushSourceBusy=false;emit brushSourceChanged();
    });
    connect(m_worker,&BackendWorker::brushTipReady,this,[this](const QString &target,const QString &resource,int depth,const QString &error,bool texture){
        bool success=error.isEmpty();
        if(success && !(texture?m_brushLibrary->setTextureResource(target,resource):m_brushLibrary->setMaskResource(target,resource))){success=false;showError(QStringLiteral("目标笔刷已删除，未应用图片资源"));}
        if(!error.isEmpty())showError(error);
        m_brushImportMessage=success?(depth==16?QStringLiteral("已导入：16 位覆盖已转换为 8 位"):texture?QStringLiteral("纹理已导入"):QStringLiteral("图片笔尖已导入")):QString{};
        m_brushFileBusy=false;emit brushFileChanged();emit brushFileFinished(success);
    });
    connect(m_brushLibrary,&BrushLibrary::cursorChanged,this,&PaintCoreClient::brushCursorPreferencesChanged);
    connect(m_worker,&BackendWorker::brushFileReady,this,[this](const QByteArray &package,const QString &error,bool exporting,bool abr){
        bool success=error.isEmpty();
        if(success && !exporting && !(abr?m_brushLibrary->importAbr(package):m_brushLibrary->importPackage(package))){success=false;showError(QStringLiteral("导入失败：笔刷库容量不足或包参数无效（最多 1024 个预设）"));}
        if(!error.isEmpty())showError(error);
        if(success && abr) {
            const auto rows=QJsonDocument::fromJson(package).object().value("presets").toArray();int partial=0;for(const auto &row:rows)if(row.toObject().value("partial").toBool())++partial;
            m_brushImportMessage=QStringLiteral("已导入 %1 个 ABR 笔刷（%2 个部分转换）；在画笔设置的“来源与兼容”查看报告").arg(rows.size()).arg(partial);
        }
        m_brushFileBusy=false;emit brushFileChanged();emit brushFileFinished(success);
    });
    connect(m_brushLibrary,&BrushLibrary::persistRequested,this,&PaintCoreClient::persistBrushPreferences);
    connect(m_brushLibrary,&BrushLibrary::previewRequested,m_worker,&BackendWorker::queueBrushPreview);
    connect(m_brushLibrary,&BrushLibrary::settingsChanged,this,[this]{if(m_hoverPose)paint_brush_pose_clear(m_hoverPose);m_hoverPosePreparation=0;m_poseExpectedSource=m_brushLibrary->catalogSnapshot();m_radius=m_brushLibrary->radius();m_opacity=m_brushLibrary->opacity();m_spacing=m_brushLibrary->spacing();m_liveBasePose={false,m_radius,m_brushLibrary->angle(),m_brushLibrary->roundness()};emit brushChanged();});
    connect(m_brushLibrary,&BrushLibrary::settingsChanged,this,[this]{
        const auto identity=m_brushLibrary->maskResource();if(identity==m_outlineResource)return;
        m_outlineResource=identity;m_brushOutlinePath.clear();m_brushOutlineMessage.clear();m_brushOutlineReady=false;emit brushOutlineChanged();
        QMetaObject::invokeMethod(m_worker,[worker=m_worker,identity]{worker->queueBrushOutline(identity);},Qt::QueuedConnection);
    });
    connect(m_brushLibrary,&BrushLibrary::recentColorsChanged,this,&PaintCoreClient::recentColorsChanged);
    connect(m_worker,&BackendWorker::storageState,this,[this](QVariantMap saved,QVariantMap active,QVariantMap info,QString message) {
        m_storageSettings=std::move(saved); m_activeStorageSettings=std::move(active);
        m_storageInfo=std::move(info); m_storageMessage=std::move(message); emit storageChanged();
    });
    connect(m_worker,&BackendWorker::storageDone,this,[this](bool success) {
        m_storageBusy=false; emit storageChanged(); emit storageFinished(success);
    });
    connect(&m_thread, &QThread::started, m_worker, &BackendWorker::boot);
    connect(m_worker, &BackendWorker::metadata, this, [this](PaintSessionInfo info, QVariantList layers, QVariantMap selection, QVariantList history,QVariantMap transform) {
        m_connection->metadataPending.store(false);
        const bool propertiesChanged = info.document_generation != m_generation
            || static_cast<int>(info.width) != m_width || static_cast<int>(info.height) != m_height
            || static_cast<int>(info.undo_depth) != m_undo || static_cast<int>(info.redo_depth) != m_redo
            || info.active_layer_id != m_active;
        const bool wasLiquify=m_liquifyActive,wasLiquifyBusy=liquifyBusy();
        const bool wasReady = m_ready, wasModified = modified(), wasLayerBusy=layerEditBusy();
        const bool changedLayers = m_layers != layers;
        const bool activeChanged = info.active_layer_id != m_active;
        const bool changedGeneration = info.document_generation != m_generation;
        if(changedGeneration){m_blendPreviewLayer=0;m_layerThumbnails.clear();emit layerThumbnailsChanged();}
        else if(m_blendPreviewLayer && (info.revision!=m_revision || info.active_layer_id!=m_active))clearLayerBlendPreview();
        if(changedGeneration) m_nextDefaultLayer=1;
        if(changedGeneration && !m_collapsedGroups.isEmpty()) {m_collapsedGroups.clear(); emit groupExpansionChanged();}
        if(info.completed_sequence>=m_pendingLiquify) {m_pendingLiquify=0;m_liquifyActive=(info.flags&PAINT_SESSION_LIQUIFY)!=0;}
        const bool publishedTransform=(info.flags&PAINT_SESSION_TRANSFORM)!=0;
        if(publishedTransform && !m_transformPublished){m_transformInitialPoints=transform.value("points").toList();emit transformChanged();}
        m_transformPublished=publishedTransform;
        if(!publishedTransform && m_transformActive && info.completed_sequence>=m_pendingTransform && m_afterTransform){auto action=std::move(m_afterTransform);QTimer::singleShot(0,this,std::move(action));}
        if(info.completed_sequence>=m_pendingTransform)m_transformActive=(info.flags&PAINT_SESSION_TRANSFORM)!=0;
        if(info.completed_sequence>=m_lastTransform){const auto points=transform.value("points").toList();const bool warp=transform.value("warp").toBool();const int interpolation=transform.value("interpolation").toInt();const auto skewH=transform.value("skewH").toDouble(),skewV=transform.value("skewV").toDouble();if(points!=m_transformPoints || warp!=m_transformWarp || interpolation!=m_transformInterpolation || skewH!=m_transformSkewH || skewV!=m_transformSkewV){m_transformPoints=points;m_transformWarp=warp;m_transformInterpolation=interpolation;m_transformSkewH=skewH;m_transformSkewV=skewV;emit transformChanged();}}
        m_completedSequence=info.completed_sequence;
        m_generation = info.document_generation;
        m_revision = info.revision; m_width = static_cast<int>(info.width); m_height = static_cast<int>(info.height);
        m_undo = static_cast<int>(info.undo_depth); m_redo = static_cast<int>(info.redo_depth);
        if (info.completed_sequence >= m_pendingMutation) m_pendingMutation = 0;
        if (info.completed_sequence >= m_pendingLayer) m_pendingLayer=0;
        // A lasso/wand edit is a busy layer edit too: keep the panel and painting idled until the
        // worker published the resulting selection.

        m_active = info.active_layer_id;
        const bool historyLiquify=(info.flags&(PAINT_SESSION_LIQUIFY|PAINT_SESSION_TRANSFORM))!=0;
        if(historyLiquify!=m_historyLiquify){emit historyScopeChanging(historyLiquify);m_historyLiquify=historyLiquify;}
        if(changedGeneration)m_documentHistoryScroll=0;
        if(m_history!=history){m_history=std::move(history);emit historyChanged();}
        if(m_selection!=selection) {m_selection=std::move(selection);emit selectionChanged();}
        if (changedLayers) {
            m_layers = std::move(layers);
            if(!m_collapsedGroups.isEmpty()) {
                QSet<quint64> groups;for(const auto &value:m_layers) {const auto layer=value.toMap();if(layer.value("group").toBool()) groups.insert(layer.value("id").toULongLong());}
                const auto previous=m_collapsedGroups;m_collapsedGroups.intersect(groups);
                if(previous!=m_collapsedGroups) emit groupExpansionChanged();
            }
        }
        m_modified = (info.flags & PAINT_SESSION_MODIFIED) != 0 || m_pendingMutation != 0;
        if (info.completed_sequence >= m_pendingNew) m_pendingNew = 0;
        m_backendFailed = (info.flags & PAINT_SESSION_FAILED) != 0;
        m_ready = !m_closing && !m_pendingNew && !m_fileOpening && !m_backendFailed;
        if (changedGeneration) {
            for (int i = 0; i < 2; ++i) {
                m_frames[i] = {}; m_regions[i] = {}; m_frameRevisions[i] = 0;
                m_requests[i] = 0; m_requestedRegions[i] = {}; m_requestedPixels[i] = {};
            }
            emit frameChanged();
        }
        // Pixel revisions arrive through frameChanged. Keep QML's layer delegates
        // alive throughout a stroke; notify their model only for metadata changes.
        if (changedLayers) emit layersChanged();
        // Structural changes (delete, undo, paste, merge) must not leave stale selected ids.
        if(activeChanged && !m_selectedLayers.contains(m_active)) {
            quint64 selectedId=m_active;
            for(const auto &value:m_layers){const auto layer=value.toMap();if(layer.value("id").toULongLong()==m_active && layer.value("mask").toBool())selectedId=layer.value("parent").toULongLong();}
            if(!m_selectedLayers.contains(selectedId)){m_selectedLayers={selectedId};emit selectedLayersChanged();}
        }
        if (changedLayers || activeChanged) pruneSelectedLayers();
        if (wasLiquify!=m_liquifyActive || m_transformActive || wasLiquifyBusy!=liquifyBusy() || propertiesChanged || wasReady != m_ready || wasModified != modified() || wasLayerBusy != layerEditBusy()) emit stateChanged();
    });
    connect(m_worker,&BackendWorker::thumbnailReady,this,[this](quint64 generation,quint64 revision,quint64 id,const QString &image) {
        if(m_closing || generation!=m_generation || revision!=m_revision)return;
        for(auto &value:m_layers) {
            auto layer=value.toMap();
            if(layer.value("id").toULongLong()!=id)continue;
            layer.insert("thumbnail",image);value=layer;
            m_layerThumbnails.insert(QString::number(id),image);
            emit layerThumbnailsChanged();return;
        }
    });
    connect(m_worker, &BackendWorker::pixels, this, [this](int view, const QImage &image, QRectF region, quint64 generation, quint64 revision, quint64 request) {
        m_connection->framePending[view].store(false);
        if (m_closing || m_pendingNew || generation != m_generation ||
            (m_requests[view] && request != m_requests[view]) || revision < m_frameRevisions[view]) return;
        m_frames[view] = image; m_regions[view] = region; m_frameRevisions[view] = revision; emit frameChanged();
    });
    connect(m_worker, &BackendWorker::failure, this, &PaintCoreClient::showError);
    connect(m_worker, &BackendWorker::fileResult, this, [this](PaintFileJobInfo info, const QString &message) {
        if (info.job_id != m_fileId || m_closing) return;
        m_fileBusy = false; m_fileOpening = false;
        const bool success = info.state == PAINT_FILE_SUCCEEDED;
        if (success && info.result_generation == m_generation) {
            if (info.kind == PAINT_FILE_OPEN) {
                m_sourceUrl = m_fileUrl;
                m_documentUrl = info.format == PAINT_FILE_OPENRASTER ? m_fileUrl : QUrl{};
            } else if (info.kind==PAINT_FILE_SAVE && info.format == PAINT_FILE_OPENRASTER) { m_documentUrl = m_fileUrl; m_sourceUrl = m_fileUrl; }
        }
        if (!success) showError(message);
        m_ready = !m_closing && !m_pendingNew && !m_backendFailed;
        emit stateChanged(); emit fileFinished(success);
    });
    connect(m_worker, &BackendWorker::completed, this, &PaintCoreClient::commandCompleted);
    connect(m_worker, &BackendWorker::finished, &m_thread, &QThread::quit, Qt::DirectConnection);
    connect(&m_thread, &QThread::finished, m_worker, &QObject::deleteLater);
    connect(&m_thread, &QThread::finished, this, &PaintCoreClient::stopped);
    m_thread.start();
}
PaintCoreClient::~PaintCoreClient() {
    if(m_hoverPose)paint_brush_pose_destroy(&m_hoverPose);if(m_strokePose)paint_brush_pose_destroy(&m_strokePose);
    // Window close uses asynchronous shutdown; this is a lifetime safety net.
    if (m_thread.isRunning()) {
        QMetaObject::invokeMethod(m_worker, &BackendWorker::stop, Qt::BlockingQueuedConnection);
        m_thread.quit(); m_thread.wait();
    }
}
void PaintCoreClient::shutdown() {
    if (m_closing) return;
    m_closing = true; m_ready = false; m_drawing = false;
    if(m_hoverPose)paint_brush_pose_destroy(&m_hoverPose);if(m_strokePose)paint_brush_pose_destroy(&m_strokePose);
    emit stateChanged(); QMetaObject::invokeMethod(m_worker, &BackendWorker::stop, Qt::QueuedConnection);
}
bool PaintCoreClient::submit(int type, const InputSample &sample, quint64 id, const QString &text, qreal amount, bool flag) {
    if (m_closing || !m_ready || m_liquifyActive) return false;
    clearLayerBlendPreview();
    auto command = dto<PaintCommand>(); command.kind = static_cast<uint32_t>(type); command.layer_id = id;
    const auto utf8 = text.toUtf8(); command.text = reinterpret_cast<const uint8_t *>(utf8.constData());
    command.text_length = static_cast<uint64_t>(utf8.size());
    if (type == New || type == NewWhite) { command.width = static_cast<uint32_t>(id); command.height = static_cast<uint32_t>(amount); }
    if (type == Properties) { command.visible = flag ? 1 : 0; command.opacity = static_cast<float>(amount); }
    if (type == Move || type == Begin) command.point = point(sample);
    if (type == Begin) {
        if(!synchronizeBrushCatalog(m_brushLibrary->snapshot()))return false;
        command.stroke = dto<PaintStrokeDesc>(); // Layer 0 resolves active layer in execution order.
        command.stroke.mode = m_eraser || sample.tool == PAINT_TOOL_ERASER ? PAINT_MODE_ERASE : PAINT_MODE_PAINT;
        command.stroke.radius = static_cast<float>(m_radius); command.stroke.opacity = static_cast<float>(m_opacity);
        command.stroke.spacing = static_cast<float>(m_spacing);
        command.stroke.linear_rgba[0] = linear(m_color.redF()); command.stroke.linear_rgba[1] = linear(m_color.greenF());
        command.stroke.linear_rgba[2] = linear(m_color.blueF()); command.stroke.linear_rgba[3] = m_color.alphaF();
    }
    uint64_t sequence = 0; PaintStatus status; QString error;
    {
        QMutexLocker lock(&m_connection->mutex);
        if (!m_connection->session) return false;
        if(type==Begin) {
            auto request=dto<PaintBrushToolDesc>();request.radius=command.stroke.radius;std::copy(std::begin(command.stroke.linear_rgba),std::end(command.stroke.linear_rgba),std::begin(request.linear_rgba));request.strength=1;
            request.operation=command.stroke.mode==PAINT_MODE_ERASE?PAINT_BRUSH_ERASE:PAINT_BRUSH_PAINT;
            if(m_smudgeTool){request.operation=PAINT_BRUSH_SMUDGE;
                request.strength=static_cast<float>(m_smudgeStrength);request.all_layers=m_smudgeAllLayers;request.finger_painting=m_smudgeFingerPainting;request.pressure_strength=m_smudgePressure;}
            const auto presetId=m_brushLibrary->selectedId().toUtf8();QMutexLocker catalogLock(&m_brushCatalogConnection->mutex);
            auto background=dto<PaintBrushBackgroundDesc>();background.linear_rgba[0]=linear(m_secondaryColor.redF());background.linear_rgba[1]=linear(m_secondaryColor.greenF());background.linear_rgba[2]=linear(m_secondaryColor.blueF());background.linear_rgba[3]=m_secondaryColor.alphaF();
            status=paint_session_begin_prepared_preset_with_background(m_connection->core,m_connection->session,m_brushCatalogConnection->library,m_brushCatalogConnection->preparation,reinterpret_cast<const uint8_t*>(presetId.constData()),quint64(presetId.size()),&request,&background,&command.point,&sequence);
            if(status==PAINT_OK){
                m_strokeBasePose={false,m_radius,m_brushLibrary->angle(),m_brushLibrary->roundness()};
                m_strokeOutlineResource=m_brushLibrary->maskResource();m_strokeOutlinePath=m_brushOutlinePath;m_strokeOutlineReady=m_brushOutlineReady;
                if(!m_strokePose)paint_brush_pose_create(&m_strokePose);
                m_strokePoseBound=m_strokePose && paint_brush_pose_bind_prepared(m_strokePose,m_brushCatalogConnection->library,m_brushCatalogConnection->preparation,reinterpret_cast<const uint8_t*>(presetId.constData()),quint64(presetId.size()),request.radius)==PAINT_OK;
            }
        } else status = paint_session_submit(m_connection->core, m_connection->session, &command, &sequence);
        if (status != PAINT_OK) error = immediateError(status);
    }
    if (status != PAINT_OK) {
        if ((type == Move || type == End) && (status == PAINT_LIMIT_EXCEEDED || status == PAINT_CANCELLED)) m_dropStroke = true;
        showError(error); return false;
    }
    if (type == New || type == NewWhite) { m_pendingNew = sequence; m_ready = false; emit stateChanged(); }
    if(type==Select) { m_pendingLayer=sequence; emit stateChanged(); }
    if (type == End || type == Undo || type == Redo || type == Add || type == Remove || type == Properties || type == AddMask) {
        // Accepted edits must protect against an immediate New/Open/Close before
        // the next metadata poll. Failed/no-op commands are reconciled by the actor.
        m_pendingMutation = sequence;
        if(type!=End) m_pendingLayer=sequence;
        m_modified = true; emit stateChanged();
    }
    return true;
}
BrushCursorPose PaintCoreClient::baseCursorPose() const {
    if(m_drawing && !m_liquifyActive)return m_strokeBasePose;
    return m_liveBasePose;
}
BrushCursorPose PaintCoreClient::brushCursorPose(const InputSample &sample,QPointF direction,bool directionKnown,bool pressureKnown) {
    auto result=baseCursorPose();if(!m_ready || m_closing || m_liquifyActive)return result;
    PaintBrushPose *handle=nullptr;
    if(m_drawing){if(!m_strokePoseBound)return result;handle=m_strokePose;}
    else {
        QMutexLocker lock(&m_brushCatalogConnection->mutex);if(!m_brushCatalogConnection->library || !m_brushCatalogConnection->preparation)return result;
        const float radius=static_cast<float>(m_radius);
        if(!m_hoverPose && paint_brush_pose_create(&m_hoverPose)!=PAINT_OK)return result;
        if(m_hoverPosePreparation!=m_brushCatalogConnection->preparation || m_hoverPoseId!=m_brushLibrary->selectedId() || m_hoverPoseRadius!=radius){
            const auto id=m_brushLibrary->selectedId().toUtf8();
            if(m_poseExpectedSource.isEmpty())m_poseExpectedSource=m_brushLibrary->catalogSnapshot();
            if(m_brushCatalogConnection->source!=m_poseExpectedSource)return result;
            if(paint_brush_pose_bind_prepared(m_hoverPose,m_brushCatalogConnection->library,m_brushCatalogConnection->preparation,reinterpret_cast<const uint8_t*>(id.constData()),quint64(id.size()),radius)!=PAINT_OK)return result;
            m_hoverPosePreparation=m_brushCatalogConnection->preparation;m_hoverPoseId=m_brushLibrary->selectedId();m_hoverPoseRadius=radius;
        }
        handle=m_hoverPose;
    }
    const auto input=point(sample);auto pose=dto<PaintTipPose>();
    if(paint_brush_pose_read(handle,&input,pressureKnown,directionKnown,directionKnown?direction.x():0,directionKnown?direction.y():0,&pose)==PAINT_OK)result={true,pose.radius,pose.angle,pose.roundness};
    return result;
}
void PaintCoreClient::requestViewport(int view, QRectF region, QSize pixels, bool enabled) {
    if (!m_ready || m_closing || view < 0 || view > 1) return;
    if (enabled) {
        if (!std::isfinite(region.x()) || !std::isfinite(region.y()) || !std::isfinite(region.width()) ||
            !std::isfinite(region.height()) || region.isEmpty() || pixels.isEmpty()) return;
        const double ratio = std::min({1., 4096. / pixels.width(), 4096. / pixels.height(),
            std::sqrt(4194304. / (static_cast<double>(pixels.width()) * pixels.height()))});
        pixels = {std::max(1, static_cast<int>(pixels.width() * ratio)), std::max(1, static_cast<int>(pixels.height() * ratio))};
    }
    if (m_requests[view] && m_viewEnabled[view] == enabled && m_requestedRegions[view] == region && m_requestedPixels[view] == pixels) return;
    auto viewport = dto<PaintViewport>(); viewport.view_id = static_cast<uint32_t>(view); viewport.enabled = enabled ? 1 : 0;
    viewport.document_generation = m_generation; viewport.x = region.x(); viewport.y = region.y();
    viewport.width = region.width(); viewport.height = region.height();
    viewport.pixel_width = static_cast<uint32_t>(pixels.width()); viewport.pixel_height = static_cast<uint32_t>(pixels.height());
    uint64_t request = 0; PaintStatus status; QString error;
    {
        QMutexLocker lock(&m_connection->mutex);
        if (!m_connection->session) return;
        status = m_blendPreviewLayer && enabled
            ? paint_session_set_blend_preview(m_connection->core,m_connection->session,m_blendPreviewLayer,static_cast<uint32_t>(m_blendPreviewMode),&viewport,&request)
            : paint_session_set_viewport(m_connection->core, m_connection->session, &viewport, &request);
        if (status != PAINT_OK && status != PAINT_BUSY) error = immediateError(status);
    }
    if (status == PAINT_OK) {
        m_requests[view] = request; m_requestedRegions[view] = region; m_requestedPixels[view] = pixels; m_viewEnabled[view] = enabled;
        if (!enabled) { m_frames[view] = {}; m_regions[view] = {}; emit frameChanged(); }
    } else if (!error.isEmpty()) showError(error);
}
void PaintCoreClient::showError(const QString &error) { m_error = error; emit errorChanged(); }
void PaintCoreClient::clearError() { m_error.clear(); emit errorChanged(); }
QVariantList PaintCoreClient::recentColors() const {return m_brushLibrary->recentColors();}
void PaintCoreClient::setHistoryLimit(int commands) {
    const int next=std::clamp(commands,10,1000);
    if(next==m_historyLimit)return;
    m_historyLimit=next;
    emit historyLimitChanged();
    if(m_worker)QMetaObject::invokeMethod(m_worker,[worker=m_worker,next]{worker->storeHistoryLimit(next);},Qt::QueuedConnection);
}
void PaintCoreClient::applyHistoryLimit() {
    // Persist the preference and make it take effect on the open document as well.
    if(m_worker)QMetaObject::invokeMethod(m_worker,[worker=m_worker]{worker->applyHistoryLimit();},Qt::QueuedConnection);
}
void PaintCoreClient::setBrushColor(const QColor &c) {
    // Picking a colour is only a preview: the swatch row advances when the pixel stroke commits.
    if (c.isValid() && c != m_color) { m_color = c; emit brushChanged(); }
}
void PaintCoreClient::setSecondaryBrushColor(const QColor &c){if(c.isValid() && c!=m_secondaryColor){m_secondaryColor=c;emit brushChanged();}}
void PaintCoreClient::swapBrushColors(){
    // Swapping selects a colour; the recent swatch row only advances on a committed stroke.
    std::swap(m_color,m_secondaryColor);
    emit brushChanged();
}
void PaintCoreClient::setBrushRadius(qreal r) {m_brushLibrary->setRadius(r);}
void PaintCoreClient::setBrushOpacity(qreal o) {m_brushLibrary->setOpacity(o);}
void PaintCoreClient::setBrushSpacing(qreal s) {m_brushLibrary->setSpacing(s);}
// Active tools are exclusive; the remembered marquee shape is separate from these flags.
void PaintCoreClient::setEraser(bool e) {
    if(deferTransformAction([this ,e]{setEraser(e);}))return;
    if(m_drawing || m_liquifyActive || m_transformActive || (e==m_eraser && !m_moveTool && !m_selectionTool && !m_eyedropperTool && !m_bucketTool && !m_smudgeTool)) return;
    m_moveTool=false;m_selectionTool=0;m_eyedropperTool=false;m_bucketTool=false;m_smudgeTool=false;
    m_eraser=e; m_brushLibrary->setEraser(e); emit brushChanged();
}
void PaintCoreClient::setMoveTool(bool enabled) {
    if(deferTransformAction([this,enabled]{setMoveTool(enabled);}))return;
    if(m_drawing || m_liquifyActive || m_transformActive || enabled==m_moveTool) return;
    if(enabled){m_selectionTool=0;m_eyedropperTool=false;m_bucketTool=false;m_smudgeTool=false;m_eraser=false;m_brushLibrary->setEraser(false);}
    m_moveTool=enabled; emit brushChanged();
}
void PaintCoreClient::setSelectionTool(int tool) {
    if(deferTransformAction([this,tool]{setSelectionTool(tool);}))return;
    if(tool<0 || tool>5 || m_drawing || m_liquifyActive || m_transformActive || tool==m_selectionTool) return;
    m_selectionTool=tool;
    if(tool>0) {
        if(tool<=2)m_marqueeShape=tool;else if(tool<=4)m_lassoShape=tool;
        m_moveTool=false;m_eyedropperTool=false;m_bucketTool=false;m_smudgeTool=false;m_eraser=false;m_brushLibrary->setEraser(false);
    }
    emit brushChanged();
}
void PaintCoreClient::activateBrush() {if(deferTransformAction([this]{activateBrush();}))return;
    if(m_liquifyActive)return;
    if(m_drawing || m_liquifyActive)return;
    if(m_eyedropperTool){m_eyedropperTool=false;emit brushChanged();}
    // Tool changes wait for the current stroke; the remembered marquee shape survives.
    setEraser(false);
    setMoveTool(false);
    if(m_selectionTool!=0) {m_selectionTool=0; emit brushChanged();}
}
void PaintCoreClient::activateEraser() {if(deferTransformAction([this]{activateEraser();}))return;
    if(m_drawing || m_liquifyActive)return;
    if(m_eyedropperTool){m_eyedropperTool=false;emit brushChanged();}
    setMoveTool(false);
    if(m_selectionTool!=0) {m_selectionTool=0; emit brushChanged();}
    setEraser(true);
}

void PaintCoreClient::activateMoveTool() {if(deferTransformAction([this]{activateMoveTool();}))return;
    if(m_drawing || m_liquifyActive)return;
    if(m_eyedropperTool){m_eyedropperTool=false;emit brushChanged();}
    if(m_selectionTool!=0) {m_selectionTool=0; emit brushChanged();}
    setEraser(false);
    setMoveTool(true);
}
void PaintCoreClient::activateMarquee() {if(deferTransformAction([this]{activateMarquee();}))return;
    if(m_drawing || m_liquifyActive)return;
    // Activating the slot restores its remembered shape.
    const int shape = (m_selectionTool==1 || m_selectionTool==2) ? m_selectionTool : m_marqueeShape;
    setMoveTool(false); setEraser(false);
    setSelectionTool(shape);
}
void PaintCoreClient::activateLasso() {setSelectionTool(m_lassoShape);}
void PaintCoreClient::setWandSampleSize(int value) {if(!QList<int>{1,3,5,11,31,51,101}.contains(value) || value==m_wandSampleSize)return;m_wandSampleSize=value;emit wandOptionsChanged();}
void PaintCoreClient::setWandTolerance(int value) {if(value<0 || value>255 || value==m_wandTolerance)return;m_wandTolerance=value;emit wandOptionsChanged();}
void PaintCoreClient::setWandOperation(int value) {if(value<0 || value>3 || value==m_wandOperation)return;m_wandOperation=value;emit wandOptionsChanged();}
void PaintCoreClient::setWandAntialias(bool value) {if(value==m_wandAntialias)return;m_wandAntialias=value;emit wandOptionsChanged();}
void PaintCoreClient::setWandContiguous(bool value) {if(value==m_wandContiguous)return;m_wandContiguous=value;emit wandOptionsChanged();}
void PaintCoreClient::setWandAllLayers(bool value) {if(value==m_wandAllLayers)return;m_wandAllLayers=value;emit wandOptionsChanged();}
bool PaintCoreClient::selectWand(QPointF position,int operation) {
    if(m_selectionTool!=5 || !m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy || m_closing || operation<0 || operation>3)return false;
    if(!std::isfinite(position.x()) || !std::isfinite(position.y()) || position.x()<0 || position.y()<0 || position.x()>=m_width || position.y()>=m_height)return false;
    auto request=dto<PaintWandSelection>();request.x=static_cast<uint32_t>(position.x());request.y=static_cast<uint32_t>(position.y());request.sample_size=m_wandSampleSize;request.tolerance=m_wandTolerance;
    request.antialias=m_wandAntialias;request.contiguous=m_wandContiguous;request.all_layers=m_wandAllLayers;request.operation=operation;
    uint64_t sequence=0;QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;
    const auto status=paint_session_select_wand(m_connection->core,m_connection->session,&request,&sequence);lock.unlock();
    if(status!=PAINT_OK){showError(immediateError(status));return false;}m_pendingLayer=m_pendingMutation=sequence;emit stateChanged();return true;
}
void PaintCoreClient::setSelectionOperation(int operation) {if(operation<0 || operation>3 || operation==m_selectionOperation)return;m_selectionOperation=operation;emit selectionOptionsChanged();}
void PaintCoreClient::setSelectionFeather(qreal feather) {if(!std::isfinite(feather) || feather<0 || feather>1000 || feather==m_selectionFeather)return;m_selectionFeather=feather;emit selectionOptionsChanged();}
void PaintCoreClient::setSelectionAntialias(bool enabled) {if(enabled==m_selectionAntialias)return;m_selectionAntialias=enabled;emit selectionOptionsChanged();}
void PaintCoreClient::activateEyedropper() {if(deferTransformAction([this]{activateEyedropper();}))return;
    if(m_drawing || m_liquifyActive)return;
    activateBrush();m_eyedropperTool=true;emit brushChanged();
}

void PaintCoreClient::setSamplingRingEnabled(bool enabled) {if(enabled==m_samplingRingEnabled)return;m_samplingRingEnabled=enabled;emit samplingRingEnabledChanged();}
void PaintCoreClient::activateBucket() {if(deferTransformAction([this]{activateBucket();}))return;if(m_drawing || m_liquifyActive)return;activateBrush();m_bucketTool=true;emit brushChanged();}
void PaintCoreClient::activateSmudge() {if(deferTransformAction([this]{activateSmudge();}))return;if(m_drawing || m_liquifyActive)return;activateBrush();m_smudgeTool=true;emit brushChanged();}
void PaintCoreClient::setSmudgeStrength(qreal value) {if(!std::isfinite(value) || value<0 || value>1 || value==m_smudgeStrength)return;m_smudgeStrength=value;emit smudgeOptionsChanged();}
void PaintCoreClient::setSmudgePressure(bool value) {if(value==m_smudgePressure)return;m_smudgePressure=value;emit smudgeOptionsChanged();}
void PaintCoreClient::setSmudgeAllLayers(bool value) {if(value==m_smudgeAllLayers)return;m_smudgeAllLayers=value;emit smudgeOptionsChanged();}
void PaintCoreClient::setSmudgeFingerPainting(bool value) {if(value==m_smudgeFingerPainting)return;m_smudgeFingerPainting=value;emit smudgeOptionsChanged();}
void PaintCoreClient::setBucketOpacity(qreal value) {if(!std::isfinite(value) || value<0 || value>1 || value==m_bucketOpacity)return;m_bucketOpacity=value;emit bucketOptionsChanged();}
void PaintCoreClient::setBucketTolerance(int value) {if(value<0 || value>255 || value==m_bucketTolerance)return;m_bucketTolerance=value;emit bucketOptionsChanged();}
void PaintCoreClient::setBucketAntialias(bool value) {if(value==m_bucketAntialias)return;m_bucketAntialias=value;emit bucketOptionsChanged();}
void PaintCoreClient::setBucketContiguous(bool value) {if(value==m_bucketContiguous)return;m_bucketContiguous=value;emit bucketOptionsChanged();}
void PaintCoreClient::setBucketAllLayers(bool value) {if(value==m_bucketAllLayers)return;m_bucketAllLayers=value;emit bucketOptionsChanged();}
bool PaintCoreClient::bucketFill(QPointF position) {
    if(!m_bucketTool || !m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy || m_closing)return false;
    if(!std::isfinite(position.x()) || !std::isfinite(position.y()) || position.x()<0 || position.y()<0 || position.x()>=m_width || position.y()>=m_height)return false;
    auto request=dto<PaintBucketFill>();request.x=static_cast<uint32_t>(position.x());request.y=static_cast<uint32_t>(position.y());
    request.linear_rgba[0]=linear(m_color.redF());request.linear_rgba[1]=linear(m_color.greenF());request.linear_rgba[2]=linear(m_color.blueF());request.linear_rgba[3]=m_color.alphaF();
    request.opacity=static_cast<float>(m_bucketOpacity);request.tolerance=static_cast<uint32_t>(m_bucketTolerance);
    request.antialias=m_bucketAntialias;request.contiguous=m_bucketContiguous;request.all_layers=m_bucketAllLayers;
    uint64_t sequence=0;QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;
    const auto status=paint_session_bucket_fill(m_connection->core,m_connection->session,&request,&sequence);lock.unlock();
    if(status!=PAINT_OK){showError(immediateError(status));return false;}
    m_pendingLayer=m_pendingMutation=sequence;emit stateChanged();return true;
}




bool PaintCoreClient::submitSelection(quint32 action,QRectF rect,int shape,int operation) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy || m_closing) return false;
    auto request=dto<PaintSelectionEdit>();request.action=action;
    if(action==PAINT_SELECTION_SHAPE) {request.shape=static_cast<uint32_t>(shape);request.operation=static_cast<uint32_t>(operation);request.antialias=1;request.x=rect.x();request.y=rect.y();request.width=rect.width();request.height=rect.height();}
    uint64_t sequence=0;
    QMutexLocker lock(&m_connection->mutex);
    if(!m_connection->session) return false;
    const auto status=paint_session_edit_selection(m_connection->core,m_connection->session,&request,&sequence);
    lock.unlock();
    if(status!=PAINT_OK){showError(immediateError(status));return false;}
    m_pendingLayer=m_pendingMutation=sequence;m_modified=true;emit stateChanged();return true;
}
bool PaintCoreClient::submitSelectionShape(const PaintSelectionShapeEdit &request) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy || m_closing)return false;
    uint64_t sequence=0;
    QMutexLocker lock(&m_connection->mutex);
    if(!m_connection->session)return false;
    const auto status=paint_session_edit_selection_shape(m_connection->core,m_connection->session,&request,&sequence);
    lock.unlock();
    if(status!=PAINT_OK){showError(immediateError(status));return false;}
    m_pendingLayer=m_pendingMutation=sequence;m_modified=true;emit stateChanged();return true;
}
bool PaintCoreClient::editSelection(QRectF rect,int shape,int operation) {
    auto request=dto<PaintSelectionShapeEdit>();
    request.shape=static_cast<uint32_t>(shape);request.operation=static_cast<uint32_t>(operation);
    request.antialias=m_selectionAntialias;request.feather=m_selectionFeather;
    request.x=rect.x();request.y=rect.y();request.width=rect.width();request.height=rect.height();
    return submitSelectionShape(request);
}
bool PaintCoreClient::editSelectionPath(const QVector<QPointF> &points,int operation,bool polygonal) {
    QVector<PaintSelectionPoint> path;
    path.reserve(points.size());for(const auto &p:points)path.append({p.x(),p.y()});
    auto request=dto<PaintSelectionShapeEdit>();
    request.shape=PAINT_SELECTION_POLYGON;request.tool=polygonal?2u:1u;request.operation=static_cast<uint32_t>(operation);
    request.antialias=m_selectionAntialias;request.feather=m_selectionFeather;
    request.point_count=static_cast<uint32_t>(path.size());request.points=path.constData();
    return submitSelectionShape(request);
}

bool PaintCoreClient::selectAll(){return submitSelection(PAINT_SELECTION_ALL);}
bool PaintCoreClient::clearSelection(){return submitSelection(PAINT_SELECTION_CLEAR);}
bool PaintCoreClient::invertSelection(){return submitSelection(PAINT_SELECTION_INVERT);}
void PaintCoreClient::persistLiquifyOptions() {
    emit liquifyOptionsChanged();
    const QVariantMap values{{"tool",m_liquifyTool},{"size",m_liquifySize},{"strength",m_liquifyStrength},{"density",m_liquifyDensity},{"speed",m_liquifySpeed},{"pressure",m_liquifyPressure}};
    if(m_worker && !m_closing)QMetaObject::invokeMethod(m_worker,[worker=m_worker,values]{worker->saveLiquifyOptions(values);},Qt::QueuedConnection);
}
void PaintCoreClient::setLiquifyTool(int v){if(v<0 || v>3 || m_drawing || v==m_liquifyTool)return;m_liquifyTool=v;persistLiquifyOptions();}
void PaintCoreClient::setLiquifySize(int v){if(v<1 || v>2000 || m_drawing || v==m_liquifySize)return;m_liquifySize=v;persistLiquifyOptions();}
void PaintCoreClient::setLiquifyStrength(int v){if(v<1 || v>100 || m_drawing || v==m_liquifyStrength)return;m_liquifyStrength=v;persistLiquifyOptions();}
void PaintCoreClient::setLiquifyDensity(int v){if(v<1 || v>100 || m_drawing || v==m_liquifyDensity)return;m_liquifyDensity=v;persistLiquifyOptions();}
void PaintCoreClient::setLiquifySpeed(int v){if(v<1 || v>100 || m_drawing || v==m_liquifySpeed)return;m_liquifySpeed=v;persistLiquifyOptions();}
void PaintCoreClient::setLiquifyPressure(bool v){if(m_drawing || v==m_liquifyPressure)return;m_liquifyPressure=v;persistLiquifyOptions();}
bool PaintCoreClient::submitLiquifyControl(quint32 kind) {
    if(!m_ready || m_closing || m_fileBusy || m_drawing || m_pendingLiquify || (layerEditBusy() && kind!=PAINT_LIQUIFY_CANCEL))return false;
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;status=paint_session_liquify(m_connection->core,m_connection->session,kind,nullptr,&sequence);if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}
    m_pendingLiquify=sequence;
    if(kind==PAINT_LIQUIFY_BEGIN){m_liquifyActive=true;m_liquifyFocus=false;emit liquifyOptionsChanged();}
    emit stateChanged();return true;
}
bool PaintCoreClient::beginLiquify() {
    if(m_liquifyActive)return false;
    for(const auto &value:m_layers){const auto l=value.toMap();if(l.value("id").toULongLong()==m_active && (l.value("group").toBool() || l.value("effectiveLocks").toUInt())) {showError(QStringLiteral("液化需要未锁定的像素图层或蒙版"));return false;}}
    clearLayerBlendPreview();clearError();return submitLiquifyControl(PAINT_LIQUIFY_BEGIN);
}
void PaintCoreClient::setLiquifyFocus(bool value){if(value==m_liquifyFocus || !m_liquifyActive)return;if(submitLiquifyControl(value?PAINT_LIQUIFY_FOCUS_ON:PAINT_LIQUIFY_FOCUS_OFF)){m_liquifyFocus=value;emit liquifyOptionsChanged();}}
bool PaintCoreClient::resetLiquify(){return m_liquifyActive && submitLiquifyControl(PAINT_LIQUIFY_RESET);}
bool PaintCoreClient::finishLiquify(bool commit){return m_liquifyActive && submitLiquifyControl(commit?PAINT_LIQUIFY_COMMIT:PAINT_LIQUIFY_CANCEL);}
bool PaintCoreClient::submitLiquifyStrokeMarker(quint32 kind) {
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;status=paint_session_liquify(m_connection->core,m_connection->session,kind,nullptr,&sequence);if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}m_lastLiquifyDab=sequence;return true;
}
bool PaintCoreClient::submitLiquifyDab(const InputSample &from,const InputSample &to,int elapsedMs) {
    auto request=dto<PaintLiquifyDab>();request.tool=m_liquifyTool;request.radius=m_liquifySize*.5f;request.strength=m_liquifyStrength*.01f;request.density=m_liquifyDensity*.01f;request.speed=m_liquifySpeed*.01f;request.pressure_enabled=m_liquifyPressure;request.from=point(from);request.to=point(to);request.elapsed_ms=float(elapsedMs);
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;status=paint_session_liquify(m_connection->core,m_connection->session,PAINT_LIQUIFY_DAB,&request,&sequence);if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}m_lastLiquifyDab=sequence;return true;
}
bool PaintCoreClient::beginStroke(const InputSample &s) {
    if (!m_ready || m_drawing || layerEditBusy() || m_pendingLiquify) return false;
    if(m_liquifyActive) {m_dropStroke=false;m_liquifyLast=s;m_liquifyClock.restart();if(!submitLiquifyStrokeMarker(PAINT_LIQUIFY_STROKE_BEGIN))return false;if(!submitLiquifyDab(s,s,16)){submitLiquifyStrokeMarker(PAINT_LIQUIFY_STROKE_END);return false;}m_drawing=true;emit stateChanged();return true;}
    for(const auto &value:m_layers) { const auto layer=value.toMap(); if(layer.value("id").toULongLong()!=m_active) continue;
        if(layer.value("group").toBool()) {showError(QStringLiteral("请选中组内像素图层绘画，或先新建图层"));return false;}
        if(layer.value("effectiveLocks").toUInt() & PAINT_LOCK_ALL) {showError(QStringLiteral("当前图层或所属组已完全锁定，请先解锁"));return false;}
    }
    m_dropStroke = false; if (!submit(Begin, s)) return false;
    m_strokeColor = m_smudgeTool && !m_smudgeFingerPainting ? QColor() : m_color;
    m_drawing = true; emit stateChanged(); return true;
}
void PaintCoreClient::strokeTo(const InputSample &s) {
    if(!m_drawing || m_dropStroke)return;
    if(m_liquifyActive){if(submitLiquifyDab(m_liquifyLast,s,std::clamp(int(m_liquifyClock.restart()),0,50)))m_liquifyLast=s;else m_dropStroke=true;}
    else submit(Move,s);
}
void PaintCoreClient::liquifyTick() {if(m_drawing && m_liquifyActive && !m_dropStroke && (m_liquifyTool==1 || m_liquifyTool==2)) {if(!submitLiquifyDab(m_liquifyLast,m_liquifyLast,std::clamp(int(m_liquifyClock.restart()),0,50)))m_dropStroke=true;m_liquifyClock.restart();}}
void PaintCoreClient::endStroke() {
    if (!m_drawing) return;
    if(m_liquifyActive){submitLiquifyStrokeMarker(PAINT_LIQUIFY_STROKE_END);m_drawing=false;m_pendingLayer=m_lastLiquifyDab>m_completedSequence?m_lastLiquifyDab:0;emit stateChanged();return;}
    if (!m_dropStroke) submit(End);
    // A committed stroke is what makes the colour "recently used"; a cancelled one never shows up.
    if (!m_dropStroke && m_strokeColor.isValid()) m_brushLibrary->useColor(m_strokeColor.name(QColor::HexArgb));
    m_drawing = false;if(m_strokePose)paint_brush_pose_clear(m_strokePose);m_strokePoseBound=false;m_strokeOutlineResource.clear();m_strokeOutlinePath.clear();emit brushOutlineChanged();emit stateChanged();
}
void PaintCoreClient::cancelStroke() {
    if (!m_drawing) return;
    if(m_liquifyActive){submitLiquifyStrokeMarker(PAINT_LIQUIFY_STROKE_END);m_drawing=false;m_pendingLayer=m_lastLiquifyDab>m_completedSequence?m_lastLiquifyDab:0;emit stateChanged();return;}
    if (!m_dropStroke) submit(Cancel);
    m_dropStroke = true;m_drawing = false;if(m_strokePose)paint_brush_pose_clear(m_strokePose);m_strokePoseBound=false;m_strokeOutlineResource.clear();m_strokeOutlinePath.clear();emit brushOutlineChanged();emit stateChanged();
}
void PaintCoreClient::undo() { if (m_ready && !m_drawing) {if(m_transformActive)submitTransform(PAINT_TRANSFORM_UNDO);else if(m_liquifyActive)submitLiquifyControl(PAINT_LIQUIFY_UNDO);else submit(Undo);} }
void PaintCoreClient::redo() { if (m_ready && !m_drawing) {if(m_transformActive)submitTransform(PAINT_TRANSFORM_REDO);else if(m_liquifyActive)submitLiquifyControl(PAINT_LIQUIFY_REDO);else submit(Redo);} }

QColor PaintCoreClient::sampleDocumentPixel(qreal x, qreal y) const {
    if(!m_connection || m_closing || !std::isfinite(x) || !std::isfinite(y)) return {};
    float rgba[4]={0,0,0,0};
    QMutexLocker lock(&m_connection->mutex);
    if(!m_connection->session) return {};
    // View 0 is the main canvas frame the user is looking at.
    if(paint_session_sample_pixel(m_connection->core,m_connection->session,0,
                                  qint64(std::floor(x)),qint64(std::floor(y)),rgba)!=PAINT_OK) return {};
    if(!(rgba[3]>0.f)) return QColor(0,0,0,0);
    return QColor::fromRgbF(std::clamp(qreal(rgba[0]),0.,1.),std::clamp(qreal(rgba[1]),0.,1.),
                            std::clamp(qreal(rgba[2]),0.,1.),std::clamp(qreal(rgba[3]),0.,1.));
}
void PaintCoreClient::addLayer(const QString &name) {
    if (m_ready && !m_drawing && !layerEditBusy() && submit(Add, {}, 0, name) && m_collapsedGroups.remove(m_active)) emit groupExpansionChanged();
}
void PaintCoreClient::removeLayer(quint64 id) { if (m_ready && !m_drawing) submit(Remove, {}, id); }
void PaintCoreClient::requestSelectedLayerRemoval() {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy)return;
    m_deleteLayers=m_selectedLayers;
    // An active mask is an explicit single-node edit, not a deletion of its owner's selection.
    bool groups=false;
    for(const auto &value:m_layers) {
        const auto layer=value.toMap();const auto id=layer.value("id").toULongLong();
        if(id==m_active && layer.value("mask").toBool()){m_deleteLayers={id};break;}
    }
    for(const auto &value:m_layers) {const auto layer=value.toMap();if(m_deleteLayers.contains(layer.value("id").toULongLong()) && layer.value("group").toBool())groups=true;}
    m_deleteGeneration=m_generation;m_deleteRevision=m_revision;
    if(groups)emit groupDeletionRequested();else confirmSelectedLayerRemoval(2);
}
bool PaintCoreClient::confirmSelectedLayerRemoval(int choice) {
    const auto ids=m_deleteLayers;m_deleteLayers.clear();
    if(choice==0)return true;
    if(choice<1 || choice>2 || ids.isEmpty() || m_deleteGeneration!=m_generation || m_deleteRevision!=m_revision)return false;
    return removeLayers(ids,choice==1);
}
bool PaintCoreClient::removeLayers(const QList<quint64> &ids,bool keepChildren) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy || ids.isEmpty())return false;
    const std::vector<uint64_t> input(ids.begin(),ids.end());
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;
        status=paint_session_remove_layers(m_connection->core,m_connection->session,input.data(),static_cast<uint32_t>(input.size()),keepChildren?PAINT_REMOVE_KEEP_GROUP_CHILDREN:PAINT_REMOVE_SUBTREES,&sequence);
        if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}
    m_pendingLayer=m_pendingMutation=sequence;m_modified=true;emit stateChanged();return true;
}
void PaintCoreClient::selectLayer(quint64 id) {
    if(deferTransformAction([this,id]{selectLayer(id);}))return; if (m_ready && !m_drawing) submit(Select, {}, id); }
QVariantList PaintCoreClient::selectedLayers() const {
    QVariantList rows;for(const auto id:m_selectedLayers)rows.append(id);return rows;
}
void PaintCoreClient::selectLayers(quint64 id, int modifiers) {
    if(deferTransformAction([this,id,modifiers]{selectLayers(id,modifiers);}))return;
    const bool additive=(modifiers&int(Qt::ControlModifier))!=0 || (modifiers&int(Qt::MetaModifier))!=0;
    const bool range=(modifiers&int(Qt::ShiftModifier))!=0;
    // Only real, non-mask layers take part; masks follow their owner.
    QList<quint64> order;
    for(const auto &value:m_layers) {const auto layer=value.toMap();if(!layer.value("mask").toBool())order.append(layer.value("id").toULongLong());}
    if(!order.contains(id))return;
    if(range && m_active!=0 && order.contains(m_active)) {
        const int from=order.indexOf(m_active),to=order.indexOf(id);
        QList<quint64> span;
        for(int i=qMin(from,to);i<=qMax(from,to);++i)span.append(order[i]);
        m_selectedLayers=span;
    } else if(additive) {
        if(m_selectedLayers.contains(id)) {
            if(m_selectedLayers.size()==1)return; // Never leave the panel with no selection.
            m_selectedLayers.removeAll(id);
        } else m_selectedLayers.append(id);
    } else if(m_selectedLayers!=QList<quint64>{id}) {
        m_selectedLayers={id};
    } else {
        if(m_active!=id)selectLayer(id);
        return;
    }
    emit selectedLayersChanged();emit stateChanged();
    selectLayer(m_selectedLayers.contains(id)?id:m_selectedLayers.last());
}
void PaintCoreClient::pruneSelectedLayers() {
    QList<quint64> live;
    for(const auto &value:m_layers) {const auto layer=value.toMap();if(!layer.value("mask").toBool())live.append(layer.value("id").toULongLong());}
    QList<quint64> kept;for(const auto id:m_selectedLayers)if(live.contains(id))kept.append(id);
    if(kept.isEmpty() && m_active!=0)kept.append(m_active);
    if(kept!=m_selectedLayers) {m_selectedLayers=kept;emit selectedLayersChanged();}
}
void PaintCoreClient::setLayerProperties(quint64 id, bool v, qreal opacity) {
    if (m_ready && !m_drawing && !layerEditBusy() && std::isfinite(opacity)) submit(Properties, {}, id, {}, std::clamp(opacity, 0., 1.), v);
}
bool PaintCoreClient::setAppearance(quint64 id,const QString &field,const QVariant &value) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy()) return false;
    clearLayerBlendPreview();
    QVariantMap layer;
    for(const auto &entry:m_layers) if(entry.toMap().value("id").toULongLong()==id) { layer=entry.toMap(); break; }
    if(layer.isEmpty()) return false;
    layer[field]=value;
    auto options=dto<PaintLayerAppearance>(); options.locks=layer.value("locks").toUInt();
    options.blend_mode=layer.value("blendMode").toUInt(); options.fill=layer.value("fill").toFloat();
    options.offset_x=layer.value("offsetX").toInt(); options.offset_y=layer.value("offsetY").toInt(); options.dissolve_seed=layer.value("dissolveSeed").toUInt();
    uint64_t sequence=0; PaintStatus status; QString error;
    { QMutexLocker lock(&m_connection->mutex); if(!m_connection->session) return false;
      status=paint_session_set_layer_appearance(m_connection->core,m_connection->session,id,&options,&sequence);
      if(status!=PAINT_OK) error=immediateError(status);
    }
    if(status!=PAINT_OK) { showError(error); return false; }
    m_pendingLayer=m_pendingMutation=sequence; m_modified=true; emit stateChanged(); return true;
}
bool PaintCoreClient::setLayerFill(quint64 id,qreal fill) { return std::isfinite(fill) && fill>=0 && fill<=1 && setAppearance(id,"fill",fill); }
bool PaintCoreClient::setLayerBlend(quint64 id,int blend) { if(blend<0 || blend>27)return false;
    if(blend==27 && !std::any_of(m_layers.begin(),m_layers.end(),[id](const QVariant &v){const auto l=v.toMap();return l.value("id").toULongLong()==id && l.value("group").toBool();}))return false;
    return setAppearance(id,"blendMode",blend); }
void PaintCoreClient::previewLayerBlend(quint64 id,int blend) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy || blend<0 || blend>27)return;
    bool found=false;
    for(const auto &value:m_layers) {const auto layer=value.toMap();if(layer.value("id").toULongLong()==id && !layer.value("mask").toBool() && (blend!=27 || layer.value("group").toBool()) && !(layer.value("effectiveLocks").toUInt()&PAINT_LOCK_ALL))found=true;}
    if(!found || (m_blendPreviewLayer==id && m_blendPreviewMode==blend))return;
    m_blendPreviewLayer=id;m_blendPreviewMode=blend;
    for(int view=0;view<2;++view)if(m_viewEnabled[view]) {
        m_requests[view]=0;requestViewport(view,m_requestedRegions[view],m_requestedPixels[view]);
    }
}
void PaintCoreClient::clearLayerBlendPreview() {
    if(!m_blendPreviewLayer)return;
    m_blendPreviewLayer=0;
    for(int view=0;view<2;++view)if(m_viewEnabled[view]) {
        m_requests[view]=0;requestViewport(view,m_requestedRegions[view],m_requestedPixels[view]);
    }
}
bool PaintCoreClient::setLayerLocks(quint64 id,int locks) { return locks>=0 && locks<=7 && setAppearance(id,"locks",locks); }
void PaintCoreClient::requestLayerPreview(quint64 id) {
    if(!m_closing) QMetaObject::invokeMethod(m_worker,[worker=m_worker,id]{worker->referencePreview(id,true);},Qt::QueuedConnection);
}
void PaintCoreClient::releaseLayerPreview(quint64 id) {
    if(!m_closing) QMetaObject::invokeMethod(m_worker,[worker=m_worker,id]{worker->referencePreview(id,false);},Qt::QueuedConnection);
}
bool PaintCoreClient::moveLayer(quint64 id,int dx,int dy) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || (!dx && !dy)) return false;
    uint64_t sequence=0; PaintStatus status; QString error;
    { QMutexLocker lock(&m_connection->mutex); if(!m_connection->session) return false;
      status=paint_session_move_layer(m_connection->core,m_connection->session,id,dx,dy,&sequence);
      if(status!=PAINT_OK) error=immediateError(status);
    }
    if(status!=PAINT_OK) { showError(error); return false; }
    m_pendingLayer=m_pendingMutation=sequence; m_modified=true; emit stateChanged(); return true;
}
bool PaintCoreClient::pickLayer(QPointF point,int dx,int dy) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || m_fileBusy ||
       !std::isfinite(point.x()) || !std::isfinite(point.y()) || point.x()<0 || point.y()<0 ||
       point.x()>=m_width || point.y()>=m_height) return false;
    uint64_t sequence=0; PaintStatus status;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;
     status=paint_session_pick_layer(m_connection->core,m_connection->session,
         uint32_t(std::floor(point.x())),uint32_t(std::floor(point.y())),dx,dy,&sequence);}
    if(status!=PAINT_OK)return false;
    m_pendingLayer=sequence;if(dx || dy)m_pendingMutation=sequence;emit stateChanged();return true;
}
bool PaintCoreClient::textInputFocused() const {
    const auto *focus=QGuiApplication::focusObject();
    // CanvasItem also has cursorPosition; that property does not identify a text editor.
    return focus && (focus->inherits("QQuickTextInput") || focus->inherits("QQuickTextEdit"));
}
bool PaintCoreClient::layerShortcutsAllowed() const {
    return m_ready && !m_liquifyActive && !m_transformActive && !m_drawing && !layerEditBusy() && !m_fileBusy && !m_closing
        && !textInputFocused();
}
bool PaintCoreClient::canPasteLayers() const {return m_ready && !m_liquifyActive && !m_transformActive && !m_drawing && !layerEditBusy() && !m_fileBusy && QGuiApplication::clipboard()->mimeData()->hasFormat("application/x-drawverse-layers-v1");}
QString PaintCoreClient::mergeLayersText() const {
    if(m_selectedLayers.size()>1)return QStringLiteral("合并图层");
    for(const auto &v:m_layers){const auto l=v.toMap();if(l.value("id").toULongLong()==m_active)return l.value("group").toBool()?QStringLiteral("合并组"):QStringLiteral("向下合并");}
    return QStringLiteral("向下合并");
}
bool PaintCoreClient::canMergeLayers() const {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy)return false;
    QVariantMap active;
    for(const auto &v:m_layers){const auto l=v.toMap();if(l.value("id").toULongLong()==m_active)active=l;
        if(m_selectedLayers.contains(l.value("id").toULongLong()) && (l.value("effectiveLocks").toUInt()&PAINT_LOCK_ALL))return false;}
    if(active.isEmpty() || active.value("mask").toBool())return false;
    if(m_selectedLayers.size()>1 || active.value("group").toBool())return true;
    bool found=false;
    for(const auto &v:m_layers){const auto l=v.toMap();if(l.value("id").toULongLong()==m_active){found=true;continue;}
        if(found && !l.value("mask").toBool() && l.value("parent")==active.value("parent"))return !(l.value("effectiveLocks").toUInt()&PAINT_LOCK_ALL);}
    return false;
}
bool PaintCoreClient::submitLayerAction(quint32 kind,const QByteArray &bytes) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy || m_closing)return false;
    clearLayerBlendPreview();
    const std::vector<uint64_t> ids(m_selectedLayers.begin(),m_selectedLayers.end());
    const bool paste=kind==PAINT_LAYER_ACTION_PASTE;
    if(!paste && ids.empty())return false;
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;
        status=paint_session_layer_action(m_connection->core,m_connection->session,kind,paste?nullptr:ids.data(),paste?0:static_cast<uint32_t>(ids.size()),paste?m_active:0,
            paste?reinterpret_cast<const uint8_t*>(bytes.constData()):nullptr,paste?static_cast<uint64_t>(bytes.size()):0,&sequence);
        if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}
    m_pendingLayer=sequence;
    if(kind==PAINT_LAYER_ACTION_COPY){m_pendingCopy=sequence;QMetaObject::invokeMethod(m_worker,[worker=m_worker,sequence]{worker->watchLayerCopy(sequence);},Qt::QueuedConnection);}
    else {m_pendingMutation=sequence;m_modified=true;}
    emit stateChanged();return true;
}
bool PaintCoreClient::copySelectedLayers(){return submitLayerAction(PAINT_LAYER_ACTION_COPY);}
bool PaintCoreClient::pasteLayers(){return canPasteLayers() && submitLayerAction(PAINT_LAYER_ACTION_PASTE,QGuiApplication::clipboard()->mimeData()->data("application/x-drawverse-layers-v1"));}
bool PaintCoreClient::duplicateSelectedLayers(){return submitLayerAction(PAINT_LAYER_ACTION_DUPLICATE);}
bool PaintCoreClient::mergeSelectedLayers(){return canMergeLayers() && submitLayerAction(PAINT_LAYER_ACTION_MERGE);}

bool PaintCoreClient::submitGroup(quint32 kind,quint64 id,quint64 parent,const QString &name) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy) return false;
    const auto bytes=name.toUtf8();auto request=dto<PaintGroupRequest>();request.kind=kind;request.layer_id=id;request.parent_id=parent;
    if(kind==PAINT_GROUP_WRAP) {request.name=reinterpret_cast<const uint8_t*>(bytes.constData());request.name_length=static_cast<uint64_t>(bytes.size());}
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session) return false;
        status=paint_session_group(m_connection->core,m_connection->session,&request,&sequence);
        if(status!=PAINT_OK) error=immediateError(status);
    }
    if(status!=PAINT_OK) {showError(error);return false;}
    m_pendingLayer=m_pendingMutation=sequence;m_modified=true;emit stateChanged();return true;
}
bool PaintCoreClient::groupLayer(quint64 id,const QString &name) {return submitGroup(PAINT_GROUP_WRAP,id,0,name);}
bool PaintCoreClient::groupSelectedLayers(const QString &name) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy || m_selectedLayers.isEmpty())return false;
    clearLayerBlendPreview();
    const std::vector<uint64_t> ids(m_selectedLayers.begin(),m_selectedLayers.end());
    const auto bytes=name.toUtf8();uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;
        status=paint_session_group_layers(m_connection->core,m_connection->session,ids.data(),static_cast<uint32_t>(ids.size()),
            reinterpret_cast<const uint8_t*>(bytes.constData()),static_cast<uint64_t>(bytes.size()),&sequence);
        if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}
    m_pendingLayer=m_pendingMutation=sequence;m_modified=true;emit stateChanged();return true;
}
bool PaintCoreClient::ungroupLayer(quint64 id) {return submitGroup(PAINT_GROUP_UNGROUP,id,0);}
bool PaintCoreClient::reparentLayer(quint64 id,quint64 parent) {return submitGroup(PAINT_GROUP_REPARENT,id,parent);}
bool PaintCoreClient::addMask(quint64 id) {return !layerEditBusy() && !m_drawing && !m_fileBusy && submit(AddMask,{},id);}
bool PaintCoreClient::setLayerClipping(quint64 id,bool enabled) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy) return false;
    auto options=dto<PaintLayerClipping>();options.enabled=enabled?1u:0u;
    uint64_t seq=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;
        status=paint_session_set_layer_clipping(m_connection->core,m_connection->session,id,&options,&seq);
        if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}
    m_pendingLayer=m_pendingMutation=seq;m_modified=true;emit stateChanged();return true;
}
bool PaintCoreClient::toggleActiveClipping() {
    quint64 id=m_active;
    for(const auto &v:m_layers){const auto l=v.toMap();if(l.value("id").toULongLong()==id && l.value("mask").toBool()){id=l.value("parent").toULongLong();break;}}
    for(const auto &v:m_layers){const auto l=v.toMap();if(l.value("id").toULongLong()==id)return setLayerClipping(id,!l.value("clipped").toBool());}
    return false;
}
bool PaintCoreClient::dropLayer(quint64 id,quint64 target,int placement) {
    return dropLayers({id},target,placement);
}
bool PaintCoreClient::dropSelectedLayers(quint64 target,int placement) {
    return dropLayers(m_selectedLayers,target,placement);
}
bool PaintCoreClient::dropLayers(const QList<quint64> &ids,quint64 target,int placement) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy || ids.isEmpty())return false;
    const std::vector<uint64_t> input(ids.begin(),ids.end());
    uint64_t seq=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;
        status=paint_session_drop_layers(m_connection->core,m_connection->session,input.data(),static_cast<uint32_t>(input.size()),target,static_cast<uint32_t>(placement),&seq);
        if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}
    m_pendingLayer=m_pendingMutation=seq;m_modified=true;
    if(placement==0 && m_collapsedGroups.remove(target))emit groupExpansionChanged();
    emit stateChanged();return true;
}
QString PaintCoreClient::layerDragPayload(quint64 id) const {
    const auto ids=m_selectedLayers.contains(id)?m_selectedLayers:QList<quint64>{id};
    QStringList encoded;for(const auto layer:ids)encoded.append(QString::number(layer));
    return QString::number(m_generation)+":"+encoded.join(',');
}
QVariantList PaintCoreClient::layerDragIds(const QString &payload) const {
    const auto parts=payload.split(':');bool generationOk=false;
    if(parts.size()!=2 || parts[0].toULongLong(&generationOk)!=m_generation || !generationOk)return {};
    const auto values=parts[1].split(',');if(values.isEmpty() || values.size()>4096)return {};
    QVariantList result;QSet<quint64> seen;
    for(const auto &text:values){bool ok=false;const auto id=text.toULongLong(&ok);if(!ok || !id || seen.contains(id))return {};seen.insert(id);result.append(id);}
    return result;
}
bool PaintCoreClient::acceptLayerDrop(const QString &payload,quint64 target,int placement) {
    const auto values=layerDragIds(payload);QList<quint64> ids;
    for(const auto &value:values)ids.append(value.toULongLong());
    return !ids.isEmpty() && dropLayers(ids,target,placement);
}
void PaintCoreClient::beginLayerDrag(quint64 id) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy())return;
    for(const auto &v:m_layers)if(v.toMap().value("id").toULongLong()==id) {
        if(v.toMap().value("mask").toBool() || (v.toMap().value("effectiveLocks").toUInt() & 4))return;
        if(!m_selectedLayers.contains(id))selectLayers(id,0);
        QDrag drag(this);auto *mime=new QMimeData;mime->setData("application/x-drawverse-layer",layerDragPayload(id).toUtf8());drag.setMimeData(mime);drag.exec(Qt::MoveAction);return;
    }
}
void PaintCoreClient::addDefaultLayer() {
    const auto number=[](int n){const QString digits=QStringLiteral("零一二三四五六七八九");const QString units=QStringLiteral("千百十");QString result;bool zero=false;for(int d=1000,u=0;d>1;d/=10,++u){const int v=n/d;n%=d;if(v){if(zero)result+=digits[0];if(!(d==10 && v==1 && result.isEmpty()))result+=digits[v];result+=units[u];zero=false;}else if(!result.isEmpty() && n)zero=true;}if(n){if(zero)result+=digits[0];result+=digits[n];}return result;};
    // Names do not count groups/masks and do not reuse an existing default name.
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy) return;
    QSet<QString> names;
    for(const auto &v:m_layers) names.insert(v.toMap().value("name").toString());
    int serial=m_nextDefaultLayer;QString name;
    for(;serial<=4096;++serial){name=QStringLiteral("图层")+number(serial);if(!names.contains(name))break;}
    if(serial>4096){showError(QStringLiteral("默认图层编号达到上限"));return;}
    m_nextDefaultLayer=serial+1;addLayer(name);
}
QVariantList PaintCoreClient::collapsedGroups() const {
    QVariantList result;for(auto id:m_collapsedGroups) result.append(QVariant::fromValue<qulonglong>(id));return result;
}
void PaintCoreClient::toggleGroupExpanded(quint64 id) {
    for(const auto &value:m_layers) {const auto layer=value.toMap();if(layer.value("id").toULongLong()==id && layer.value("group").toBool()) {
        if(!m_collapsedGroups.remove(id)) m_collapsedGroups.insert(id);emit groupExpansionChanged();return;
    }}
}
void PaintCoreClient::newTransparentDocument(int w,int h) {
    if(w<1 || h<1 || w>1000000 || h>1000000) {showError(QStringLiteral("画布尺寸支持 1–1,000,000 像素"));return;}
    if(m_ready && !m_drawing && !m_fileBusy && submit(New,{},static_cast<quint64>(w),{},h)) {m_documentUrl={};m_sourceUrl={};emit stateChanged();}
}
void PaintCoreClient::newDocument(int w, int h) {
    if (w < 1 || h < 1 || w > 1000000 || h > 1000000) { showError(QStringLiteral("画布尺寸支持 1–1,000,000 像素")); return; }
    if (m_ready && !m_drawing && !m_fileBusy && submit(NewWhite, {}, static_cast<quint64>(w), {}, h)) {
        m_documentUrl = {}; m_sourceUrl = {}; emit stateChanged();
    }
}
QString PaintCoreClient::documentName() const {
    return m_sourceUrl.isEmpty() ? QStringLiteral("未命名") : QFileInfo(m_sourceUrl.toLocalFile()).fileName();
}
bool PaintCoreClient::openDocument(const QUrl &path) { return submitFile(path, PAINT_FILE_OPEN, PAINT_FILE_AUTO); }
bool PaintCoreClient::inspectBrushSource() {
    if(m_closing || !m_ready)return false;
    const auto identity=m_brushLibrary->sourceResource();
    if(identity.isEmpty())return false;
    if(m_brushSourceBusy && m_brushSourceIdentity==identity)return true;
    m_brushSourceIdentity=identity;const auto token=++m_brushSourceToken;
    m_brushSourceInfo.clear();m_brushSourceMessage.clear();m_brushSourceBusy=true;emit brushSourceChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,identity,token]{worker->inspectBrushSource(identity,token);},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::importBrushes(const QUrl &path) {
    if(m_closing || !m_ready || m_brushFileBusy || !path.isLocalFile() || path.toLocalFile().isEmpty())return false;
    m_brushImportMessage.clear();
    m_brushFileBusy=true;emit brushFileChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,path=path.toLocalFile()]{worker->brushFile(path,{},false);},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::importBrushTip(const QUrl &path,int interpretation) {
    if(m_closing || !m_ready || m_brushFileBusy || !path.isLocalFile() || path.toLocalFile().isEmpty() || interpretation<0 || interpretation>2)return false;
    const auto target=m_brushLibrary->selectedId();m_brushImportMessage.clear();m_brushFileBusy=true;emit brushFileChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,path=path.toLocalFile(),target,interpretation]{worker->importBrushTip(path,target,interpretation);},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::importBrushTexture(const QUrl &path,int interpretation) {
    if(m_closing || !m_ready || m_brushFileBusy || !path.isLocalFile() || path.toLocalFile().isEmpty() || interpretation<0 || interpretation>2)return false;
    const auto target=m_brushLibrary->selectedId();m_brushImportMessage.clear();m_brushFileBusy=true;emit brushFileChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,path=path.toLocalFile(),target,interpretation]{worker->importBrushTip(path,target,interpretation,true);},Qt::QueuedConnection);return true;
}
void PaintCoreClient::persistBrushPreferences(const QByteArray &state) {
    if(!m_brushPreferencePersistenceEnabled || m_closing)return;
    {QMutexLocker lock(&m_brushCatalogConnection->mutex);m_brushCatalogConnection->preferences=state;}
    if(!synchronizeBrushCatalog(state))return;
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,state]{worker->saveBrushPreferences(state);},Qt::QueuedConnection);
}
bool PaintCoreClient::synchronizeBrushCatalog(const QByteArray &state) {
    const auto root=QJsonDocument::fromJson(state).object();
    const auto source=QJsonDocument(QJsonObject{{"version",root.value("version")},{"selected",root.value("selected")},{"presets",root.value("presets")}}).toJson(QJsonDocument::Compact);
    QMutexLocker lock(&m_brushCatalogConnection->mutex);
    if(!m_brushCatalogConnection->library)return false;
    if(source==m_brushCatalogConnection->source && m_brushCatalogConnection->preparation)return true;
    uint64_t preparation=0;const auto status=paint_brush_library_prepare_state(m_brushCatalogConnection->library,reinterpret_cast<const uint8_t*>(source.constData()),quint64(source.size()),&preparation);
    if(status!=PAINT_OK){lock.unlock();showError(immediateError(status));return false;}
    m_brushCatalogConnection->source=source;m_brushCatalogConnection->preparation=preparation;return true;
}
bool PaintCoreClient::exportBrush(const QUrl &path,bool includeSource,const QString &expectedPreset) {
    if(m_closing || !m_ready || m_brushFileBusy || !path.isLocalFile() || path.toLocalFile().isEmpty())return false;
    if(!expectedPreset.isEmpty() && expectedPreset!=m_brushLibrary->selectedId()){showError(QStringLiteral("待导出的笔刷已更改，请重新选择导出"));return false;}
    const auto package=m_brushLibrary->exportPackage(includeSource);
    if(package.isEmpty()){showError(QStringLiteral("请在导出选项中选择包含原始 ABR 包；原始包可能包含其他预设和资源"));return false;}
    m_brushFileBusy=true;emit brushFileChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,path=path.toLocalFile(),package]{worker->brushFile(path,package,true);},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::saveDocument(const QUrl &path, int format) { return submitFile(path, PAINT_FILE_SAVE, format); }
bool PaintCoreClient::submitFile(const QUrl &path, int kind, int format) {
    if (!m_ready || m_liquifyActive || m_transformActive || m_drawing || m_fileBusy || m_closing) return false;
    clearLayerBlendPreview();
    if (!path.isLocalFile() || path.toLocalFile().isEmpty()) { showError(QStringLiteral("请选择本地文件")); return false; }
    auto request = dto<PaintFileRequest>(); request.kind = static_cast<uint32_t>(kind);
    request.format = static_cast<uint32_t>(format); request.quality = 95; request.document_generation = m_generation;
    for (auto &channel : request.linear_background) channel = 1;
    const auto bytes = path.toLocalFile().toUtf8(); request.path = reinterpret_cast<const uint8_t *>(bytes.constData());
    request.path_length = static_cast<uint64_t>(bytes.size());
    uint64_t job = 0; PaintStatus status = PAINT_BUSY; QString error;
    { QMutexLocker lock(&m_connection->mutex);
      if (m_connection->session) status = paint_session_file_submit(m_connection->core, m_connection->session, &request, &job);
      if (status == PAINT_OK) m_connection->fileJob.store(job); else error = immediateError(status);
    }
    if (status != PAINT_OK) { showError(error); return false; }
    clearError(); m_fileId = job; m_fileUrl = path; m_fileBusy = true; m_fileOpening = kind == PAINT_FILE_OPEN;
    if (m_fileOpening) m_ready = false;
    emit stateChanged(); return true;
}
void PaintCoreClient::cancelFile() {
    if (!m_fileBusy || m_closing) return;
    PaintStatus status = PAINT_OK;
    { QMutexLocker lock(&m_connection->mutex);
      if (m_connection->session) status = paint_session_file_cancel(m_connection->core, m_connection->session, m_fileId);
    }
    if (status != PAINT_OK && status != PAINT_BUSY) showError(immediateError(status));
}
#include "PaintCoreClient.moc"

bool PaintCoreClient::inspectStorage() {
    if (m_closing || m_storageBusy) return false;
    m_storageBusy=true; emit storageChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker]{worker->inspectStorage();},Qt::QueuedConnection);
    return true;
}
bool PaintCoreClient::saveStorageSettings(const QString &directory,int memoryMiB,int scratchGiB,int reserveMiB) {
    if (m_closing || m_storageBusy) return false;
    const QVariantMap values{{"directory",directory},{"memoryMiB",memoryMiB},{"scratchGiB",scratchGiB},{"reserveMiB",reserveMiB}};
    m_storageBusy=true; m_storageMessage.clear(); emit storageChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,values]{worker->saveStorage(values);},Qt::QueuedConnection);
    return true;
}

bool PaintCoreClient::placeImage(const QUrl &path){return submitFile(path,PAINT_FILE_PLACE,PAINT_FILE_AUTO);}
bool PaintCoreClient::submitTransform(quint32 kind,const PaintTransform *geometry){
    if(!m_ready || m_closing || m_fileBusy || m_drawing || (kind==PAINT_TRANSFORM_BEGIN && (m_liquifyActive || m_transformActive || layerEditBusy())))return false;
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;if(kind==PAINT_TRANSFORM_BEGIN){QList<quint64> ids=m_selectedLayers;if(ids.isEmpty())ids={m_active};QVector<uint64_t> targets;for(auto id:ids)targets.append(id);status=paint_session_begin_transform_layers(m_connection->core,m_connection->session,targets.data(),targets.size(),&sequence);}else status=paint_session_transform(m_connection->core,m_connection->session,kind,geometry,&sequence);if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}m_lastTransform=sequence;
    if(kind!=PAINT_TRANSFORM_UPDATE)m_pendingTransform=sequence;
    if(kind==PAINT_TRANSFORM_BEGIN)m_transformActive=true;
    if(kind==PAINT_TRANSFORM_COMMIT || kind==PAINT_TRANSFORM_RASTERIZE)m_pendingMutation=sequence;
    emit stateChanged();return true;
}
bool PaintCoreClient::beginTransform(){clearLayerBlendPreview();return submitTransform(PAINT_TRANSFORM_BEGIN);}
bool PaintCoreClient::updateTransform(const QVariantList &points,bool warp,int interpolation,qreal skewH,qreal skewV){if(!m_transformActive || points.size()!=16)return false;auto g=dto<PaintTransform>();g.warp=warp;g.interpolation=interpolation;g.skew_h=qIsFinite(skewH)?skewH:m_transformSkewH;g.skew_v=qIsFinite(skewV)?skewV:m_transformSkewV;for(int i=0;i<16;++i){const auto p=points[i].toPointF();g.points[i*2]=p.x();g.points[i*2+1]=p.y();}if(!submitTransform(PAINT_TRANSFORM_UPDATE,&g))return false;m_transformPoints=points;m_transformWarp=warp;m_transformInterpolation=interpolation;m_transformSkewH=g.skew_h;m_transformSkewV=g.skew_v;emit transformChanged();return true;}
bool PaintCoreClient::checkpointTransform(){return m_transformActive && submitTransform(PAINT_TRANSFORM_CHECKPOINT);}
bool PaintCoreClient::finishTransform(bool commit){return m_transformActive && submitTransform(commit?PAINT_TRANSFORM_COMMIT:PAINT_TRANSFORM_CANCEL);}
bool PaintCoreClient::rasterizeImage(){return !m_transformActive && submitTransform(PAINT_TRANSFORM_RASTERIZE);}

bool PaintCoreClient::deferTransformAction(std::function<void()> action){if(!m_transformActive)return false;m_afterTransform=std::move(action);if(!finishTransform(true))m_afterTransform={};return true;}
