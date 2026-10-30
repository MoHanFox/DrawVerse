#include "platform/InteractionTrace.h"
#include "platform/TabletInput.h"
#include "platform/MemoryBudget.h"
#include "brushes/BrushCurveModes.h"
#include "engine/PaintCoreClient.h"
#include <QtEndian>
#include <cstring>
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
#include <limits>

Q_DECLARE_METATYPE(PaintSessionInfo)
Q_DECLARE_METATYPE(CompositionPacketPtr)
Q_DECLARE_METATYPE(PaintFileJobInfo)
namespace {
enum Operation { New = 1, Begin, Move, End, Cancel, Undo, Redo, Add, Remove, Select, Properties, NewWhite, AddMask, AirbrushTick };
template<class T> T dto() { T value{}; value.struct_size = sizeof(T); return value; }
float linear(float v) { return v <= .04045f ? v / 12.92f : std::pow((v + .055f) / 1.055f, 2.4f); }
void raiseBrushStateVersion(QJsonObject &state, int version) {
    state["version"] = std::max(state.value("version").toInt(), version);
}
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
    std::atomic_bool compositionPending{false};
    CompositionViewRequest compositionView;
    std::atomic_bool residentPreviewPending{false};
    std::atomic_bool transformMeshAssets{false};
    std::atomic_bool transformSourcePairSupported{false};
    std::atomic<quint32> transformMaxEdge{0};
    std::atomic<quint64> transformMaxBytes{0};
    std::atomic<quint64> fileJob{0};
};
struct BrushCatalogConnection {
    QMutex mutex;
    PaintBrushLibrary *library=nullptr;
    PaintWetTool *wet=nullptr;
    quint64 preparation=0;
    int workers=0;
    QByteArray source,preferences,requestedState;
};
class BackendWorker final : public QObject {
    Q_OBJECT
public:
    explicit BackendWorker(std::shared_ptr<BackendConnection> connection, QString settingsFile,std::shared_ptr<BrushCatalogConnection> catalog) : m_catalog(std::move(catalog)),m_settingsFile(std::move(settingsFile)), m_connection(std::move(connection)) {
        m_timer = new QTimer(this); m_timer->setInterval(16);
        connect(m_timer, &QTimer::timeout, this, &BackendWorker::poll);
        m_compositionTimer=new QTimer(this);m_compositionTimer->setInterval(2);
        m_compositionTimer->setTimerType(Qt::PreciseTimer);
        connect(m_compositionTimer,&QTimer::timeout,this,[this]{
            if(!m_session || m_stopped){m_compositionTimer->stop();return;}
            auto info=dto<PaintSessionInfo>();
            if(paint_session_info(m_core,m_session,&info)==PAINT_OK)pollComposition(info);
        });
    }
    void boot() {
        auto version = dto<PaintVersion>();
        if (!check(paint_core_version(&version))) return;
        if (version.major != PAINT_ABI_MAJOR || version.minor < 93) {
            emit failure(QStringLiteral("当前功能需要核心 ABI 1.93 或兼容后续版本")); return;
        }
        m_fullResolutionResidentSupported=version.minor>=90;
        m_connection->transformSourcePairSupported.store(version.minor>=91);
        auto settings = preferences();
        {QMutexLocker lock(&m_catalog->mutex);
            if(!m_catalog->library){
                if(!check(paint_brush_library_create(0,&m_catalog->library)))return;
                const auto directory=QFileInfo(settings->fileName()).dir().filePath("brush-resources").toUtf8();
                if(!check(paint_brush_library_bind_resources(m_catalog->library,reinterpret_cast<const uint8_t*>(directory.constData()),quint64(directory.size())))){paint_brush_library_destroy(&m_catalog->library);return;}
            }
            if(!m_catalog->wet && !check(paint_wet_tool_create(&m_catalog->wet))){if(!m_catalog->workers)paint_brush_library_destroy(&m_catalog->library);return;}
            ++m_catalog->workers;m_catalogLease=true;
        }
        prepareBrushPreferences(settings->value("brushes/state").toByteArray());
        emit liquifyPreferences({{"tool",settings->value("liquify/tool",0)},{"size",settings->value("liquify/size",200)},{"strength",settings->value("liquify/strength",50)},{"density",settings->value("liquify/density",100)},{"speed",settings->value("liquify/speed",100)},{"pressure",settings->value("liquify/pressure",false)}});
        m_historyLimit = std::clamp(settings->value("history/limit", 100).toInt(), 10, 1000);
        m_saved = QVariantMap{{"directory", settings->value("storage/directory", "")},
            {"memoryMiB", settings->value("storage/memoryMiB", defaultMemoryMiB())},
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
        m_active["memoryMiB"]=qulonglong(options.resident_bytes/(1024*1024));
        inspectStorage();
        auto caps = dto<PaintCapabilities>();
        if (!check(paint_core_capabilities(m_core, &caps))) return;
        constexpr uint64_t required = PAINT_FEATURE_STROKE_REPLAY_OUTLINE | PAINT_FEATURE_TASK_PROGRESS | PAINT_FEATURE_HISTORY_SELECT | PAINT_FEATURE_DUAL_TIP_TEXTURE | PAINT_FEATURE_STROKE_OUTLINE | PAINT_FEATURE_BRUSH_GROUP_PACKAGE | PAINT_FEATURE_CURVE_NODE_MODES | PAINT_FEATURE_BRUSH_GROUP_STATE | PAINT_FEATURE_BRUSH_CURSOR_IMAGE | PAINT_FEATURE_WET_DUAL_COLOR | PAINT_FEATURE_STROKE_TAPER | PAINT_FEATURE_WET_BRUSH | PAINT_FEATURE_COMPOSITE_BRUSH_POSE | PAINT_FEATURE_DUAL_COLOR_STROKE | PAINT_FEATURE_DUAL_COLOR_INTERLEAVED | PAINT_FEATURE_AIRBRUSH | PAINT_FEATURE_DUAL_RELATIVE_SIZE | PAINT_FEATURE_PATH_SMOOTHING | PAINT_FEATURE_PRESSURE_SMOOTH | PAINT_FEATURE_BRUSH_WRITE_BLEND | PAINT_FEATURE_ASYNC_SESSION | PAINT_FEATURE_CPU_VIEWPORT | PAINT_FEATURE_FILE_IO | PAINT_FEATURE_STORAGE_SETTINGS | PAINT_FEATURE_LAYER_APPEARANCE | PAINT_FEATURE_LAYER_GROUPS | PAINT_FEATURE_MASKS | PAINT_FEATURE_CLIPPING;
        if ((caps.features & (required | PAINT_FEATURE_SELECTION)) != (required | PAINT_FEATURE_SELECTION)) { emit failure(QStringLiteral("当前核心缺少异步视口或选区能力")); return; }
        auto desc = dto<PaintDocumentDesc>(); desc.width = 960; desc.height = 640;
        desc.working_space = PAINT_WORKING_LINEAR_SRGB; desc.pixel_format = PAINT_STORAGE_RGBA32F_PREMULTIPLIED;
        if (!check(paint_session_create(m_core, &desc, &m_session))) return;
        {PaintWetTool *tool=nullptr;{QMutexLocker lock(&m_catalog->mutex);tool=m_catalog->wet;}uint64_t sequence=0;if(!check(paint_session_bind_wet_tool(m_core,m_session,tool,&sequence)))return;}
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
    void queueWetAction(bool reload,QString id,quint64 preparation,QColor color,QColor background,qreal radius,quint64 after) {
        m_pendingWet={{"reload",reload},{"id",id},{"preparation",QVariant::fromValue(preparation)},{"color",color},{"background",background},{"radius",radius},{"after",QVariant::fromValue(after)}};m_wetClock.restart();pollWetAction();
    }
    void pollWetAction() {
        if(m_pendingWet.isEmpty())return;
        auto session=dto<PaintSessionInfo>();const auto sessionStatus=paint_session_info(m_core,m_session,&session);
        if(sessionStatus!=PAINT_OK){m_pendingWet.clear();emit wetActionReady(false,immediateError(sessionStatus));return;}
        if(session.completed_sequence<m_pendingWet.value("after").toULongLong()){
            if(m_wetClock.elapsed()<3000)return;
            m_pendingWet.clear();emit wetActionReady(false,QStringLiteral("上一操作尚未完成，请稍后重试"));return;
        }
        PaintWetTool *tool=nullptr;PaintBrushLibrary *library=nullptr;{QMutexLocker lock(&m_catalog->mutex);tool=m_catalog->wet;library=m_catalog->library;}
        auto info=dto<PaintWetToolInfoDual>();auto status=paint_wet_tool_info_dual(tool,&info);
        if(status==PAINT_OK && m_pendingWet.value("reload").toBool()) {
            const auto id=m_pendingWet.value("id").toString().toUtf8();const auto preparation=m_pendingWet.value("preparation").toULongLong();uint64_t required=0;
            status=paint_brush_library_prepared_state(library,preparation,nullptr,0,&required);
            if(status==PAINT_BUSY && m_wetClock.elapsed()<3000)return;
            if(status==PAINT_BUFFER_TOO_SMALL){const auto color=m_pendingWet.value("color").value<QColor>();auto desc=dto<PaintWetLoadDesc>();desc.radius=m_pendingWet.value("radius").toFloat();desc.linear_rgba[0]=linear(color.redF());desc.linear_rgba[1]=linear(color.greenF());desc.linear_rgba[2]=linear(color.blueF());desc.linear_rgba[3]=color.alphaF();const auto color2=m_pendingWet.value("background").value<QColor>();auto background=dto<PaintBrushBackgroundDesc>();background.linear_rgba[0]=linear(color2.redF());background.linear_rgba[1]=linear(color2.greenF());background.linear_rgba[2]=linear(color2.blueF());background.linear_rgba[3]=color2.alphaF();status=paint_wet_tool_reload_prepared_dual(tool,library,preparation,reinterpret_cast<const uint8_t*>(id.constData()),quint64(id.size()),&desc,&background);}
        }else if(status==PAINT_OK)status=paint_wet_tool_clean(tool);
        m_pendingWet.clear();emit wetActionReady(status==PAINT_OK,status==PAINT_OK?QString{}:immediateError(status));
    }
    // Disk queries and QSettings synchronization stay on the backend thread.
    void inspectStorage() { evaluateStorage(m_saved, false); }
    void inspectCompute() {
        if (!m_core || !m_session || m_stopped) return;
        uint64_t size=0;
        if(paint_compute_hardware(m_core,nullptr,0,&size)==PAINT_BUFFER_TOO_SMALL && size<=65536) {
            QByteArray bytes(qsizetype(size),'\0');
            if(paint_compute_hardware(m_core,reinterpret_cast<uint8_t*>(bytes.data()),size,&size)==PAINT_OK)
                m_computeHardware=QJsonDocument::fromJson(bytes).array().toVariantList();
        }
        pollCompute(true);
    }
    void saveCompute(const QString &id) {
        if(!m_session || m_stopped)return;
        auto settings=preferences();settings->setValue("performance/computeDevice",id);settings->sync();
        if(settings->status()!=QSettings::NoError){emit computeState(m_computeHardware,m_computeDevice,QStringLiteral("无法保存 GPU 设置"),false,settings->value("performance/highestQuality",false).toBool());return;}
        pollCompute(true);
    }
    void pollCompute(bool force=false) {
        if(!m_session || m_stopped || (!force && m_computeClock.isValid() && m_computeClock.elapsed()<500))return;
        m_computeClock.restart();
        auto info=dto<PaintSessionInfo>();if(paint_session_info(m_core,m_session,&info)!=PAINT_OK)return;
        const auto settings=preferences();
        const auto desired=settings->value("performance/computeDevice","auto").toString();
        if((desired!=m_computeDevice) && !info.stroke_active && !(info.flags&(PAINT_SESSION_LIQUIFY|PAINT_SESSION_TRANSFORM|PAINT_SESSION_LAYER_MOVE|PAINT_SESSION_ADJUSTMENT))) {
            const auto bytes=desired.toUtf8();quint64 sequence=0;
            if(paint_session_set_compute(m_core,m_session,reinterpret_cast<const uint8_t*>(bytes.constData()),quint64(bytes.size()),&sequence)==PAINT_OK) {
                m_computeDevice=desired;m_computeSequence=sequence;
            }
        }
        QString status=QStringLiteral("正在初始化计算硬件…");
        const bool busy=info.completed_sequence<m_computeSequence;
        if(!busy){uint64_t size=0;if(paint_session_compute_status(m_core,m_session,nullptr,0,&size)==PAINT_BUFFER_TOO_SMALL && size<=65536){QByteArray bytes(qsizetype(size),'\0');if(paint_session_compute_status(m_core,m_session,reinterpret_cast<uint8_t*>(bytes.data()),size,&size)==PAINT_OK)status=QString::fromUtf8(bytes);}}
        emit computeState(m_computeHardware,m_computeDevice,status,busy,settings->value("performance/highestQuality",false).toBool());
    }
    void saveStorage(const QVariantMap &values) { evaluateStorage(values, true); }
    void prepareBrushPreferences(QByteArray json) {
        {QMutexLocker lock(&m_catalog->mutex);if(!m_catalog->preferences.isEmpty())json=m_catalog->preferences;}
        emit brushPreferences(json,BrushLibrary::preparePreferences(json));
    }
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
    void cancelBrushPreview(const QString &id,quint64 token) {
        m_brushQueue.removeIf([&](const QVariant &value){const auto job=value.toMap();return job.value("id").toString()==id && job.value("token").toULongLong()==token;});
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
        m_stopped = true; m_timer->stop();m_compositionTimer->stop();
        m_compositionReader.reset(m_core,m_session);
        if(!m_pendingWet.isEmpty()){m_pendingWet.clear();emit wetActionReady(false,QStringLiteral("后端已关闭"));}
        while(!m_pendingBrushPreferences.isEmpty()){pollBrushState();if(!m_pendingBrushPreferences.isEmpty())QThread::msleep(1);}
        { QMutexLocker lock(&m_connection->mutex); m_connection->session = nullptr; m_connection->core = nullptr; }
        // Release the GUI lock before cancellation and thread joins.
        if(m_brushSession) check(paint_session_destroy(m_core,&m_brushSession));
        if(m_previewCatalog)check(paint_brush_library_destroy(&m_previewCatalog));
        if(m_previewWet)check(paint_wet_tool_destroy(&m_previewWet));
        if (m_session) check(paint_session_destroy(m_core, &m_session));
        if (m_core) check(paint_core_destroy(&m_core));
        {QMutexLocker lock(&m_catalog->mutex);if(m_catalogLease && --m_catalog->workers==0){check(paint_brush_library_destroy(&m_catalog->library));if(m_catalog->wet)check(paint_wet_tool_destroy(&m_catalog->wet));m_catalog->preparation=0;m_catalog->source.clear();}m_catalogLease=false;}
        emit finished();
    }
    void watchLayerCopy(quint64 sequence) {m_layerCopySequence=sequence;}
    void queueBrushOutlines(const QStringList &identities) {m_pendingOutlines=identities;}
    void pollBrushOutline() {
        if(m_pendingOutlines.isEmpty())return;
        const auto identity=m_pendingOutlines.takeFirst();const auto bytes=identity.toLatin1();
        PaintBrushLibrary *library=nullptr;{QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
        QByteArray path(4*1024*1024,'\0');uint64_t required=0;
        const auto status=paint_brush_library_tip_outline(library,reinterpret_cast<const uint8_t*>(bytes.constData()),quint64(bytes.size()),reinterpret_cast<uint8_t*>(path.data()),quint64(path.size()),&required);
        if(status==PAINT_BUSY){m_pendingOutlines.append(identity);return;}
        if(status==PAINT_OK)path.resize(qsizetype(required));
        emit brushOutlineReady(identity,status==PAINT_OK?QString::fromLatin1(path):QString{},status==PAINT_OK?QString{}:immediateError(status));
    }
    void queueBrushCursorImages(const QStringList &identities) {m_pendingCursorImages=identities;}
    void listBrushResources() {
        PaintBrushLibrary *library=nullptr;{QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
        QByteArray json(512*1024,'\0');uint64_t required=0;
        const auto status=paint_brush_library_resources_json(library,reinterpret_cast<uint8_t*>(json.data()),quint64(json.size()),&required);
        if(status==PAINT_OK)json.resize(qsizetype(required));
        emit brushResourcesReady(status==PAINT_OK?json:QByteArray{},status==PAINT_OK?QString{}:immediateError(status));
    }
    QString resourcePreview(const QString &identity,QString &error) {
        PaintBrushLibrary *library=nullptr;{QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
        const auto bytes=identity.toLatin1();QByteArray png(512*1024,'\0');uint64_t required=0;
        const auto status=paint_brush_library_tip_cursor_png(library,reinterpret_cast<const uint8_t*>(bytes.constData()),quint64(bytes.size()),reinterpret_cast<uint8_t*>(png.data()),quint64(png.size()),&required);
        if(status!=PAINT_OK){error=immediateError(status);return {};}
        png.resize(qsizetype(required));return QStringLiteral("data:image/png;base64,")+QString::fromLatin1(png.toBase64());
    }
    void previewBrushResource(const QString &identity) {QString error;const auto image=resourcePreview(identity,error);emit brushResourcePreviewReady(identity,image);}
    void selectBrushResource(const QString &identity,const QString &target,int role) {
        QString error;const auto image=resourcePreview(identity,error);
        emit brushResourcePreviewReady(identity,image);
        emit brushTipReady(target,error.isEmpty()?identity:QString{},8,error,role);
    }
    void pollBrushCursorImage() {
        if(m_pendingCursorImages.isEmpty())return;
        const auto identity=m_pendingCursorImages.takeFirst();const auto bytes=identity.toLatin1();
        PaintBrushLibrary *library=nullptr;{QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
        QByteArray png(512*1024,'\0');uint64_t required=0;
        const auto status=paint_brush_library_tip_cursor_png(library,reinterpret_cast<const uint8_t*>(bytes.constData()),quint64(bytes.size()),reinterpret_cast<uint8_t*>(png.data()),quint64(png.size()),&required);
        if(status==PAINT_BUSY){m_pendingCursorImages.append(identity);return;}
        if(status==PAINT_OK)png.resize(qsizetype(required));
        emit brushCursorImageReady(identity,status==PAINT_OK?QStringLiteral("data:image/png;base64,")+QString::fromLatin1(png.toBase64()):QString{},status==PAINT_OK?QString{}:immediateError(status));
    }
    void inspectBrushSource(QString identity,quint64 token) {
        const auto bytes=identity.toLatin1();
        PaintBrushLibrary *library=nullptr;{QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
        QByteArray record(2*1024*1024,'\0');uint64_t required=0;
        const auto status=paint_brush_library_source_info(library,reinterpret_cast<const uint8_t*>(bytes.constData()),quint64(bytes.size()),reinterpret_cast<uint8_t*>(record.data()),quint64(record.size()),&required);
        if(status==PAINT_OK)record.resize(qsizetype(required));
        emit brushSourceReady(identity,token,status==PAINT_OK?record:QByteArray{},status==PAINT_OK?QString{}:immediateError(status));
    }
    void importBrushTip(QString path,QString target,int interpretation,int role=0) {
        PaintBrushLibrary *library=nullptr;
        {QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
        const auto encoded=path.toUtf8();QByteArray identity(64,'\0');uint64_t required=0;uint32_t depth=0;
        const auto status=paint_brush_library_import_tip_file(library,reinterpret_cast<const uint8_t*>(encoded.constData()),quint64(encoded.size()),quint32(interpretation),reinterpret_cast<uint8_t*>(identity.data()),64,&required,&depth);
        emit brushTipReady(target,status==PAINT_OK?QString::fromLatin1(identity):QString{},depth,status==PAINT_OK?QString{}:immediateError(status),role);
    }
    void brushFile(QString path,QByteArray package,bool exporting) {
        if(m_stopped){emit brushFileReady({},QStringLiteral("后端已关闭"),exporting,false);return;}
        const bool abr=!exporting && path.endsWith(".abr",Qt::CaseInsensitive);
        const auto encoded=path.toUtf8();PaintStatus status;
        PaintBrushLibrary *library=nullptr;{QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
        const bool grouped=exporting && QJsonDocument::fromJson(package).object().value("model").toString()=="ordered-package-v1";
        if(exporting)status=grouped?paint_brush_library_package_write_groups(library,reinterpret_cast<const uint8_t*>(encoded.constData()),quint64(encoded.size()),reinterpret_cast<const uint8_t*>(package.constData()),quint64(package.size())):paint_brush_library_package_write_outline(library,reinterpret_cast<const uint8_t*>(encoded.constData()),quint64(encoded.size()),reinterpret_cast<const uint8_t*>(package.constData()),quint64(package.size()));
        else {
            package.resize(2*1024*1024);uint64_t required=0;
            status=abr?paint_brush_library_abr_read(library,reinterpret_cast<const uint8_t*>(encoded.constData()),quint64(encoded.size()),reinterpret_cast<uint8_t*>(package.data()),quint64(package.size()),&required):paint_brush_library_package_read_groups(library,reinterpret_cast<const uint8_t*>(encoded.constData()),quint64(encoded.size()),reinterpret_cast<uint8_t*>(package.data()),quint64(package.size()),&required);
            if(status==PAINT_OK)package.resize(qsizetype(required));
        }
        emit brushFileReady(status==PAINT_OK?package:QByteArray{},status==PAINT_OK?QString{}:immediateError(status),exporting,abr);
    }
    void pollTaskProgress() {
        auto progress=dto<PaintTaskProgress>();
        if(paint_session_task_progress(m_core,m_session,&progress)!=PAINT_OK)return;
        QVariantMap values{{"state",progress.state},{"kind",progress.kind},{"phase",progress.phase},{"status",progress.status},
            {"sequence",qulonglong(progress.sequence)},{"generation",qulonglong(progress.document_generation)},
            {"stage",qulonglong(progress.stage)},{"completed",qulonglong(progress.completed)},{"total",qulonglong(progress.total)},{"elapsedMs",qulonglong(progress.elapsed_ms)}};
        if(values!=m_lastTaskProgress){m_lastTaskProgress=values;emit taskProgress(values);}
    }
signals:
    void taskProgress(QVariantMap values);
    void residentPreviewReady(ResidentPreviewPtr preview);
    void movePreparationReady(quint64 generation,quint64 revision,quint64 layer,quint64 request,ResidentPreviewAssetsPtr assets);
    void wetActionReady(bool success,QString error);
    void brushSourceReady(QString identity,quint64 token,QByteArray record,QString error);
    void brushOutlineReady(QString identity,QString path,QString error);
    void brushCursorImageReady(QString identity,QString image,QString error);
    void brushResourcesReady(QByteArray json,QString error);
    void brushResourcePreviewReady(QString identity,QString image);
    void brushTipReady(QString target,QString resource,int depth,QString error,int role);
    void nativeBrushCatalog(quint64 preparation,QByteArray state,QString error);
    void brushPreparationFinished(quint64 preparation,bool success);
    void brushFileReady(QByteArray package,QString error,bool exporting,bool abr);
    void layerClipboardReady(QByteArray bytes);
    void liquifyPreferences(QVariantMap values);
    void brushPreferences(QByteArray json,BrushLibrary::PreparedPreferencesPtr prepared);
    void brushPreviewReady(QString id,quint64 token,QString image);
    void storageState(QVariantMap saved, QVariantMap active, QVariantMap info, QString message);
    void storageDone(bool success);
    void computeState(QVariantList hardware,QString device,QString status,bool busy,bool highestQuality);
    void metadata(PaintSessionInfo info, QVariantList layers, QVariantMap selection, QVariantList history,QVariantMap transform,QVariantList committedStrokes);
    void thumbnailReady(quint64 generation, quint64 revision, quint64 layer, QString image);
    void pixels(int view, QImage image, QRectF region, quint64 generation, quint64 revision, quint64 request, quint64 frameId);
    void compositionReady(CompositionPacketPtr packet,quint64 serial,QString fallback);
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
        if (memory < 1 || memory > 1048576 || scratch < 1 || scratch > 64 || reserve < 0 || reserve > 1048576) return options;
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
        InteractionTrace::Scope metadataTrace("move.worker.metadata");
        if(metadataTrace.active)metadataTrace.event.value=double(info.completed_sequence);
        InteractionTrace::point("move.worker.metadata.generation",double(info.document_generation));
        InteractionTrace::point("move.worker.metadata.revision",double(info.revision));
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
            uint32_t maskLinked=1;status=paint_session_layer_mask_linked(m_core,m_session,info.publication,layer.layer_id,&maskLinked);if(status==PAINT_BUSY)return false;if(!check(status))return false;
            uint32_t image=0;status=paint_session_layer_is_image(m_core,m_session,info.publication,layer.layer_id,&image);if(status==PAINT_BUSY)return false;if(!check(status))return false;
            layers.prepend(QVariantMap{{"id", QVariant::fromValue<qulonglong>(layer.layer_id)},
                {"image",image!=0},{"name", QString::fromUtf8(name)}, {"visible", layer.visible != 0}, {"opacity", layer.opacity},
                {"fill",appearance.fill},{"blendMode",appearance.blend_mode},{"locks",appearance.locks},
                {"maskLinked",maskLinked!=0},{"mask",hierarchy.kind==PAINT_LAYER_MASK},{"group",hierarchy.kind==PAINT_LAYER_GROUP},{"parent",QVariant::fromValue<qulonglong>(hierarchy.parent_id)},{"depth",hierarchy.depth},{"effectiveLocks",hierarchy.effective_locks},
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
        const QStringList names{QStringLiteral("初始状态"),QStringLiteral("画笔"),QStringLiteral("橡皮擦"),QStringLiteral("全选"),QStringLiteral("矩形选区"),QStringLiteral("取消选区"),QStringLiteral("反选"),QStringLiteral("新建图层"),QStringLiteral("删除图层"),QStringLiteral("图层属性"),QStringLiteral("混合模式"),QStringLiteral("图层填充"),QStringLiteral("图层锁定"),QStringLiteral("移动图层"),QStringLiteral("新建图层组"),QStringLiteral("取消图层组"),QStringLiteral("图层归组"),QStringLiteral("添加蒙版"),QStringLiteral("剪贴蒙版"),QString(),QStringLiteral("椭圆选区"),QStringLiteral("套索"),QStringLiteral("魔棒"),QStringLiteral("释放剪贴蒙版"),QStringLiteral("多边形套索"),QStringLiteral("油漆桶"),QStringLiteral("涂抹"),QStringLiteral("复制图层"),QStringLiteral("粘贴图层"),QStringLiteral("合并图层"),QStringLiteral("液化"),QStringLiteral("液化 · 推拉"),QStringLiteral("液化 · 膨大"),QStringLiteral("液化 · 扭曲"),QStringLiteral("液化 · 重建"),QStringLiteral("液化 · 重置"),QStringLiteral("置入图片"),QStringLiteral("自由变换"),QStringLiteral("栅格化图片"),QStringLiteral("水平翻转"),QStringLiteral("垂直翻转"),QStringLiteral("清除选区内容"),QStringLiteral("曲线"),QStringLiteral("色阶"),QStringLiteral("色相/饱和度/明度"),QStringLiteral("选取不透明像素"),QStringLiteral("调整画布"),QStringLiteral("顺时针 90°"),QStringLiteral("逆时针 90°"),QStringLiteral("羽化选区"),QStringLiteral("通过复制创建图层"),QStringLiteral("通过剪切创建图层"),QStringLiteral("填充选区"),QStringLiteral("描边选区")};
        const QStringList icons{"page","brush","eraser","rectangleSelection","rectangleSelection","rectangleSelection","rectangleSelection","plus","trash","layers","layers","layers","layers","move","folder","folder","folder","mask","layers","page","ellipseSelection","lassoSelection","wand","layers","polygonSelection","bucket","smudge","layers","layers","layers","liquify","liquifyPush","liquifyBloat","liquifyTwirl","liquifyReconstruct","undo","page","move","layers","move","move","eraser","layers","layers","layers","rectangleSelection",QStringLiteral("transform"),"transform","transform","rectangleSelection","layers","layers","bucket","brush"};
        for(uint32_t depth=0;depth<=info.undo_depth+info.redo_depth;++depth) {
            auto entry=dto<PaintHistoryEntry>();auto historyStatus=paint_session_history_entry(m_core,m_session,info.publication,depth,&entry);
            if(historyStatus==PAINT_BUSY)return false;if(!check(historyStatus))return false;
            if(entry.kind==PAINT_HISTORY_TRUNCATED)continue;
            // An unknown kind is shown as generically as possible instead of aborting the publication:
            // dropping it would leave the canvas, layer list and selection stale for good.
            const bool known=entry.kind<static_cast<uint32_t>(names.size()) && entry.kind<static_cast<uint32_t>(icons.size());
            history.append(QVariantMap{{"depth",entry.depth},{"kind",entry.kind},
                {"title",entry.kind==PAINT_HISTORY_INITIAL && (info.flags&PAINT_SESSION_LIQUIFY)?QStringLiteral("液化起始状态"):entry.kind==PAINT_HISTORY_INITIAL && (info.flags&PAINT_SESSION_TRANSFORM)?QStringLiteral("变换起始状态"):entry.kind==PAINT_HISTORY_RELEASE_CLIPPING?QStringLiteral("释放剪贴蒙版"):known?names[entry.kind]:QStringLiteral("未知操作")},
                {"icon",entry.kind==PAINT_HISTORY_RELEASE_CLIPPING?QStringLiteral("layers"):known?icons[entry.kind]:QStringLiteral("page")}});
        }
        uint32_t canvasDpi=72;status=paint_session_canvas_dpi(m_core,m_session,info.publication,&canvasDpi);if(status==PAINT_BUSY)return false;if(!check(status))return false;
        auto transform=dto<PaintTransform>();status=paint_session_transform_info(m_core,m_session,info.publication,&transform);if(status==PAINT_BUSY)return false;if(!check(status))return false;QVariantList transformPoints;for(int i=0;i<16;++i)transformPoints.append(QPointF(transform.points[i*2],transform.points[i*2+1]));
        uint64_t committed[4096];uint32_t count=0;const auto committedStatus=paint_session_committed_strokes(m_core,m_session,info.publication,m_strokeCommitCursor,committed,4096,&count);
        if(committedStatus==PAINT_BUSY)return false;if(!check(committedStatus))return false;
        QVariantList committedStrokes;for(uint32_t i=0;i<count;++i)committedStrokes.append(QVariant::fromValue<qulonglong>(committed[i]));
        QVariantList adjustmentHistogram;
        if(info.flags&PAINT_SESSION_ADJUSTMENT){
            float bins[1024];const auto histStatus=paint_session_adjustment_histogram(m_core,m_session,info.publication,bins,1024);
            if(histStatus==PAINT_BUSY)return false;if(!check(histStatus))return false;
            for(int c=0;c<4;c++){QVariantList channel;for(int i=0;i<256;i++)channel.append(bins[c*256+i]);adjustmentHistogram.append(QVariant(channel));}
        }
        // Only queued metadata has a GUI acknowledgement. A stale publication
        // above must remain retryable, or poll() also stops delivering pixels.
        m_connection->metadataPending.store(true);
        InteractionTrace::point("move.worker.metadata.emit",double(info.completed_sequence));
        emit metadata(info, layers,{{"dpi",canvasDpi},{"enabled",summary.enabled!=0},{"steps",steps},{"adjustmentHistogram",adjustmentHistogram}},history,{{"points",transformPoints},{"warp",transform.warp!=0},{"interpolation",transform.interpolation},{"skewH",transform.skew_h},{"skewV",transform.skew_v}},committedStrokes); m_publication = info.publication;m_strokeCommitCursor=info.completed_sequence;
        return true;
    }
    void transformInputActivity(){
        if(m_stopped)return;
        m_transformInputClock.start();
        m_timer->setInterval(2);
    }
    void watchCompositionAcknowledgement(quint64 sequence){
        pollCompositionAcknowledgement(++m_compositionAckWatch,sequence,256);
    }
    void pollCompositionAcknowledgement(quint64 watch,quint64 sequence,int remaining){
        if(m_stopped || !m_session || watch!=m_compositionAckWatch)return;
        auto info=dto<PaintSessionInfo>();
        if(paint_session_info(m_core,m_session,&info)==PAINT_OK && info.completed_sequence>=sequence && publish(info)){
            InteractionTrace::point("move.worker.compositionAcknowledgement",double(sequence));return;
        }
        // Bounded asynchronous acknowledgement observation only. It never reads
        // pixels, waits for the actor, or replaces the ordinary recovery poll.
        if(remaining>0)QTimer::singleShot(1,this,[this,watch,sequence,remaining]{pollCompositionAcknowledgement(watch,sequence,remaining-1);});
    }
    void pollComposition(const PaintSessionInfo &info){
        CompositionViewRequest view;
        {QMutexLocker lock(&m_connection->mutex);view=m_connection->compositionView;}
        const bool active=info.stroke_active && view.enabled && view.generation==info.document_generation && !(info.flags & PAINT_SESSION_FAILED);
        if(active && !m_compositionTimer->isActive())m_compositionTimer->start();
        else if(!active)m_compositionTimer->stop();
        if(!m_connection->compositionPending.load()){
            QString fallback=m_compositionFallback;
            auto packet=m_compositionReader.poll(m_core,m_session,info,view,fallback);
            if(packet || fallback!=m_compositionFallback){
                m_compositionFallback=fallback;m_connection->compositionPending=true;
                emit compositionReady(std::move(packet),view.serial,fallback);
            }
        }
    }
    void poll() {
        if (!m_session || m_stopped) return;
        pollTaskProgress();
        pollCompute();
        pollBrushState();
        pollWetAction();
        pollBrushOutline();
        pollBrushCursorImage();
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
        pollMovePreparation();
        pollResidentPreview();
        pollComposition(info);
        // Only actual legacy/CPU transform input opens a short response window.
        // Return to the normal idle cadence without presenting unchanged pixels.
        const int pollInterval=m_transformInputClock.isValid() && m_transformInputClock.elapsed()<48?2:(m_residentPreview?8:16);
        if(m_timer->interval()!=pollInterval)m_timer->setInterval(pollInterval);
        for (uint32_t view = 0; view < 2; ++view) {
            // At most one queued QImage per view when the GUI is busy.
            if (m_connection->framePending[view].load()) continue;
            auto frame = dto<PaintFrameInfo>();
            auto status = paint_session_frame_info(m_core, m_session, view, &frame);
            if (status == PAINT_BUSY) continue;
            if (!check(status)) return;
            if (frame.frame_id == m_frames[view]) continue;
            InteractionTrace::Scope frameTrace("move.worker.copyFrame");
            if(frameTrace.active){
                frameTrace.event.value=double(frame.revision);
                InteractionTrace::point("move.copyFrame.view",view);
                InteractionTrace::point("move.copyFrame.generation",double(frame.document_generation));
                InteractionTrace::point("move.copyFrame.request",double(frame.request_id));
                InteractionTrace::point("move.copyFrame.id",double(frame.frame_id));
            }
            QImage image(static_cast<int>(frame.pixel_width), static_cast<int>(frame.pixel_height), QImage::Format_RGBA8888_Premultiplied);
            if (image.isNull()) { emit failure(QStringLiteral("无法分配视口显示缓存")); return; }
            auto tile = dto<PaintTile>(); tile.format = PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED;
            tile.data = image.bits(); tile.capacity = static_cast<uint64_t>(image.sizeInBytes());
            tile.stride = static_cast<uint64_t>(image.bytesPerLine());
            status = paint_session_read_frame(m_core, m_session, view, frame.request_id, frame.frame_id, &tile);
            if (status == PAINT_BUSY) continue;
            if (!check(status)) return;
            InteractionTrace::point("move.copyFrame.bytes",double(image.sizeInBytes()));
            m_frames[view] = frame.frame_id;
            m_connection->framePending[view].store(true);
            emit pixels(static_cast<int>(view), image, QRectF(frame.x, frame.y, frame.width, frame.height),
                frame.document_generation, frame.revision, frame.request_id, frame.frame_id);
        }
    }
    void pollMovePreparation() {
        auto info=dto<PaintMovePreparationInfo>();
        const auto status=paint_session_move_preparation_info(m_core,m_session,&info);
        if(status==PAINT_NOT_FOUND || status==PAINT_BUSY)return;
        if(!check(status) || !(info.flags&PAINT_MOVE_PREPARATION_READY))return;
        const bool sourceReady=(info.flags&PAINT_MOVE_PREPARATION_SOURCE)!=0 && info.source_id!=0;
        if(m_movePreparationPublishedRequest==info.request_id &&
           m_movePreparationPublishedSource==info.source_id)return;
        InteractionTrace::Scope trace("move.worker.decodePreparation");
        if(trace.active)trace.event.value=double(info.request_id);
        InteractionTrace::point("move.preparation.source",double(info.source_id));
        ResidentPreviewAssetsPtr assets;
        if(sourceReady) {
            uint64_t required=0;
            auto read=paint_session_read_move_preparation(m_core,m_session,info.document_generation,
                info.request_id,info.source_id,1,nullptr,0,&required);
            if(read==PAINT_BUSY || read==PAINT_NOT_FOUND)return;
            if(read!=PAINT_BUFFER_TOO_SMALL || required>64ull*1024*1024) {
                check(read);return;
            }
            QByteArray bytes(static_cast<qsizetype>(required),'\0');
            read=paint_session_read_move_preparation(m_core,m_session,info.document_generation,
                info.request_id,info.source_id,1,reinterpret_cast<uint8_t*>(bytes.data()),required,&required);
            if(read==PAINT_BUSY || read==PAINT_NOT_FOUND)return;
            if(!check(read))return;
            auto decoded=std::make_shared<ResidentPreviewAssets>();
            decoded->sourceId=info.source_id;
            if(bytes.size()<24)return;
            qsizetype offset=24;
            for(int i=0;i<3;++i){
                const auto w=qFromLittleEndian<quint32>(bytes.constData()+i*8);
                const auto h=qFromLittleEndian<quint32>(bytes.constData()+i*8+4);
                const quint64 size=quint64(w)*h*4;
                if(!w || !h || w>2048 || h>2048 || size>quint64(bytes.size()-offset))return;
                decoded->images[size_t(i)]=QImage(reinterpret_cast<const uchar*>(bytes.constData()+offset),
                    int(w),int(h),int(w*4),QImage::Format_RGBA8888_Premultiplied).copy();
                if(decoded->images[size_t(i)].isNull())return;
                offset+=qsizetype(size);
            }
            if(offset!=bytes.size())return;
            assets=std::move(decoded);
            m_movePreparation=assets;
        }else{
            m_movePreparation.reset();
        }
        m_movePreparationPublishedRequest=info.request_id;
        m_movePreparationPublishedSource=info.source_id;
        emit movePreparationReady(info.document_generation,info.revision,info.layer_id,info.request_id,assets);
    }
    void pollResidentPreview() {
        if(m_connection->residentPreviewPending.load())return;
        auto info=dto<PaintResidentPreviewInfo>();
        const auto status=paint_session_resident_preview_info(m_core,m_session,&info);
        if(status==PAINT_NOT_FOUND){m_residentPreview.reset();return;}
        if(!check(status))return;
        if(m_residentPreview && m_residentPreview->sourceId==info.source_id && m_residentPreview->revision==info.revision)return;
        InteractionTrace::Scope trace("move.worker.readResident");
        if(trace.active)trace.event.value=double(info.revision);
        InteractionTrace::point("move.resident.source",double(info.source_id));
        InteractionTrace::point("move.resident.generation",double(info.document_generation));
        auto next=std::make_shared<ResidentPreviewFrame>();
        next->generation=info.document_generation;next->revision=info.revision;next->sourceId=info.source_id;
        next->columns=info.columns;next->rows=info.rows;next->documentSize=QSize(int(info.document_width),int(info.document_height));next->focus=(info.focus&1)!=0;next->forward=(info.focus&2)!=0;
        const bool fullResolution=m_fullResolutionResidentSupported && m_connection->transformMeshAssets.load()
            && next->forward && !(info.focus&PAINT_RESIDENT_PREVIEW_SELECTED);
        const auto edge=m_connection->transformMaxEdge.load();
        quint64 alternateId=0,identityId=0;
        if(next->forward && m_connection->transformSourcePairSupported.load()){
            std::array<uint8_t,16> sources{};uint64_t required=0;
            if(paint_session_read_resident_preview(m_core,m_session,info.document_generation,info.revision,info.source_id,4,
                sources.data(),sources.size(),&required)!=PAINT_OK || required!=sources.size())return;
            alternateId=qFromLittleEndian<quint64>(sources.data());identityId=qFromLittleEndian<quint64>(sources.data()+8);
        }
        next->transformIdentity=identityId && identityId==next->sourceId;
        const auto budget=m_connection->transformMaxBytes.load()/(alternateId?2:1);
        const auto read=[&](quint32 kind,QByteArray &bytes,PaintStatus *result=nullptr,quint64 source=0) {
            if(!source)source=info.source_id;
            uint64_t required=0;
            auto s=kind==4?paint_session_read_transform_preview_assets(m_core,m_session,info.document_generation,source,edge,budget,nullptr,0,&required):
                paint_session_read_resident_preview(m_core,m_session,info.document_generation,info.revision,info.source_id,kind,nullptr,0,&required);
            if(result)*result=s;
            if(s==PAINT_BUSY || s==PAINT_NOT_FOUND)return false;
            if(s!=PAINT_BUFFER_TOO_SMALL || (required>(kind==4?budget:64ull*1024*1024) || required>quint64(std::numeric_limits<qsizetype>::max()))){check(s);return false;}
            bytes.resize(qsizetype(required));
            s=kind==4?paint_session_read_transform_preview_assets(m_core,m_session,info.document_generation,source,edge,budget,reinterpret_cast<uint8_t*>(bytes.data()),required,&required):
                paint_session_read_resident_preview(m_core,m_session,info.document_generation,info.revision,info.source_id,kind,reinterpret_cast<uint8_t*>(bytes.data()),required,&required);
            if(result)*result=s;
            return s==PAINT_OK;
        };
        // Capture the small exact-revision mesh BEFORE encoding immutable source
        // images. Pointer input can advance revision throughout that expensive
        // read; reading the mesh afterwards repeatedly discarded valid images
        // until the pointer happened to pause. The GUI mesh path reprojects this
        // captured UV range using its latest authoritative absolute geometry.
        QByteArray mesh;
        const auto count=quint64(next->columns)*next->rows;
        if(next->columns<2 || next->rows<2 || count>257*257)return;
        {
            mesh.resize(qsizetype(count*16));uint64_t required=0;
            if(paint_session_read_resident_preview(m_core,m_session,info.document_generation,info.revision,info.source_id,0,
                reinterpret_cast<uint8_t*>(mesh.data()),uint64_t(mesh.size()),&required)!=PAINT_OK || required!=uint64_t(mesh.size()))return;
        }
        const auto decodeAssets=[](const QByteArray &assets,ResidentPreviewAssets &output,int edge=2048){
            if(assets.size()<24)return false;
            qsizetype offset=24;
            for(int i=0;i<3;++i){
                const auto w=qFromLittleEndian<quint32>(assets.constData()+i*8),h=qFromLittleEndian<quint32>(assets.constData()+i*8+4);
                const quint64 size=quint64(w)*h*4;
                if(!w || !h || w>quint32(edge) || h>quint32(edge) || size>quint64(assets.size()-offset))return false;
                output.images[size_t(i)]=QImage(reinterpret_cast<const uchar*>(assets.constData()+offset),int(w),int(h),int(w*4),QImage::Format_RGBA8888_Premultiplied).copy();
                if(output.images[size_t(i)].isNull())return false;
                offset+=qsizetype(size);
            }
            return offset==assets.size();
        };
        if(m_residentPreview && m_residentPreview->sourceId==next->sourceId){
            static_cast<ResidentPreviewAssets&>(*next)=*m_residentPreview;next->prepared=m_residentPreview->prepared;
        }else if(m_residentPreview && m_residentPreview->prepared && m_residentPreview->prepared->sourceId==next->sourceId){
            static_cast<ResidentPreviewAssets&>(*next)=*m_residentPreview->prepared;
            next->prepared=std::make_shared<ResidentPreviewAssets>(*m_residentPreview);
        }else {
            if(m_movePreparation && m_movePreparation->sourceId==next->sourceId) {
                static_cast<ResidentPreviewAssets&>(*next)=*m_movePreparation;
            }else{
                InteractionTrace::Scope assetsTrace("transform.worker.sourceAssets");
                QByteArray assets;if(!read(fullResolution?4:1,assets) || !decodeAssets(assets,*next,fullResolution?int(edge):2048))return;
                next->fullResolution=fullResolution;
            }
            if(info.focus&PAINT_RESIDENT_PREVIEW_SELECTED){
                QByteArray mask;if(!read(2,mask) || mask.size()<24)return;
                const auto w=qFromLittleEndian<quint32>(mask.constData()),h=qFromLittleEndian<quint32>(mask.constData()+4);
                if(!w || !h || w>2048 || h>2048 || quint64(mask.size())!=24+quint64(w)*h)return;
                for(int k=0;k<4;++k){const auto bits=qFromLittleEndian<quint32>(mask.constData()+8+k*4);std::memcpy(&next->identity[size_t(k)],&bits,4);if(!std::isfinite(next->identity[size_t(k)]))return;}
                if(next->identity[2]<=0 || next->identity[3]<=0)return;
                next->selection=QImage(int(w),int(h),QImage::Format_RGBA8888);if(next->selection.isNull())return;
                for(quint32 y=0;y<h;++y){auto *row=next->selection.scanLine(int(y));for(quint32 x=0;x<w;++x){const auto value=quint8(mask[qsizetype(24+quint64(y)*w+x)]);row[x*4]=value;row[x*4+1]=value;row[x*4+2]=value;row[x*4+3]=255;}}
            }
            if(next->forward && fullResolution && alternateId){
                auto prepared=std::make_shared<ResidentPreviewAssets>();prepared->sourceId=alternateId;
                prepared->transformIdentity=alternateId==identityId;prepared->fullResolution=true;prepared->meshPreview=true;
                QByteArray packet;if(!read(4,packet,nullptr,alternateId) || !decodeAssets(packet,*prepared,int(edge)))return;
                next->prepared=prepared;
            }else if(next->forward && !fullResolution){
                QByteArray packet;PaintStatus result=PAINT_OK;
                if(read(3,packet,&result)){
                    if(packet.size()<32)return;
                    auto prepared=std::make_shared<ResidentPreviewAssets>();prepared->sourceId=qFromLittleEndian<quint64>(packet.constData());
                    if(!prepared->sourceId || prepared->sourceId==next->sourceId || !decodeAssets(packet.sliced(8),*prepared))return;
                    prepared->transformIdentity=prepared->sourceId==identityId;
                    next->prepared=prepared;
                }else if(result!=PAINT_NOT_FOUND)return;
            }
        }
        if(quint64(mesh.size())!=count*16)return;
        next->vertices.resize(qsizetype(count));
        for(qsizetype i=0;i<qsizetype(count);++i)for(int k=0;k<4;++k){
            const auto bits=qFromLittleEndian<quint32>(mesh.constData()+i*16+k*4);
            float value=0;std::memcpy(&value,&bits,4);if(!std::isfinite(value))return;
            next->vertices[i][size_t(k)]=value;
        }
        m_residentPreview=next;m_connection->residentPreviewPending.store(true);emit residentPreviewReady(next);
    }
    // One background preview at a time, only after a quiet document interval.
    // Slot 2 is bounded by the core and is cancelled whenever painting resumes.
    void disablePreview(const PaintSessionInfo &info) {
        auto view=dto<PaintViewport>(); view.view_id=2; view.document_generation=info.document_generation;
        uint64_t ignored=0; paint_session_set_viewport(m_core,m_session,&view,&ignored); m_previewRequest=0;
    }
    void pollPreviews(const PaintSessionInfo &info) {
        const auto now=m_previewClock.elapsed();
        if(info.document_generation!=m_seenGeneration || info.revision!=m_seenRevision || info.stroke_active || (info.flags&PAINT_SESSION_LAYER_MOVE)) {
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
        InteractionTrace::Scope trace("worker.pollBrushState");
        PaintBrushLibrary *library=nullptr;quint64 preparation=0;QByteArray wanted,submitted;
        {QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;preparation=m_catalog->preparation;wanted=m_catalog->preferences;submitted=m_catalog->source;}
        if(!library || !preparation){m_pendingBrushPreferences.clear();return;}
        if(m_pendingBrushPreferences.isEmpty() && preparation==m_reportedBrushPreparation)return;
        QByteArray canonical(2*1024*1024,'\0');uint64_t required=0;
        const auto status=paint_brush_library_prepared_state(library,preparation,reinterpret_cast<uint8_t*>(canonical.data()),quint64(canonical.size()),&required);
        if(status==PAINT_BUSY)return;
        QString error;
        if(status!=PAINT_OK){error=immediateError(status);if(paint_brush_library_current_state(library,reinterpret_cast<uint8_t*>(canonical.data()),quint64(canonical.size()),&required)!=PAINT_OK)canonical.clear();}
        if(!canonical.isEmpty())canonical.resize(qsizetype(required));
        if(preparation!=m_reportedBrushPreparation){m_reportedBrushPreparation=preparation;
            // Identity/default normalization must not rebuild the entire live
            // library. Compare this exact prepared input on the worker first.
            if(!error.isEmpty() || !BrushLibrary::equivalentCatalogState(canonical,submitted))emit nativeBrushCatalog(preparation,canonical,error);
            InteractionTrace::point("catalog.complete.preparation",double(preparation));
            emit brushPreparationFinished(preparation,error.isEmpty());
        }
        if(!m_pendingBrushPreferences.isEmpty()){
            const auto pending=std::exchange(m_pendingBrushPreferences,{});
            if(pending!=wanted || !error.isEmpty())return;
            auto root=QJsonDocument::fromJson(pending).object();const auto native=QJsonDocument::fromJson(canonical).object();root["presets"]=native.value("presets");root["selected"]=native.value("selected");root["version"]=native.value("version");if(native.contains("organization"))root["organization"]=native.value("organization");else root.remove("organization");
            auto settings=preferences();settings->setValue("brushes/state",QJsonDocument(root).toJson(QJsonDocument::Compact));settings->sync();if(settings->status()!=QSettings::NoError)emit failure(QStringLiteral("无法保存画笔设置，请检查配置目录权限"));
        }
    }
    void pollBrushPreview() {
        if(m_brushJob.isEmpty()) {
            if(m_brushQueue.isEmpty())return;
            m_brushJob=m_brushQueue.takeFirst().toMap();m_brushRequest=0;m_brushClock.restart();
            const auto tip=m_brushJob.value("tip").toMap();const auto dual=tip.value("dualBrush").toMap();const bool dualActive=dual.value("enabled").toBool();const bool wetActive=tip.value("wetMix").toMap().value("enabled").toBool();
            const double primary=std::clamp(tip.value("presetRadius",24.).toDouble(),.5,1000.);const auto sizing=dual.value("sizing").toMap();const double secondary=sizing.value("mode").toString()=="relative"?std::clamp((sizing.value("model").toString()=="relative-ratio-v1"?primary*sizing.value("diameterRatio").toDouble():primary+sizing.value("diameterOffset").toDouble()/2.),.5,1000.):dual.value("diameter",24.).toDouble()/2.;
            m_brushPreviewScale=dualActive?std::max(1.,std::max(primary,secondary)/8.4):1.;
            const double mainDiameter=primary*2.*(tip.value("maskResource").toString().isEmpty()?1.:std::sqrt(2.))+2.;
            const double childDiameter=secondary*2.*(dual.value("resource").toString().isEmpty()?1.:std::sqrt(2.))+2.;
            const double mainCount=tip.value("scatterDynamics").toMap().value("enabled").toBool()?tip.value("scatterDynamics").toMap().value("count",1).toDouble():1.;
            const double childCount=dual.value("scatterDynamics").toMap().value("enabled").toBool()?dual.value("scatterDynamics").toMap().value("count",1).toDouble():1.;
            const double childCost=childCount*(dual.value("resource").toString().isEmpty()?1.:8.);
            const bool ownPath=dual.value("model").toString()=="path-coverage-v1";
            const double density=mainDiameter*mainDiameter*mainCount*(2.+(!tip.value("texture").toMap().isEmpty()?8.:0.)+(ownPath?0.:childCost))/std::max(.01,primary*m_brushJob.value("spacing").toDouble())
                +(ownPath?childDiameter*childDiameter*childCost/std::max(.01,secondary*2.*dual.value("spacing",.25).toDouble()):0.);
            const int sampleCount=dualActive?std::clamp(int(std::ceil(200.*m_brushPreviewScale*density/4000000.)),32,256):32;
            if(!m_brushSession){auto desc=dto<PaintDocumentDesc>();desc.width=240;desc.height=60;desc.working_space=PAINT_WORKING_LINEAR_SRGB;desc.pixel_format=PAINT_STORAGE_RGBA32F_PREMULTIPLIED;if(paint_session_create(m_core,&desc,&m_brushSession)!=PAINT_OK){finishBrushPreview({});return;}}
            auto command=dto<PaintCommand>();command.kind=PAINT_COMMAND_NEW_DOCUMENT;command.width=uint32_t(std::ceil(240*m_brushPreviewScale));command.height=uint32_t(std::ceil(60*m_brushPreviewScale));
            if(paint_session_submit(m_core,m_brushSession,&command,&m_brushSequence)!=PAINT_OK){finishBrushPreview({});return;}
            if(wetActive){
                if(!m_previewWet && paint_wet_tool_create(&m_previewWet)!=PAINT_OK){finishBrushPreview({});return;}
                if(paint_wet_tool_clean(m_previewWet)!=PAINT_OK || paint_session_bind_wet_tool(m_core,m_brushSession,m_previewWet,&m_brushSequence)!=PAINT_OK){finishBrushPreview({});return;}
                for(int half=0;half<2;++half){auto seed=dto<PaintCommand>();seed.kind=PAINT_COMMAND_BEGIN_STROKE;seed.stroke=dto<PaintStrokeDesc>();seed.stroke.mode=PAINT_MODE_PAINT;seed.stroke.radius=float(std::min(256.,16.*m_brushPreviewScale));seed.stroke.opacity=1;seed.stroke.spacing=.5;seed.stroke.linear_rgba[half==0?0:2]=1;seed.stroke.linear_rgba[3]=1;seed.point=dto<PaintPoint>();seed.point.tool=PAINT_TOOL_MOUSE;seed.point.pressure=1;seed.point.x=(half==0?20.:125.)*m_brushPreviewScale;seed.point.y=30.*m_brushPreviewScale;
                    if(paint_session_submit(m_core,m_brushSession,&seed,&m_brushSequence)!=PAINT_OK){finishBrushPreview({});return;}
                    seed.kind=PAINT_COMMAND_STROKE_TO;seed.point.x=(half==0?115.:220.)*m_brushPreviewScale;if(paint_session_submit(m_core,m_brushSession,&seed,&m_brushSequence)!=PAINT_OK){finishBrushPreview({});return;}
                    seed=dto<PaintCommand>();seed.kind=PAINT_COMMAND_END_STROKE;if(paint_session_submit(m_core,m_brushSession,&seed,&m_brushSequence)!=PAINT_OK){finishBrushPreview({});return;}
                }
            }
            command=dto<PaintCommand>();command.kind=PAINT_COMMAND_BEGIN_STROKE;command.stroke=dto<PaintStrokeDesc>();command.stroke.mode=PAINT_MODE_PAINT;
            command.stroke.radius=static_cast<float>(std::clamp(m_brushJob.value("radius").toDouble()*.35,2.,18.));command.stroke.opacity=m_brushJob.value("opacity").toFloat();command.stroke.spacing=m_brushJob.value("spacing").toFloat();
            if(dualActive)command.stroke.radius=static_cast<float>(primary);
            for(auto &v:command.stroke.linear_rgba)v=1;
            if(!m_previewCatalog){
                if(paint_brush_library_create(0,&m_previewCatalog)!=PAINT_OK){finishBrushPreview({});return;}
                PaintBrushLibrary *library=nullptr;{QMutexLocker lock(&m_catalog->mutex);library=m_catalog->library;}
                if(paint_brush_library_share_resources(m_previewCatalog,library)!=PAINT_OK){paint_brush_library_destroy(&m_previewCatalog);finishBrushPreview({});return;}
            }
            const QByteArray presetId="round-pressure";
            BrushLibrary draft;auto state=QJsonDocument::fromJson(draft.catalogSnapshot()).object();auto rows=state.value("presets").toArray();auto row=rows[0].toObject();
            row["radius"]=dualActive?primary:24.;row["opacity"]=command.stroke.opacity;row["spacing"]=command.stroke.spacing;
            for(const auto &key:{"hardness","roundness","angle","flow","pressureSize","pressureFlow"})row[key]=QJsonValue::fromVariant(tip.value(key));
            if(!tip.value("maskResource").toString().isEmpty()){row["maskResource"]=tip.value("maskResource").toString();raiseBrushStateVersion(state,3);}
            if(!tip.value("pressureDynamics").toMap().isEmpty()){row["pressureDynamics"]=QJsonObject::fromVariantMap(tip.value("pressureDynamics").toMap());raiseBrushStateVersion(state,tip.value("pressureDynamics").toMap().contains("opacity")?6:5);}
            if(!tip.value("tipDynamics").toMap().isEmpty()){row["tipDynamics"]=QJsonObject::fromVariantMap(tip.value("tipDynamics").toMap());raiseBrushStateVersion(state,7);}
            const auto scatter=tip.value("scatterDynamics").toMap();if(!scatter.isEmpty()){row["scatterDynamics"]=QJsonObject::fromVariantMap(scatter);raiseBrushStateVersion(state,(scatter.contains("amountControl")||scatter.contains("countControl"))?35:8);}
            if(!tip.value("shapeJitter").toMap().isEmpty()){row["shapeJitter"]=QJsonObject::fromVariantMap(tip.value("shapeJitter").toMap());raiseBrushStateVersion(state,9);}
            if(!tip.value("transferJitter").toMap().isEmpty()){row["transferJitter"]=QJsonObject::fromVariantMap(tip.value("transferJitter").toMap());raiseBrushStateVersion(state,37);}
            if(!tip.value("colorDynamics").toMap().isEmpty()){row["colorDynamics"]=QJsonObject::fromVariantMap(tip.value("colorDynamics").toMap());raiseBrushStateVersion(state,10);}
            const auto texture=tip.value("texture").toMap();if(!texture.isEmpty()){row["texture"]=QJsonObject::fromVariantMap(texture);raiseBrushStateVersion(state,BrushLibrary::textureStateVersion(texture));}
            if(!tip.value("dualBrush").toMap().isEmpty()){row["dualBrush"]=QJsonObject::fromVariantMap(tip.value("dualBrush").toMap());raiseBrushStateVersion(state,tip.value("dualBrush").toMap().contains("texture")?44:12);if(tip.value("dualBrush").toMap().value("scatterDynamics").toMap().contains("amountControl") || tip.value("dualBrush").toMap().value("scatterDynamics").toMap().contains("countControl"))raiseBrushStateVersion(state,35);const QStringList extended={"colorBurn","linearBurn","colorDodge","linearDodge","overlay","softLight","hardLight","exclusion","hardMix","vividLight","linearLight","pinLight","divide","linearHeight"};if(extended.contains(tip.value("dualBrush").toMap().value("mode").toString()))raiseBrushStateVersion(state,36);}
            const auto smoothPressure=[](const QVariantMap &data){for(const auto &target:{"size","flow","opacity"})if(data.value(target).toMap().value("interpolation").toString()=="smooth-pchip-v1")return true;return false;};
            if(smoothPressure(tip.value("pressureDynamics").toMap()) || smoothPressure(tip.value("dualBrush").toMap().value("pressureDynamics").toMap()))raiseBrushStateVersion(state,13);
            if(!tip.value("smoothing").toMap().isEmpty()){row["smoothing"]=QJsonObject::fromVariantMap(tip.value("smoothing").toMap());raiseBrushStateVersion(state,14);}
            if(tip.value("dualBrush").toMap().contains("sizing"))raiseBrushStateVersion(state,tip.value("dualBrush").toMap().value("sizing").toMap().value("model").toString()=="relative-ratio-v1"?38:15);
            if(tip.value("dualBrush").toMap().contains("color"))raiseBrushStateVersion(state,16);
            if(!tip.value("wetMix").toMap().isEmpty()){row["wetMix"]=QJsonObject::fromVariantMap(tip.value("wetMix").toMap());raiseBrushStateVersion(state,17);}
            if(!tip.value("strokeTaper").toMap().isEmpty()){row["strokeTaper"]=QJsonObject::fromVariantMap(tip.value("strokeTaper").toMap());const auto taper=tip.value("strokeTaper").toMap();raiseBrushStateVersion(state,taper.value("start").toMap().contains("lengthMode") || taper.value("end").toMap().contains("lengthMode")?39:18);}
            if(!tip.value("strokeOutline").toMap().isEmpty()){const auto outline=tip.value("strokeOutline").toMap();row["strokeOutline"]=QJsonObject::fromVariantMap(outline);raiseBrushStateVersion(state,BrushLibrary::outlineStateVersion(outline));}
            if(BrushCurveModes::present(QJsonObject::fromVariantMap(tip)))raiseBrushStateVersion(state,40);
            if(tip.value("wetMix").toMap().contains("dualColor"))raiseBrushStateVersion(state,19);
            if(tip.value("dualBrush").toMap().value("color").toMap().value("model").toString()=="interleaved-stamp-v1")state["version"]=std::max(state.value("version").toInt(),22);
            const auto air=tip.value("airbrush").toMap();if(air.value("enabled").toBool() || (air.contains("intervalMs") && std::abs(air.value("intervalMs").toDouble()-50.)>1e-9))state["version"]=std::max(state.value("version").toInt(),23);
            if(tip.value("speedDynamics").toMap().contains("enabled"))state["version"]=std::max(state.value("version").toInt(),24);
            if(tip.value("tangentDynamics").toMap().contains("enabled"))state["version"]=std::max(state.value("version").toInt(),25);
            if(tip.value("distanceDynamics").toMap().contains("enabled"))state["version"]=std::max(state.value("version").toInt(),26);
            if(tip.value("tiltDynamics").toMap().contains("enabled"))state["version"]=std::max(state.value("version").toInt(),27);
            if(tip.value("rotationDynamics").toMap().contains("enabled"))state["version"]=std::max(state.value("version").toInt(),28);
            if(tip.value("directionDynamics").toMap().contains("enabled"))state["version"]=std::max(state.value("version").toInt(),29);
            if(!tip.value("sourceMatrix").toMap().isEmpty()){row["sourceMatrix"]=QJsonObject::fromVariantMap(tip.value("sourceMatrix").toMap());state["version"]=std::max(state.value("version").toInt(),32);}
            if(!tip.value("fadeDynamics").toMap().isEmpty()){row["fadeDynamics"]=QJsonObject::fromVariantMap(tip.value("fadeDynamics").toMap());state["version"]=std::max(state.value("version").toInt(),34);}
            if(QSet<QString>{"linearBurn","colorDodge","linearDodge","overlay","softLight","hardLight","exclusion","hardMix"}.contains(tip.value("texture").toMap().value("mode").toString()))state["version"]=std::max(state.value("version").toInt(),30);
            if(QSet<QString>{"vividLight","linearLight","pinLight","divide","linearHeight","height"}.contains(tip.value("texture").toMap().value("mode").toString()))state["version"]=std::max(state.value("version").toInt(),31);
            if(tip.value("texture").toMap().value("mode").toString()=="dissolve")state["version"]=std::max(state.value("version").toInt(),33);
            if(scatter.contains("amountControl") || scatter.contains("countControl") || tip.value("dualBrush").toMap().value("scatterDynamics").toMap().contains("amountControl") || tip.value("dualBrush").toMap().value("scatterDynamics").toMap().contains("countControl"))raiseBrushStateVersion(state,35);
            if(QStringList{"colorBurn","linearBurn","colorDodge","linearDodge","overlay","softLight","hardLight","exclusion","hardMix","vividLight","linearLight","pinLight","divide","linearHeight"}.contains(tip.value("dualBrush").toMap().value("mode").toString()))raiseBrushStateVersion(state,36);
            rows[0]=row;state["presets"]=rows;const auto json=QJsonDocument(state).toJson(QJsonDocument::Compact);uint64_t preparation=0;
            if(paint_brush_library_prepare_state_outline(m_previewCatalog,reinterpret_cast<const uint8_t*>(json.constData()),quint64(json.size()),&preparation)!=PAINT_OK){finishBrushPreview({});return;}
            auto request=dto<PaintBrushToolDesc>();request.radius=command.stroke.radius;request.strength=1;for(auto &v:request.linear_rgba)v=1;
            auto background=dto<PaintBrushBackgroundDesc>();for(auto &v:background.linear_rgba)v=1;
            if(tip.value("colorDynamics").toMap().value("enabled").toBool()){request.linear_rgba[0]=1;request.linear_rgba[1]=request.linear_rgba[2]=0;background.linear_rgba[0]=background.linear_rgba[1]=0;background.linear_rgba[2]=1;}
            auto blend=dto<PaintBrushBlendDesc>();blend.mode=0;
            for(int i=0;i<=sampleCount;++i){const double t=i/double(sampleCount);InputSample sample;sample.position={(20+200*t)*m_brushPreviewScale,(30+8*std::sin(t*6.283185307))*m_brushPreviewScale};sample.pressure=static_cast<float>(.25+.75*std::sin(t*3.141592654));sample.tool=PAINT_TOOL_PEN;if(tip.value("smoothing").toMap().value("enabled").toBool())sample.timestamp=quint64(i)*8000000; if(!tip.value("tipDynamics").toMap().isEmpty() || !dual.value("tipDynamics").toMap().isEmpty() || tip.value("tiltDynamics").toMap().value("enabled").toBool() || tip.value("rotationDynamics").toMap().value("enabled").toBool()){sample.capabilities=PAINT_INPUT_PRESSURE|PAINT_INPUT_TILT|PAINT_INPUT_ROTATION;sample.rotation=static_cast<float>(-60+120*t);sample.tiltX=static_cast<float>(30+30*std::sin(t*3.141592654));sample.tiltY=20;}command.point=point(sample);command.kind=PAINT_COMMAND_STROKE_TO;
                const auto status=i==0?paint_session_begin_prepared_preset_with_blend(m_core,m_brushSession,m_previewCatalog,preparation,reinterpret_cast<const uint8_t*>(presetId.constData()),quint64(presetId.size()),&request,&background,&blend,&command.point,&m_brushSequence):paint_session_submit(m_core,m_brushSession,&command,&m_brushSequence);
                if(status!=PAINT_OK){finishBrushPreview({});return;}}
            command=dto<PaintCommand>();command.kind=PAINT_COMMAND_END_STROKE;if(paint_session_submit(m_core,m_brushSession,&command,&m_brushSequence)!=PAINT_OK){finishBrushPreview({});return;}
            return;
        }
        if(m_brushClock.elapsed()>3000){finishBrushPreview({});return;}
        auto info=dto<PaintSessionInfo>();if(paint_session_info(m_core,m_brushSession,&info)!=PAINT_OK){finishBrushPreview({});return;}
        if(info.completed_sequence<m_brushSequence)return;
        if(info.last_error_status!=PAINT_OK){finishBrushPreview({});return;}
        if(!m_brushRequest){auto view=dto<PaintViewport>();view.view_id=0;view.enabled=1;view.width=240*m_brushPreviewScale;view.height=60*m_brushPreviewScale;view.pixel_width=240;view.pixel_height=60;view.document_generation=info.document_generation;if(paint_session_set_viewport(m_core,m_brushSession,&view,&m_brushRequest)!=PAINT_OK)finishBrushPreview({});return;}
        auto frame=dto<PaintFrameInfo>();const auto status=paint_session_frame_info(m_core,m_brushSession,0,&frame);if(status==PAINT_BUSY)return;
        if(status!=PAINT_OK){finishBrushPreview({});return;}
        if(frame.request_id!=m_brushRequest || frame.document_generation!=info.document_generation || frame.revision!=info.revision)return;
        QImage image(240,60,QImage::Format_RGBA8888_Premultiplied);auto tile=dto<PaintTile>();tile.format=PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED;tile.data=image.bits();tile.capacity=image.sizeInBytes();tile.stride=image.bytesPerLine();
        if(paint_session_read_frame(m_core,m_brushSession,0,frame.request_id,frame.frame_id,&tile)!=PAINT_OK){finishBrushPreview({});return;}
        // Display every brush in white while preserving the engine's exact alpha.
        // RGBA8888 is byte ordered and premultiplied: white RGB equals alpha here.
        for(int y=0;y<image.height();++y){auto *row=image.scanLine(y);for(int x=0;x<image.width();++x){auto *pixel=row+x*4;pixel[0]=pixel[1]=pixel[2]=pixel[3];}}
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
    PaintWetTool *m_previewWet=nullptr;
    QVariantMap m_pendingWet;
    QElapsedTimer m_wetClock;
    std::shared_ptr<BrushCatalogConnection> m_catalog;
    bool m_catalogLease=false;
    QByteArray m_pendingBrushPreferences;
    quint64 m_reportedBrushPreparation=0;
    QStringList m_pendingOutlines,m_pendingCursorImages;
    QVariantList m_brushQueue;
    QVariantMap m_brushJob;
    double m_brushPreviewScale=1.;
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
    QVariantList m_computeHardware;
    QString m_computeDevice;
    QElapsedTimer m_computeClock;
    quint64 m_computeSequence=0;
    std::shared_ptr<BackendConnection> m_connection;
    QVariantMap m_lastTaskProgress;
    QTimer *m_timer;
    PaintCore *m_core = nullptr;
    PaintSession *m_session = nullptr;
    QTimer *m_compositionTimer=nullptr;
    CompositionReader m_compositionReader;
    quint64 m_compositionAckWatch=0;
    QElapsedTimer m_transformInputClock;
    QString m_compositionFallback;
    quint64 m_layerCopySequence=0;
    ResidentPreviewPtr m_residentPreview;
    ResidentPreviewAssetsPtr m_movePreparation;
    quint64 m_movePreparationPublishedRequest=0,m_movePreparationPublishedSource=0;
    quint64 m_publication = 0, m_completed = 0, m_errorSequence = 0, m_frames[2]{};
    quint64 m_strokeCommitCursor=0;
    quint64 m_deliveredFile = 0;
    uint32_t m_flags = 0;
    PaintStatus m_errorStatus = PAINT_OK;
    bool m_fullResolutionResidentSupported=false;
    bool m_stopped = false;
};

PaintCoreClient::PaintCoreClient(QObject *parent, const QString &settingsFile,std::shared_ptr<BrushCatalogConnection> brushCatalog) : QObject(parent), m_settingsFile(settingsFile), m_connection(std::make_shared<BackendConnection>()),m_brushCatalogConnection(brushCatalog?std::move(brushCatalog):std::make_shared<BrushCatalogConnection>()) {
    {auto settings=m_settingsFile.isEmpty()?std::make_unique<QSettings>(QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse"):std::make_unique<QSettings>(m_settingsFile,QSettings::IniFormat);m_longTaskDelay=std::clamp(settings->value("performance/taskProgressDelayMs",1000).toInt(),0,10000);}
    {
        const auto values=renderSettings();
        CompositionBudget::memoryBytes=quint64(values["memoryMiB"].toInt())*1024*1024;
        auto settings=m_settingsFile.isEmpty()?std::make_unique<QSettings>(QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse"):std::make_unique<QSettings>(m_settingsFile,QSettings::IniFormat);
        CompositionBudget::cacheBytes=quint64(std::clamp(settings->value("storage/memoryMiB",defaultMemoryMiB()).toInt(),64,1048576))*1024*1024;
        m_highestCanvasQuality=values["highestQuality"].toBool();
        const auto gpu=values["gpuMiB"].toInt();
        CompositionBudget::automaticGpu=gpu==0;
        CompositionBudget::gpuBytes=gpu?quint64(gpu)*1024*1024:CompositionBudget::memoryBytes.load();
    }
    connect(QGuiApplication::clipboard(),&QClipboard::dataChanged,this,&PaintCoreClient::stateChanged);
    // Focus changes can occur inside a QML enabled binding. Notify only the
    // shortcuts that depend on focus, without reentering document/busy bindings.
    connect(qGuiApp,&QGuiApplication::focusObjectChanged,this,[this]{emit layerShortcutsChanged();});
    connect(this,&PaintCoreClient::stateChanged,this,&PaintCoreClient::layerShortcutsChanged);
    m_brushLibrary=new BrushLibrary(this);
    m_layerMoveTimer=new QTimer(this);m_layerMoveTimer->setInterval(16);
    connect(m_layerMoveTimer,&QTimer::timeout,this,&PaintCoreClient::flushLayerMove);
    m_airbrushTimer=new QTimer(this);connect(m_airbrushTimer,&QTimer::timeout,this,&PaintCoreClient::airbrushTick);
    connect(m_brushLibrary,&BrushLibrary::activeColorsChanged,this,[this]{m_color=m_brushLibrary->foregroundColor();m_secondaryColor=m_brushLibrary->backgroundColor();emit brushChanged();});
    qRegisterMetaType<PaintSessionInfo>();
    qRegisterMetaType<PaintFileJobInfo>();
    m_worker = new BackendWorker(m_connection, settingsFile,m_brushCatalogConnection); m_worker->moveToThread(&m_thread);
    connect(m_worker,&BackendWorker::liquifyPreferences,this,[this](const QVariantMap &values){
        m_liquifyTool=std::clamp(values.value("tool").toInt(),0,3);m_liquifySize=std::clamp(values.value("size").toInt(),1,15000);m_liquifyStrength=std::clamp(values.value("strength").toInt(),1,100);m_liquifyDensity=std::clamp(values.value("density").toInt(),1,100);m_liquifySpeed=std::clamp(values.value("speed").toInt(),1,100);m_liquifyPressure=values.value("pressure").toBool();emit liquifyOptionsChanged();
    });
    connect(m_worker,&BackendWorker::layerClipboardReady,this,[this](const QByteArray &bytes){
        if(m_closing)return;
        m_pendingCopy=0;
        if(!bytes.isEmpty()){auto *mime=new QMimeData;mime->setData("application/x-drawverse-layers-v1",bytes);QGuiApplication::clipboard()->setMimeData(mime);}
        emit stateChanged();
    });
    connect(m_worker,&BackendWorker::brushPreferences,m_brushLibrary,[this](const QByteArray &json,const BrushLibrary::PreparedPreferencesPtr &prepared){
        if(m_closing)return;
        QByteArray source=json;{QMutexLocker lock(&m_brushCatalogConnection->mutex);if(!m_brushCatalogConnection->preferences.isEmpty())source=m_brushCatalogConnection->preferences;}
        if(source!=json) {
            QMetaObject::invokeMethod(m_worker,[worker=m_worker,source]{worker->prepareBrushPreferences(source);},Qt::QueuedConnection);
            return;
        }
        if(!m_brushLibrary->applyPreparedPreferences(prepared))showError(QStringLiteral("无法恢复笔刷设置，原配置已保留"));
        m_brushLibrary->setToolContext(m_smudgeTool?2:m_eraser?1:0);
        synchronizeBrushCatalog(m_brushLibrary->snapshot());
        if(!m_brushLibraryReady){m_brushLibraryReady=true;emit brushLibraryReadyChanged();}
        if(!m_pendingBrushImport.isEmpty()){const auto path=m_pendingBrushImport;m_pendingBrushImport.clear();m_brushFileBusy=false;emit brushFileChanged();QMetaObject::invokeMethod(this,[this,path]{importBrushes(QUrl::fromLocalFile(path));},Qt::QueuedConnection);}
    });
    connect(m_worker,&BackendWorker::brushPreparationFinished,this,[this](quint64 preparation,bool success){
        if(m_closing || m_brushCatalogPrepared || !success)return;
        {QMutexLocker lock(&m_brushCatalogConnection->mutex);if(preparation!=m_brushCatalogConnection->preparation)return;}
        m_brushCatalogPrepared=true;emit brushCatalogPreparedChanged();
    });
    connect(m_worker,&BackendWorker::nativeBrushCatalog,this,[this](quint64 preparation,const QByteArray &state,const QString &error){
        {QMutexLocker lock(&m_brushCatalogConnection->mutex);if(preparation!=m_brushCatalogConnection->preparation)return;}
        if(m_brushLibrary->persistencePending())return;
        if(!state.isEmpty() && m_brushLibrary->applyNativeCatalog(state)){QMutexLocker lock(&m_brushCatalogConnection->mutex);if(preparation==m_brushCatalogConnection->preparation)m_brushCatalogConnection->source=error.isEmpty()?m_brushLibrary->catalogSnapshot():QByteArray{};}
        if(!error.isEmpty())showError(error);
    });
    connect(m_worker,&BackendWorker::brushPreviewReady,m_brushLibrary,&BrushLibrary::acceptPreview);
    connect(m_worker,&BackendWorker::brushResourcesReady,this,[this](const QByteArray &json,const QString &error){
        m_brushResourcesBusy=false;
        if(error.isEmpty()){
            const auto data=QJsonDocument::fromJson(json).object();QStringList ids;
            if(data.value("model").toString()=="coverage-resources-v1")for(const auto &id:data.value("resources").toArray())ids.append(id.toString());
            m_brushResources=ids;
        }else if(!m_closing)showError(error);
        emit brushResourcesChanged();
    });
    connect(m_worker,&BackendWorker::brushResourcePreviewReady,this,[this](const QString &identity,const QString &image){
        m_pendingBrushResourcePreviews.remove(identity);
        m_brushResourcePreviewOrder.removeAll(identity);m_brushResourcePreviewOrder.append(identity);
        m_brushResourcePreviews.insert(identity,image);
        while(m_brushResourcePreviewOrder.size()>64)m_brushResourcePreviews.remove(m_brushResourcePreviewOrder.takeFirst());
        ++m_brushResourcePreviewRevision;emit brushResourcePreviewsChanged();
    });
    connect(m_worker,&BackendWorker::brushCursorImageReady,this,[this](const QString &identity,const QString &image,const QString &error){
        if(m_closing || !m_cursorImageDemand.contains(identity))return;
        if(error.isEmpty())m_cursorImages.insert(identity,image);else m_cursorImageErrors.insert(identity,QStringLiteral("笔尖图片暂不可用：")+error);
        emit brushCursorImageChanged();
    });
    connect(this,&PaintCoreClient::brushChanged,this,&PaintCoreClient::requestBrushCursorImages);
    connect(m_worker,&BackendWorker::brushOutlineReady,this,[this](const QString &identity,const QString &path,const QString &error){
        if(m_drawing && identity==m_strokeOutlineResource){m_strokeOutlinePath=path;m_strokeOutlineReady=error.isEmpty();emit brushOutlineChanged();}
        bool updated=false;
        if(m_drawing && identity==m_strokeSecondOutlineResource){m_strokeSecondOutlinePath=path;m_strokeSecondOutlineReady=error.isEmpty();updated=true;}
        if(identity==m_outlineResource && identity==m_brushLibrary->maskResource()){m_brushOutlinePath=path;m_brushOutlineReady=error.isEmpty();m_brushOutlineMessage=error.isEmpty()?QString{}:QStringLiteral("图片轮廓暂不可用，显示简化轮廓：")+error;updated=true;}
        if(identity==m_secondOutlineResource){m_secondOutlinePath=path;m_secondOutlineReady=error.isEmpty();m_secondOutlineMessage=error.isEmpty()?QString{}:QStringLiteral("第二图片轮廓暂不可用，显示简化轮廓：")+error;updated=true;}
        if(updated)emit brushOutlineChanged();
    });
    connect(m_worker,&BackendWorker::wetActionReady,this,[this](bool success,const QString &error){m_wetActionBusy=false;emit wetActionChanged();if(!error.isEmpty() && !m_closing)showError(error);emit wetBrushFinished(success);});
    connect(m_worker,&BackendWorker::brushSourceReady,this,[this](const QString &identity,quint64 token,const QByteArray &record,const QString &error){
        if(m_closing || token!=m_brushSourceToken || identity!=m_brushLibrary->sourceResource())return;
        m_brushSourceBusy=false;m_brushSourceInfo=error.isEmpty()?QJsonDocument::fromJson(record).object().toVariantMap():QVariantMap{};
        m_brushSourceMessage=error;emit brushSourceChanged();
    });
    connect(m_brushLibrary,&BrushLibrary::settingsChanged,this,[this]{
        const auto identity=m_brushLibrary->sourceResource();if(identity==m_brushSourceIdentity)return;
        m_brushSourceIdentity=identity;++m_brushSourceToken;m_brushSourceInfo.clear();m_brushSourceMessage.clear();m_brushSourceBusy=false;emit brushSourceChanged();
    });
    connect(m_worker,&BackendWorker::brushTipReady,this,[this](const QString &target,const QString &resource,int depth,const QString &error,int role){
        bool success=error.isEmpty();
        if(success && !((role==3 || role==4)?m_brushLibrary->setOutlineTextureResource(target,resource,role==4):role==2?m_brushLibrary->setDualResource(target,resource):role==1?m_brushLibrary->setTextureResource(target,resource):m_brushLibrary->setMaskResource(target,resource))){success=false;showError(QStringLiteral("目标笔刷已删除，未应用图片资源"));}
        if(!error.isEmpty())showError(error);
        m_brushImportMessage=success?(depth==16?QStringLiteral("已导入：16 位覆盖已转换为 8 位"):role==2?QStringLiteral("第二笔尖已导入"):(role==3 || role==4)?QStringLiteral("描边纹理已导入"):role==1?QStringLiteral("纹理已导入"):QStringLiteral("图片笔尖已导入")):QString{};
        m_brushFileBusy=false;emit brushFileChanged();emit brushFileFinished(success);
        if(success)refreshBrushResources();
    });
    connect(m_brushLibrary,&BrushLibrary::cursorChanged,this,&PaintCoreClient::brushCursorPreferencesChanged);
    connect(m_brushLibrary,&BrushLibrary::selectionChanged,this,&PaintCoreClient::brushSelectionChanged);
    connect(m_brushLibrary,&BrushLibrary::cursorChanged,this,[this]{requestBrushOutlines();emit brushOutlineChanged();});
    connect(m_worker,&BackendWorker::brushFileReady,this,[this](const QByteArray &package,const QString &error,bool exporting,bool abr){
        bool success=error.isEmpty();
        if(!exporting){
            const auto groupName=m_brushImportGroupName;m_brushImportGroupName.clear();
            if(success && !(abr?m_brushLibrary->importAbr(package,groupName):m_brushLibrary->importPackage(package,groupName))){success=false;showError(QStringLiteral("导入失败：笔刷库容量不足或包参数无效（最多 1024 个预设）"));}
        }
        if(!error.isEmpty())showError(error);
        if(success && abr) {
            const auto rows=QJsonDocument::fromJson(package).object().value("presets").toArray();int partial=0;for(const auto &row:rows)if(row.toObject().value("partial").toBool())++partial;
            m_brushImportMessage=QStringLiteral("已导入 %1 个 ABR 笔刷（%2 个部分转换）；在画笔设置的“来源与兼容”查看报告").arg(rows.size()).arg(partial);
        }
        m_brushFileBusy=false;emit brushFileChanged();emit brushFileFinished(success);
        if(success && !exporting)refreshBrushResources();
    });
    connect(m_brushLibrary,&BrushLibrary::persistRequested,this,&PaintCoreClient::persistBrushPreferences);
    connect(m_brushLibrary,&BrushLibrary::previewRequested,m_worker,&BackendWorker::queueBrushPreview);
    connect(m_brushLibrary,&BrushLibrary::previewCancelled,m_worker,[worker=m_worker](const QString &id,quint64 token){worker->cancelBrushPreview(id,token);});
    connect(m_brushLibrary,&BrushLibrary::settingsChanged,this,[this]{if(m_hoverPose)paint_brush_pose_clear(m_hoverPose);m_hoverPosePreparation=0;m_poseExpectedSource.clear();m_radius=m_brushLibrary->radius();m_opacity=m_brushLibrary->opacity();m_spacing=m_brushLibrary->spacing();m_liveBasePose={false,m_radius,m_brushLibrary->angle(),m_brushLibrary->roundness()};emit brushChanged();});
    connect(m_brushLibrary,&BrushLibrary::settingsChanged,this,[this]{
        const auto identity=m_brushLibrary->maskResource();const auto second=m_brushLibrary->dualBrush().value("resource").toString();
        const bool changed=identity!=m_outlineResource || second!=m_secondOutlineResource;
        if(identity!=m_outlineResource){m_outlineResource=identity;m_brushOutlinePath.clear();m_brushOutlineMessage.clear();m_brushOutlineReady=false;}
        if(second!=m_secondOutlineResource){m_secondOutlineResource=second;m_secondOutlinePath.clear();m_secondOutlineMessage.clear();m_secondOutlineReady=false;}
        if(changed)emit brushOutlineChanged();requestBrushOutlines();
    });
    connect(m_brushLibrary,&BrushLibrary::recentColorsChanged,this,&PaintCoreClient::recentColorsChanged);
    connect(m_worker,&BackendWorker::taskProgress,this,[this](const QVariantMap &values){if(m_closing || m_computationProgress==values)return;m_computationProgress=values;emit computationProgressChanged();});
    connect(m_worker,&BackendWorker::storageState,this,[this](QVariantMap saved,QVariantMap active,QVariantMap info,QString message) {
        m_storageSettings=std::move(saved); m_activeStorageSettings=std::move(active);
        m_storageInfo=std::move(info); m_storageMessage=std::move(message); emit storageChanged();
    });
    connect(m_worker,&BackendWorker::storageDone,this,[this](bool success) {
        m_storageBusy=false; emit storageChanged(); emit storageFinished(success);
    });
    connect(m_worker,&BackendWorker::computeState,this,[this](QVariantList hardware,QString device,QString status,bool busy,bool highestQuality) {
        const bool qualityChanged=m_highestCanvasQuality!=highestQuality;
        if(m_computeHardware==hardware && m_computeDevice==device && m_computeStatus==status && m_computeBusy==busy && !qualityChanged)return;
        m_highestCanvasQuality=highestQuality;
        const bool deviceChanged=m_computeDevice!=device;
        m_computeHardware=std::move(hardware);m_computeDevice=std::move(device);m_computeStatus=std::move(status);m_computeBusy=busy;
        if(qualityChanged)emit stateChanged();
        if(deviceChanged){
            clearLayerMovePreparation();
            clearCompositionEdit();
            emit frameChanged();
            refreshCpuDisplayDemand();
            requestLayerMovePreparation();
        }
        emit computeChanged();
    });
    connect(&m_thread, &QThread::started, m_worker, &BackendWorker::boot);
    connect(m_worker, &BackendWorker::metadata, this, [this](PaintSessionInfo info, QVariantList layers, QVariantMap selection, QVariantList history,QVariantMap transform,QVariantList committedStrokes) {
        InteractionTrace::Scope entryTrace("move.gui.metadataEntry");
        if(entryTrace.active)entryTrace.event.value=double(info.completed_sequence);
        m_connection->metadataPending.store(false);
        m_adjustmentHistogram=selection.take("adjustmentHistogram").toList();
        const bool propertiesChanged = info.document_generation != m_generation
            || static_cast<int>(info.width) != m_width || static_cast<int>(info.height) != m_height
            || static_cast<int>(info.undo_depth) != m_undo || static_cast<int>(info.redo_depth) != m_redo
            || info.active_layer_id != m_active;
        const bool wasLiquify=m_liquifyActive,wasLiquifyBusy=liquifyBusy();
        const bool wasTransform=m_transformActive,wasTransformBusy=transformBusy();
        const bool controlsWereVisible=transformControlsVisible();
        const bool wasReady = m_ready, wasModified = modified(), wasLayerBusy=layerEditBusy();
        const bool changedLayers = m_layers != layers;
        const bool activeChanged = info.active_layer_id != m_active;
        const bool changedGeneration = info.document_generation != m_generation;
        if(changedGeneration){m_transformClosing=false;m_adjustmentBase.reset();m_adjustmentFrame.reset();m_adjustmentFinalRevision=0;clearCompositionEdit();m_residentPreview.reset();m_blendPreviewLayer=0;m_layerThumbnails.clear();emit layerThumbnailsChanged();}
        else if(m_blendPreviewLayer && (info.revision!=m_revision || info.active_layer_id!=m_active))clearLayerBlendPreview();
        if(changedGeneration) m_nextDefaultLayer=1;
        if(changedGeneration && !m_collapsedGroups.isEmpty()) {m_collapsedGroups.clear(); emit groupExpansionChanged();}
        if(info.completed_sequence>=m_pendingLiquify) {m_pendingLiquify=0;m_liquifyCommitting=false;m_liquifyActive=(info.flags&PAINT_SESSION_LIQUIFY)!=0;}
        const bool publishedTransform=(info.flags&PAINT_SESSION_TRANSFORM)!=0;
        if(publishedTransform && !m_transformPublished){
            m_transformInitialPoints=transform.value("points").toList();m_residentPreviewFloor=info.revision;
            m_transformInitialWarp=transform.value("warp").toBool();m_transformInitialInterpolation=transform.value("interpolation").toInt();
            m_transformInitialSkewH=transform.value("skewH").toDouble();m_transformInitialSkewV=transform.value("skewV").toDouble();
            // Placement opens inside the file worker: opt in after file-result delivery.
            if(residentGpuPreviewEnabled() && !m_compositionTransform && !m_compositionTransformCpu)QTimer::singleShot(0,this,[this]{if(m_transformActive && m_transformPublished && !m_compositionTransform && !m_compositionTransformCpu && m_pendingTransform<=m_completedSequence && residentGpuPreviewEnabled())submitTransform(PAINT_TRANSFORM_ENABLE_RESIDENT);});
            emit transformChanged();
        }
        m_transformPublished=publishedTransform;
        if(m_transformClosing && info.completed_sequence>=m_pendingTransform){m_transformClosing=false;}
        if(!publishedTransform && info.completed_sequence>=m_pendingTransform)m_compositionTransformCpu=false;
        if(!publishedTransform && m_transformActive && info.completed_sequence>=m_pendingTransform && m_afterTransform){auto action=std::move(m_afterTransform);QTimer::singleShot(0,this,std::move(action));}
        if(info.completed_sequence>=m_pendingTransform)m_transformActive=(info.flags&PAINT_SESSION_TRANSFORM)!=0;
        if(!m_transformActive && !m_residentPreview){m_transformMeshPreview=false;m_connection->transformMeshAssets.store(false);}
        if(controlsWereVisible!=transformControlsVisible())emit transformChanged();
        if(info.completed_sequence>=m_lastTransform){const auto points=transform.value("points").toList();const bool warp=transform.value("warp").toBool();const int interpolation=transform.value("interpolation").toInt();const auto skewH=transform.value("skewH").toDouble(),skewV=transform.value("skewV").toDouble();if(points!=m_transformPoints || warp!=m_transformWarp || interpolation!=m_transformInterpolation || skewH!=m_transformSkewH || skewV!=m_transformSkewV){m_transformPoints=points;m_transformWarp=warp;m_transformInterpolation=interpolation;m_transformSkewH=skewH;m_transformSkewV=skewV;emit transformChanged();}}
        if(m_compositionTransform && info.completed_sequence>=m_pendingTransform){
            if(publishedTransform){if(!m_compositionMoveConfirmed)m_compositionTargetNs=InteractionTrace::now();m_compositionMoveConfirmed=true;refreshCompositionTransform();}
            else {m_compositionFinalRevision=info.revision;}
        }
        InteractionTrace::Scope metadataTrace("move.gui.metadata");
        if(metadataTrace.active)metadataTrace.event.value=double(info.completed_sequence);
        InteractionTrace::point("move.metadata.revision",double(info.revision));
        InteractionTrace::point("move.metadata.generation",double(info.document_generation));
        m_completedSequence=info.completed_sequence;
        for(const auto &value:committedStrokes) {
            const auto found=m_pendingStrokeColors.constFind(value.toULongLong());
            if(found==m_pendingStrokeColors.cend())continue;
            if(found->isValid())m_brushLibrary->useColor(found->name(QColor::HexArgb));
        }
        while(!m_pendingStrokeColors.isEmpty() && m_pendingStrokeColors.firstKey()<=m_completedSequence) {
            m_pendingStrokeColors.erase(m_pendingStrokeColors.begin());
        }
        if(m_layerMoveActive && (changedGeneration || (m_completedSequence>=m_layerMoveSequence && !(info.flags&PAINT_SESSION_LAYER_MOVE)))) {
            if(!changedGeneration && m_layerMoveClosing && (info.last_error_status==PAINT_OK || info.last_error_sequence!=m_layerMoveSequence) && (m_layerMoveSubmittedKind==PAINT_LAYER_MOVE_COMMIT || m_layerMoveSubmittedKind==PAINT_LAYER_MOVE_CANCEL)){
                m_moveReceipt={m_layerMoveSequence,info.document_generation,info.revision,m_layerMoveSubmittedKind};
                InteractionTrace::point("move.receipt.sequence",double(m_layerMoveSequence));
                InteractionTrace::point("move.receipt.kind",m_layerMoveSubmittedKind);
                if(m_compositionEditBase)m_compositionFinalRevision=info.revision;
            }
            if(m_compositionEditBase && !m_compositionFinalRevision)clearCompositionEdit();
            m_layerMoveActive=false;m_layerMoveStarting=false;m_layerMoveClosing=false;
            m_layerMoveAnchor.reset();m_layerMoveGpuPresentation=false;m_layerMoveSource=0;
            m_layerMoveTimer->stop();
        } else if(m_layerMoveStarting && m_completedSequence>=m_layerMoveSequence) {
            m_layerMoveStarting=false;
            if(m_compositionEditBase){
                InteractionTrace::point("move.composition.targetConfirmed",double(m_compositionBeginSequence));
                m_compositionTargetNs=InteractionTrace::now();m_compositionMoveConfirmed=true;
                if(m_compositionMovePick)m_compositionEditTargets={info.active_layer_id};
                refreshCompositionMove();
            }
            if(m_layerMoveGpuPresentation) {
                // BEGIN may have rebuilt or rejected a preparation after an
                // intervening edit. Never wait for a source that the core did not adopt.
                auto preview=dto<PaintResidentPreviewInfo>();
                QMutexLocker lock(&m_connection->mutex);
                const auto status=m_connection->session?paint_session_resident_preview_info(m_connection->core,m_connection->session,&preview):PAINT_NOT_FOUND;
                m_layerMoveGpuPresentation=status==PAINT_OK && preview.document_generation==info.document_generation && preview.revision>=info.revision;
                m_layerMoveSource=m_layerMoveGpuPresentation?preview.source_id:0;
            }
        }
        if(m_layerMoveActive && !m_layerMoveStarting)flushLayerMove();
        if(m_drawing && m_liquifyActive && !m_dropStroke)flushLiquifyInput();
        const bool progressGenerationChanged=m_generation!=info.document_generation;
        m_generation = info.document_generation;
        if(progressGenerationChanged)emit computationProgressChanged();
        m_documentDpi=selection.value("dpi",72).toInt();
        m_revision = info.revision; m_width = static_cast<int>(info.width); m_height = static_cast<int>(info.height);
        // Closing metadata can precede the final pixels. Keep the resident
        // image until the primary frame can replace it without exposing the old frame.
        if(m_residentPreview && !m_liquifyActive && !m_layerMoveActive && !m_transformActive && m_frameRevisions[0]>=m_revision) {
            m_residentPreview.reset();m_transformMeshPreview=false;m_connection->transformMeshAssets.store(false);emit frameChanged();
        }
        m_undo = static_cast<int>(info.undo_depth); m_redo = static_cast<int>(info.redo_depth);
        if (info.completed_sequence >= m_pendingMutation) m_pendingMutation = 0;
        if (info.completed_sequence >= m_pendingLayer) m_pendingLayer=0;
        // A lasso/wand edit is a busy layer edit too: keep the panel and painting idled until the
        // worker published the resulting selection.

        if(m_pendingAdjustment && info.completed_sequence>=m_pendingAdjustment) {
            m_adjustmentFailed=info.last_error_status!=PAINT_OK && info.last_error_sequence==m_pendingAdjustment;
            m_pendingAdjustment=0;
            m_adjustmentActive=(info.flags&PAINT_SESSION_ADJUSTMENT)!=0;
            if(!m_adjustmentActive){
                if(m_adjustmentBase && m_adjustmentCommitRequested && !m_adjustmentFailed)m_adjustmentFinalRevision=info.revision;
                else {m_adjustmentBase.reset();m_adjustmentFrame.reset();}
                m_adjustmentCommitRequested=false;m_adjustmentSettings.reset();emit frameChanged();
            }else if(m_adjustmentCommitRequested){
                if(m_adjustmentFailed)m_adjustmentCommitRequested=false;
                else finishAdjustment(true);
            }
            emit stateChanged();
        }
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
                m_frames[i] = {}; m_regions[i] = {}; m_frameRevisions[i] = 0;m_frameIds[i]=0;m_frameRequestIds[i]=0;
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
        if (wasLiquify!=m_liquifyActive || wasTransform!=m_transformActive || wasTransformBusy!=transformBusy() || wasLiquifyBusy!=liquifyBusy() || propertiesChanged || wasReady != m_ready || wasModified != modified() || wasLayerBusy != layerEditBusy()) emit stateChanged();
        if(m_pendingCanvasResize && info.completed_sequence>=m_pendingCanvasResize){
            const bool success=info.last_error_status==PAINT_OK || info.last_error_sequence!=m_pendingCanvasResize;
            m_pendingCanvasResize=0;emit canvasResizeFinished(success);
        }
        requestLayerMovePreparation();
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
    connect(m_worker,&BackendWorker::residentPreviewReady,this,[this](ResidentPreviewPtr preview){
        m_connection->residentPreviewPending.store(false);
        if(m_closing || m_pendingNew || preview->generation!=m_generation || preview->revision<m_residentPreviewFloor || (!m_liquifyActive && !m_layerMoveActive && !m_transformActive))return;
        InteractionTrace::Scope trace("move.gui.acceptResident");
        if(trace.active)trace.event.value=double(preview->revision);
        InteractionTrace::point("move.gui.resident.source",double(preview->sourceId));
        InteractionTrace::point("move.gui.resident.generation",double(preview->generation));
        if(m_layerMoveGpuPresentation && !m_layerMoveAnchor && !m_layerMoveStarting && !m_layerMoveFinish &&
           preview->columns==2 && preview->rows==2 && preview->vertices.size()==4 &&
           preview->sourceId==m_layerMoveSource)
            m_layerMoveAnchor=preview;
        if(m_transformMeshPreview && m_transformActive){auto next=std::make_shared<ResidentPreviewFrame>(*preview);next->meshPreview=true;preview=std::move(next);}
        m_residentPreview=std::move(preview);
        if(m_transformMeshPreview && m_transformActive && !m_transformClosing)refreshTransformMeshPreview();
        if(m_layerMoveAnchor)refreshLayerMovePresentation();else emit frameChanged();
    });
    connect(m_worker,&BackendWorker::movePreparationReady,this,[this](quint64 generation,quint64 revision,quint64 layer,quint64 request,ResidentPreviewAssetsPtr assets){
        if(m_closing || !m_moveTool || m_layerMoveActive || generation!=m_generation ||
           revision!=m_revision || request!=m_movePreparationRequest ||
           (m_movePreparationIds.size()==1 && layer!=m_active))return;
        InteractionTrace::Scope trace("move.gui.acceptPreparation");
        if(trace.active)trace.event.value=double(request);
        InteractionTrace::point("move.gui.preparation.source",assets?double(assets->sourceId):0);
        m_movePreparationPendingRequest=0;
        m_movePreparation=std::move(assets);
        m_movePreparationCompleted=true;
        emit movePreparationChanged();
    });
    connect(m_worker,&BackendWorker::compositionReady,this,[this](CompositionPacketPtr packet,quint64 serial,QString fallback){
        m_connection->compositionPending=false;
        {QMutexLocker lock(&m_connection->mutex);if(m_closing || serial!=m_connection->compositionView.serial)return;}
        if(m_compositionFinalRevision && !fallback.isEmpty() && fallback!=QStringLiteral("CPU: disabled or active editing session") && fallback!=QStringLiteral("CPU: composition view unavailable")){
            QMutexLocker lock(&m_connection->mutex);
            if(m_connection->compositionView.enabled && !m_connection->compositionView.region.isEmpty() && !m_connection->compositionView.pixels.isEmpty())m_compositionFinalFallback=true;
        }
        m_compositionFallback=std::move(fallback);
        if(packet && packet->info.key.document_generation==m_generation &&
           (!m_compositionPacket || packet->info.key.revision>=m_compositionPacket->info.key.revision))m_compositionPacket=std::move(packet);
        else if(!m_compositionFallback.isEmpty())m_compositionPacket.reset();
        refreshCpuDisplayDemand();
        emit frameChanged();
    });
    connect(m_worker, &BackendWorker::pixels, this, [this](int view, const QImage &image, QRectF region, quint64 generation, quint64 revision, quint64 request, quint64 frameId) {
        m_connection->framePending[view].store(false);
        if (m_closing || m_pendingNew || generation != m_generation ||
            (m_requests[view] && request != m_requests[view]) || revision < m_frameRevisions[view]) return;
        if(view==0 && m_residentPreview && (generation!=m_residentPreview->generation || (!m_liquifyActive && !m_layerMoveActive && !m_transformActive && revision>=m_revision))){m_residentPreview.reset();m_transformMeshPreview=false;m_connection->transformMeshAssets.store(false);}
        // Selection-only publications advance the revision while keeping the same pixels.
        // Preserve QImage identity so CanvasItem also retains its scene graph texture.
        InteractionTrace::Scope trace("move.gui.acceptFrame");
        if(trace.active)trace.event.value=double(revision);
        InteractionTrace::point("move.frame.request",double(request));
        InteractionTrace::point("move.frame.view",view);
        InteractionTrace::point("move.frame.generation",double(generation));
        InteractionTrace::point("move.frame.id",double(frameId));
        InteractionTrace::point("move.frame.imageKey",double(image.cacheKey()));
        if(m_frames[view]!=image)m_frames[view]=image;
        m_regions[view] = region; m_frameRevisions[view] = revision;m_frameIds[view]=frameId;m_frameRequestIds[view]=request; emit frameChanged();
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
    m_brushLibrary->flushPersistence();
    // Window close uses asynchronous shutdown; this is a lifetime safety net.
    if (m_thread.isRunning()) {
        QMetaObject::invokeMethod(m_worker, &BackendWorker::stop, Qt::BlockingQueuedConnection);
        m_thread.quit(); m_thread.wait();
    }
}
void PaintCoreClient::shutdown() {
    if (m_closing) return;
    m_brushLibrary->flushPersistence();
    m_layerMoveTimer->stop();m_layerMoveActive=false;
    m_closing = true;emit computationProgressChanged(); m_ready = false; m_drawing = false;if(m_airbrushTimer)m_airbrushTimer->stop();
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
    if (type == Move || type == AirbrushTick || type == Begin) command.point = point(sample);
    if (type == Begin) {
        m_brushLibrary->flushPersistence();
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
            auto blend=dto<PaintBrushBlendDesc>();blend.mode=request.operation==PAINT_BRUSH_PAINT?quint32(m_brushLibrary->writeBlend()):0;
            status=paint_session_begin_prepared_preset_with_blend(m_connection->core,m_connection->session,m_brushCatalogConnection->library,m_brushCatalogConnection->preparation,reinterpret_cast<const uint8_t*>(presetId.constData()),quint64(presetId.size()),&request,&background,&blend,&command.point,&sequence);
            if(status==PAINT_OK){
                m_strokeBasePose={false,m_radius,m_brushLibrary->angle(),m_brushLibrary->roundness()};
                m_strokeOutlineResource=m_brushLibrary->maskResource();m_strokeOutlinePath=m_brushOutlinePath;m_strokeOutlineReady=m_brushOutlineReady;
                m_strokeSecondOutlineResource=m_secondOutlineResource;m_strokeSecondOutlinePath=m_secondOutlinePath;m_strokeSecondOutlineReady=m_secondOutlineReady;requestBrushOutlines(false);
                if(!m_strokePose)paint_brush_pose_create(&m_strokePose);
                m_strokePoseBound=m_strokePose && paint_brush_pose_bind_prepared(m_strokePose,m_brushCatalogConnection->library,m_brushCatalogConnection->preparation,reinterpret_cast<const uint8_t*>(presetId.constData()),quint64(presetId.size()),request.radius)==PAINT_OK;
            }
        } else status = paint_session_submit(m_connection->core, m_connection->session, &command, &sequence);
        if (status != PAINT_OK) error = immediateError(status);
    }
    if (status != PAINT_OK) {
        if ((type == Move || type == AirbrushTick || type == End) && (status == PAINT_LIMIT_EXCEEDED || status == PAINT_CANCELLED)) m_dropStroke = true;
        showError(error); return false;
    }
    if(type==Properties)QMetaObject::invokeMethod(m_worker,[worker=m_worker]{worker->transformInputActivity();},Qt::QueuedConnection);
    if(type==End && m_strokeColor.isValid())m_pendingStrokeColors.insert(sequence,m_strokeColor);
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
void PaintCoreClient::requestBrushOutlines(bool updateImages) {
    if(m_closing)return;
    if(updateImages)requestBrushCursorImages();
    QStringList identities;
    const auto add=[&](const QString &id,bool ready){if(!id.isEmpty() && !ready && !identities.contains(id))identities.append(id);};
    add(m_outlineResource,m_brushOutlineReady || !m_brushOutlineMessage.isEmpty());if(m_brushLibrary->cursorComposite())add(m_secondOutlineResource,m_secondOutlineReady || !m_secondOutlineMessage.isEmpty());
    if(m_drawing){add(m_strokeOutlineResource,m_strokeOutlineReady);if(m_brushLibrary->cursorComposite())add(m_strokeSecondOutlineResource,m_strokeSecondOutlineReady);}
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,identities]{worker->queueBrushOutlines(identities);},Qt::QueuedConnection);
}
void PaintCoreClient::requestBrushCursorImages() {
    if(m_closing)return;
    QStringList demand;
    const auto add=[&](const QString &id){if(!id.isEmpty() && !demand.contains(id))demand.append(id);};
    if(m_brushLibrary->cursorImage() && !m_brushLibrary->cursorSimple()){
        add(m_outlineResource);
        const auto dual=m_brushLibrary->dualBrush();
        if(m_brushLibrary->cursorComposite() && !m_eraser && (!m_smudgeTool || m_smudgeFingerPainting) && dual.value("enabled").toBool())add(m_secondOutlineResource);
        if(m_drawing){add(m_strokeOutlineResource);if(m_brushLibrary->cursorComposite() && m_strokeSecondImageEligible)add(m_strokeSecondOutlineResource);}
    }
    m_cursorImageDemand=demand;
    for(auto i=m_cursorImages.begin();i!=m_cursorImages.end();)if(!demand.contains(i.key()))i=m_cursorImages.erase(i);else ++i;
    for(auto i=m_cursorImageErrors.begin();i!=m_cursorImageErrors.end();)if(!demand.contains(i.key()))i=m_cursorImageErrors.erase(i);else ++i;
    QStringList pending;for(const auto &id:demand)if(!m_cursorImages.contains(id) && !m_cursorImageErrors.contains(id))pending.append(id);
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,pending]{worker->queueBrushCursorImages(pending);},Qt::QueuedConnection);
    emit brushCursorImageChanged();
}
BrushCursorPose PaintCoreClient::baseCursorPose() const {
    if(m_drawing && !m_liquifyActive)return m_strokeBasePose;
    return m_liveBasePose;
}
BrushCursorPose PaintCoreClient::brushCursorPose(const InputSample &sample,QPointF direction,bool directionKnown,bool pressureKnown) {
    InteractionTrace::Scope trace("brushCursorPose");
    auto result=baseCursorPose();if(!m_ready || m_closing || m_liquifyActive)return result;
    PaintBrushPose *handle=nullptr;
    if(m_drawing){if(!m_strokePoseBound)return result;handle=m_strokePose;}
    else {
        // Pending preferences describe the next stroke. The already bound
        // stroke pose remains valid; only hover must await the new catalog.
        if(m_brushLibrary->persistencePending())return result;
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
    const auto input=point(sample);
    if(!m_brushLibrary->cursorComposite()){
        auto primary=dto<PaintTipPose>();
        if(paint_brush_pose_read(handle,&input,pressureKnown,directionKnown,directionKnown?direction.x():0,directionKnown?direction.y():0,&primary)==PAINT_OK)result={true,primary.radius,primary.angle,primary.roundness};
        return result;
    }
    auto pose=dto<PaintCompositeTipPose>();
    const auto poseStatus=paint_brush_pose_read_composite(handle,&input,pressureKnown,directionKnown,directionKnown?direction.x():0,directionKnown?direction.y():0,&pose);
    if(poseStatus==PAINT_OK)result={true,pose.primary.radius,pose.primary.angle,pose.primary.roundness,bool((pose.flags&PAINT_POSE_SECONDARY) && (pose.flags&PAINT_POSE_SECONDARY_OVERFLOW)),pose.secondary.radius,pose.secondary.angle,pose.secondary.roundness};
    return result;
}
void PaintCoreClient::ensureOverview() {
    const QRectF document(0,0,documentWidth(),documentHeight());
    if(document.isEmpty() || (m_viewEnabled[1] && m_requestedRegions[1]==document))return;
    const auto scale=std::min(1.,512./std::max(document.width(),document.height()));
    m_requestingAutomaticOverview=true;
    requestViewport(1,document,{std::max(1,qCeil(document.width()*scale)),std::max(1,qCeil(document.height()*scale))});
    m_requestingAutomaticOverview=false;
}
qreal PaintCoreClient::currentMovePreparationScale() const {
    qreal scale=1;
    bool found=false;
    for(int view=0;view<2;++view){
        const auto &region=m_requestedRegions[view];
        const auto pixels=m_requestedPixels[view];
        if(!m_viewEnabled[view] || region.isEmpty() || pixels.isEmpty())continue;
        const qreal density=std::max(qreal(pixels.width())/region.width(),qreal(pixels.height())/region.height());
        if(std::isfinite(density) && density>0){scale=found?std::max(scale,density):density;found=true;}
    }
    return std::min<qreal>(scale,16);
}
QVector<quint64> PaintCoreClient::currentMoveTargets() const {
    if(m_selectedLayers.size()>1)return m_selectedLayers.toVector();
    return m_active?QVector<quint64>{m_active}:QVector<quint64>{};
}
void PaintCoreClient::clearLayerMovePreparation() {
    // The request counter stays monotonic to reject stale results. It is not
    // evidence of live preparation: clearing an already empty cache must be a no-op.
    if(!m_movePreparation && !m_movePreparationPendingRequest && !m_movePreparationCompleted)return;
    ++m_movePreparationRequest;
    if(!m_movePreparationRequest)++m_movePreparationRequest;
    m_movePreparation.reset();
    m_movePreparationPendingRequest=0;
    m_movePreparationCompleted=false;
    emit movePreparationChanged();
    uint64_t sequence=0;
    QMutexLocker lock(&m_connection->mutex);
    if(m_connection->session)paint_session_clear_layer_move_preparation(m_connection->core,m_connection->session,&sequence);
}
void PaintCoreClient::requestLayerMovePreparation() {
    if(m_layerMoveActive)return;
    if(m_compositionPacket && m_compositionPacket->info.key.document_generation==m_generation &&
       m_compositionPacket->info.key.revision==m_revision && gpuCompositionEnabled()){
        clearLayerMovePreparation();return;
    }
    if(!gpuAccelerationEnabled() || !m_ready || m_closing || m_pendingNew || !m_moveTool ||
       m_liquifyActive || m_transformActive || m_drawing || m_fileBusy || !m_active){
        clearLayerMovePreparation();
        return;
    }
    const auto scale=currentMovePreparationScale();
    const auto targets=currentMoveTargets();
    const bool same=m_movePreparationGeneration==m_generation &&
        m_movePreparationRevision==m_revision && m_movePreparationLayer==m_active &&
        m_movePreparationIds==targets && qFuzzyCompare(m_movePreparationScale,scale);
    if(same && (m_movePreparationPendingRequest || m_movePreparationCompleted))return;
    ++m_movePreparationRequest;
    if(!m_movePreparationRequest)++m_movePreparationRequest;
    m_movePreparation.reset();
    m_movePreparationPendingRequest=m_movePreparationRequest;
    m_movePreparationCompleted=false;
    m_movePreparationGeneration=m_generation;
    m_movePreparationRevision=m_revision;
    m_movePreparationLayer=m_active;
    m_movePreparationIds=targets;
    m_movePreparationScale=scale;
    emit movePreparationChanged();
    uint64_t sequence=0;
    PaintStatus status;
    {
        QMutexLocker lock(&m_connection->mutex);
        if(!m_connection->session){
            m_movePreparationPendingRequest=0;
            return;
        }
        const std::vector<uint64_t> ids(m_movePreparationIds.cbegin(),m_movePreparationIds.cend());
        status=paint_session_prepare_layer_move(m_connection->core,m_connection->session,
            ids.data(),static_cast<uint32_t>(ids.size()),scale,m_movePreparationRequest,&sequence);
    }
    InteractionTrace::point("move.prepare.request",double(m_movePreparationRequest));
    InteractionTrace::point("move.prepare.sequence",double(sequence));
    if(status!=PAINT_OK)m_movePreparationPendingRequest=0;
}
CompositionPacketPtr PaintCoreClient::compositionPacket()const{
    if(m_transformMeshPreview && (m_transformActive || m_residentPreview))return {};
    if(!gpuCompositionEnabled() || m_blendPreviewLayer || m_compositionFinalFallback || m_compositionTransformCpu)return {};
    if(m_adjustmentBase){
        if(m_adjustmentFinalRevision && m_compositionPacket && m_compositionPacket->info.key.document_generation==m_generation && m_compositionPacket->info.key.revision>=m_adjustmentFinalRevision)return m_compositionPacket;
        return m_adjustmentFrame?m_adjustmentFrame:m_adjustmentBase;
    }
    if(m_compositionFinalRevision && m_compositionPacket && m_compositionPacket->info.key.document_generation==m_generation &&
       m_compositionPacket->info.key.revision>=m_compositionFinalRevision)return m_compositionPacket;
    if(m_compositionEditFrame)return m_compositionEditFrame;
    if(m_compositionEditBase)return m_compositionEditBase;
    return m_compositionPacket;
}
void PaintCoreClient::clearCompositionEdit(){
    m_compositionEditBase.reset();m_compositionEditFrame.reset();m_compositionEditTargets.clear();
    m_compositionFinalRevision=m_compositionBeginSequence=0;m_compositionMoveConfirmed=false;
    m_compositionTransform=m_compositionFinalFallback=false;m_compositionEditFallbackReason.clear();m_compositionTargetNs=0;
}
bool PaintCoreClient::prepareCompositionMove(bool pick){
    if(m_compositionFinalRevision){m_compositionFinalFallback=true;m_compositionTransform=false;emit frameChanged();}
    if(m_compositionEditBase || !m_compositionPacket || !residentGpuPreviewEnabled() ||
       !gpuCompositionEnabled() ||
       m_compositionPacket->info.key.document_generation!=m_generation || m_compositionPacket->info.key.revision!=m_revision)return false;
    const auto targets=currentMoveTargets();
    if(!pick)for(auto id:targets){
        const auto at=std::find_if(m_compositionPacket->nodes.begin(),m_compositionPacket->nodes.end(),[id](const auto &node){return node.layer_id==id && node.kind!=PAINT_COMPOSITION_MASK;});
        if(at==m_compositionPacket->nodes.end())return false;
    }
    m_compositionEditBase=m_compositionPacket;m_compositionEditTargets=targets;m_compositionMovePick=pick;
    m_compositionMoveConfirmed=false;m_compositionFinalRevision=0;return true;
}
bool PaintCoreClient::prepareCompositionTransform(){
    if(m_compositionPacket && !m_compositionPacket->operations.isEmpty())return false;
    if(!m_compositionPrecisionAvailable || selectionEnabled() || currentMoveTargets().size()!=1 || !prepareCompositionMove(false))return false;
    const auto id=m_compositionEditTargets.front();
    for(const auto &node:m_compositionEditBase->nodes)if(node.layer_id==id && node.kind==0){
        if(node.mask_index<quint32(m_compositionEditBase->nodes.size()) || quint32(node.reserved)!=0){clearCompositionEdit();return false;}
        m_compositionTransform=true;return true;
    }
    clearCompositionEdit();return false;
}
void PaintCoreClient::setTransformPreviewLimits(int edge,quint64 availableGpuBytes){
    if(m_transformActive || m_transformMeshPreview)return;
    // Keep space for mip levels and concurrent display resources, and for the
    // Rust packet, worker transfer, Qt host copy and bounded renderer scratch.
    m_connection->transformMaxEdge.store(quint32(std::max(0,edge)));
    m_connection->transformMaxBytes.store(std::min(CompositionBudget::memoryBytes.load()/4,availableGpuBytes/2*3/4));
}
bool PaintCoreClient::preferTransformMeshPreview() const {
    // The three-image split is valid only for a single Normal root and a
    // Normal foreground. Keep the exact compositor for every other stack.
    if(!residentGpuPreviewEnabled() || !gpuCompositionEnabled() || highestCanvasQuality() ||
       (selectionEnabled() && !m_connection->transformSourcePairSupported.load()) ||
       !m_connection->transformMaxEdge.load() || m_connection->transformMaxBytes.load()<(selectionEnabled()?72u:36u))return false;
    const auto targets=currentMoveTargets();if(targets.size()!=1)return false;
    // The general compositor can decline a large viewport while the cached
    // source texture still fits the device. Core resident splitting remains
    // authoritative; metadata permits only the conservative simple-root case.
    if(!m_compositionPacket || m_compositionPacket->info.key.document_generation!=m_generation ||
       m_compositionPacket->info.key.revision!=m_revision){
        if(quint64(m_width)*m_height<=1024ull*1024 && !selectionEnabled())return false;
        bool found=false;
        for(const auto &entry:m_layers){const auto layer=entry.toMap();
            if(layer.value("mask").toBool() && layer.value("parent").toULongLong()==targets.front())return false;
            if(layer.value("id").toULongLong()==targets.front()){
                if(layer.value("parent").toULongLong() || layer.value("blendMode").toUInt()!=0 ||
                   layer.value("group").toBool() || layer.value("mask").toBool() || layer.value("image").toBool() || layer.value("clipped").toBool())return false;
                found=true;
            }else if(!found && layer.value("visible").toBool() &&
                (layer.value("blendMode").toUInt()!=0 || layer.value("group").toBool() || layer.value("mask").toBool() || layer.value("clipped").toBool()))return false;
        }
        return found;
    }
    if(!m_compositionPacket->operations.isEmpty())return false;
    bool found=false;
    for(const auto &node:m_compositionPacket->nodes){
        if(node.kind==PAINT_COMPOSITION_MASK)continue;
        if(node.layer_id==targets.front()){
            if(node.mask_index<quint32(m_compositionPacket->nodes.size()) || quint32(node.reserved)!=0)return false;
            // Unselected small edits retain the exact path. A selected split
            // benefits from immediate geometry even when its moving part is small.
            const quint64 area=std::max<quint64>(quint64(node.white_width)*node.white_height,quint64(node.tile_count)*4096);
            if(area<=1024ull*1024 && !selectionEnabled())return false;
            found=true;
        }else if(found && quint32(node.reserved)!=0)return false;
    }
    return found;
}
void PaintCoreClient::refreshTransformMeshPreview(){
    if(!m_transformMeshPreview || !m_transformActive || m_transformClosing || !m_transformPublished ||
       !m_residentPreview || !m_residentPreview->forward || m_residentPreview->vertices.isEmpty() ||
       m_transformPoints.size()!=16)return;
    auto geometry=dto<PaintTransform>();geometry.warp=m_transformWarp;geometry.interpolation=m_transformInterpolation;
    geometry.skew_h=m_transformSkewH;geometry.skew_v=m_transformSkewV;
    for(int i=0;i<16;++i){const auto p=m_transformPoints[i].toPointF();geometry.points[i*2]=p.x();geometry.points[i*2+1]=p.y();}
    auto display=dto<PaintTransformDisplayGeometry>();QVector<std::array<float,4>> vertices(145*145);
    if(paint_transform_display_geometry(&geometry,&display,reinterpret_cast<float*>(vertices.data()),84100)!=PAINT_OK)return;
    vertices.resize(qsizetype(display.vertex_bytes/16));
    // Source UV endpoints include the transparent guard and any original
    // placement bounds. Reproject current input, never a delayed callback's xy.
    const auto first=m_residentPreview->vertices.front(),last=m_residentPreview->vertices.back();
    for(auto &v:vertices){v[2]=first[2]+v[2]*(last[2]-first[2]);v[3]=first[3]+v[3]*(last[3]-first[3]);}
    auto next=std::make_shared<ResidentPreviewFrame>(*m_residentPreview);
    if(next->prepared){
        const bool identity=m_transformPoints==m_transformInitialPoints && m_transformWarp==m_transformInitialWarp &&
            m_transformInterpolation==m_transformInitialInterpolation && m_transformSkewH==m_transformInitialSkewH && m_transformSkewV==m_transformInitialSkewV;
        if(next->transformIdentity!=identity){
            const auto alternate=next->prepared;
            next->prepared=std::make_shared<ResidentPreviewAssets>(*next);
            static_cast<ResidentPreviewAssets&>(*next)=*alternate;
        }
    }
    next->meshPreview=true;next->meshNearest=m_transformInterpolation==0;next->geometrySequence=++m_transformMeshGeometry;
    next->columns=display.columns;next->rows=display.rows;next->vertices=std::move(vertices);
    m_residentPreview=std::move(next);
}
void PaintCoreClient::refreshCompositionTransform(const PaintTransform *geometry){
    if(!m_compositionTransform || !m_compositionMoveConfirmed || m_transformInitialPoints.size()!=16 || m_transformPoints.size()!=16)return;
    auto t=CompositionTransform::allocate();
    if(!t){m_compositionFinalFallback=true;emit frameChanged();return;}
    t->geometry=dto<PaintTransform>();
    if(geometry)t->geometry=*geometry;
    else {
        t->geometry.warp=m_transformWarp;t->geometry.interpolation=m_transformInterpolation;
        t->geometry.skew_h=m_transformSkewH;t->geometry.skew_v=m_transformSkewV;
        for(int i=0;i<16;++i){const auto p=m_transformPoints[i].toPointF();t->geometry.points[i*2]=p.x();t->geometry.points[i*2+1]=p.y();}
    }
    const auto top=m_transformInitialPoints[0].toPointF(),right=m_transformInitialPoints[3].toPointF(),bottom=m_transformInitialPoints[12].toPointF();
    t->sourceBounds={top,QSizeF(right.x()-top.x(),bottom.y()-top.y())};
    if(m_compositionEditFrame && m_compositionEditFrame->transform &&
       m_compositionEditFrame->transform->sourceBounds==t->sourceBounds &&
       std::memcmp(&m_compositionEditFrame->transform->geometry,&t->geometry,sizeof(PaintTransform))==0)return;
    if(t->sourceBounds.isEmpty() || t->sourceBounds.width()>4096 || t->sourceBounds.height()>4096 ||
       std::abs(top.x())>1048576 || std::abs(top.y())>1048576){m_compositionFinalFallback=true;emit frameChanged();return;}
    t->display=dto<PaintTransformDisplayGeometry>();t->vertices=QByteArray(145*145*16,Qt::Uninitialized);
    const auto status=paint_transform_display_geometry(&t->geometry,&t->display,reinterpret_cast<float*>(t->vertices.data()),84100);
    if(status!=PAINT_OK){m_compositionFinalFallback=true;emit frameChanged();return;}
    t->vertices.resize(qsizetype(t->display.vertex_bytes));
    double minX=INFINITY,minY=INFINITY,maxX=-INFINITY,maxY=-INFINITY;
    const float *vertices=reinterpret_cast<const float*>(t->vertices.constData());
    for(qsizetype i=0;i<t->vertices.size()/16;++i){minX=std::min(minX,double(vertices[i*4]));minY=std::min(minY,double(vertices[i*4+1]));maxX=std::max(maxX,double(vertices[i*4]));maxY=std::max(maxY,double(vertices[i*4+1]));}
    if(!t->geometry.warp){minX=minY=INFINITY;maxX=maxY=-INFINITY;for(int i:{0,3,12,15}){minX=std::min(minX,t->geometry.points[i*2]);minY=std::min(minY,t->geometry.points[i*2+1]);maxX=std::max(maxX,t->geometry.points[i*2]);maxY=std::max(maxY,t->geometry.points[i*2+1]);}}
    t->scale=std::min((std::ceil(maxX)-std::floor(minX))/t->sourceBounds.width(),(std::ceil(maxY)-std::floor(minY))/t->sourceBounds.height());
    for(int i=0;i<m_compositionEditBase->nodes.size();++i)if(m_compositionEditBase->nodes[i].layer_id==m_compositionEditTargets.front() && m_compositionEditBase->nodes[i].kind==0){t->node=i;break;}
    if(t->node<0){m_compositionFinalFallback=true;emit frameChanged();return;}
    auto next=std::make_shared<CompositionPacket>(*m_compositionEditBase);
    next->transform=std::move(t);next->preview=true;next->geometrySequence=++m_compositionGeometry;next->editSequence=m_compositionBeginSequence;
    m_compositionEditFrame=std::move(next);InteractionTrace::point("transform.composition.localGeometry",double(m_compositionGeometry));emit frameChanged();
}
void PaintCoreClient::refreshCompositionMove(){
    if(!m_compositionEditBase || !m_compositionMoveConfirmed)return;
    auto next=std::make_shared<CompositionPacket>(*m_compositionEditBase);
    const auto delta=m_layerMoveFinish && !m_layerMoveCommit?QPoint{}:m_layerMoveDelta;
    QSet<int> moving;
    for(auto id:m_compositionEditTargets){
        int index=-1;for(int i=0;i<next->nodes.size();++i)if(next->nodes[i].layer_id==id && next->nodes[i].kind!=PAINT_COMPOSITION_MASK){index=i;break;}
        if(index<0){clearCompositionEdit();return;}
        moving.insert(index);
        if(next->nodes[index].kind==PAINT_COMPOSITION_GROUP){
            for(int i=0;i<next->nodes.size();++i){
                int ancestor=i;
                for(int depth=0;depth<17;++depth){
                    const auto parent=quint32((next->nodes[ancestor].reserved>>33)&0xffff);
                    if(!parent || parent>quint32(next->nodes.size()))break;
                    ancestor=int(parent)-1;if(ancestor==index){moving.insert(i);break;}
                }
            }
        }
    }
    // Masks follow their actual owner's movement only when linked. In
    // particular, moving a child leaves its ancestor group's mask in place.
    QSet<int> pixels;
    for(int index:moving)if(next->nodes[index].kind!=PAINT_COMPOSITION_MASK){
        if(next->nodes[index].kind==PAINT_COMPOSITION_RASTER)pixels.insert(index);
        const auto mask=next->nodes[index].mask_index;
        if(mask<quint32(next->nodes.size()) && !(next->nodes[int(mask)].reserved & PAINT_COMPOSITION_NODE_FIXED_MASK))pixels.insert(int(mask));
    }
    for(int index:pixels){
        auto &node=next->nodes[index];const qint64 x=qint64(node.offset_x)+delta.x(),y=qint64(node.offset_y)+delta.y();
        if(std::abs(x)>1048576 || std::abs(y)>1048576){clearCompositionEdit();return;}
        node.offset_x=int(x);node.offset_y=int(y);
    }
    next->geometrySequence=++m_compositionGeometry;next->editSequence=m_compositionBeginSequence;
    next->presentationDelta=delta;next->preview=true;m_compositionEditFrame=std::move(next);
    InteractionTrace::point("move.composition.localGeometry",double(m_compositionGeometry));
    emit frameChanged();
}
void PaintCoreClient::compositionPrimaryAdopted(quint64 generation,quint64 epoch,quint64 revision){
    if(m_adjustmentFinalRevision && generation==m_generation && revision>=m_adjustmentFinalRevision){
        {QMutexLocker lock(&m_connection->mutex);if(epoch!=m_connection->compositionView.epoch)return;}
        m_adjustmentBase.reset();m_adjustmentFrame.reset();m_adjustmentFinalRevision=0;emit frameChanged();return;
    }
    if(!m_compositionFinalRevision || generation!=m_generation || revision<m_compositionFinalRevision)return;
    {QMutexLocker lock(&m_connection->mutex);if(epoch!=m_connection->compositionView.epoch)return;}
    InteractionTrace::point("move.composition.finalPrimaryAdopted",double(revision));
    clearCompositionEdit();emit frameChanged();
}
void PaintCoreClient::setCompositionDevice(quint64 epoch,bool enabled){
    if(m_adjustmentBase && (!enabled || m_adjustmentBase->info.key.device_epoch!=epoch)){
        m_adjustmentBase.reset();m_adjustmentFrame.reset();m_adjustmentFinalRevision=0;emit stateChanged();emit frameChanged();
    }
    m_compositionDeviceAvailable=enabled;
    enabled=enabled && gpuCompositionEnabled() && !m_closing && m_ready && !m_transformActive && !m_liquifyActive && !m_layerMoveActive && !m_blendPreviewLayer;
    const bool native=highestCanvasQuality() && quint64(m_width)*m_height<=CompositionBudget::viewPixels();
    const QRectF targetRegion=native?QRectF(0,0,m_width,m_height):m_requestedRegions[0];
    const QSize targetPixels=native?QSize(int(m_width),int(m_height)):m_requestedPixels[0];
    enabled=enabled && targetPixels.width()<=8192 && targetPixels.height()<=8192 &&
        quint64(targetPixels.width())*targetPixels.height()<=CompositionBudget::viewPixels();
    {
        QMutexLocker lock(&m_connection->mutex);auto &view=m_connection->compositionView;
        if(view.epoch!=epoch || view.enabled!=enabled || view.generation!=m_generation || view.region!=targetRegion || view.pixels!=targetPixels){
            ++view.serial;view.epoch=epoch;view.enabled=enabled;view.generation=m_generation;view.region=targetRegion;view.pixels=targetPixels;
            if(!enabled || (m_compositionPacket && m_compositionPacket->info.key.document_generation!=m_generation))m_compositionPacket.reset();
        }
    }
    refreshCpuDisplayDemand();
}
void PaintCoreClient::refreshCpuDisplayDemand(){
    if(!m_ready || m_closing)return;
    const auto packet=compositionPacket();
    quint64 epoch=0;bool enabled=false;
    {QMutexLocker lock(&m_connection->mutex);epoch=m_connection->compositionView.epoch;enabled=m_connection->compositionView.enabled;}
    const bool gpu=m_compositionDeviceAvailable && gpuCompositionEnabled() && qEnvironmentVariableIntValue("DRAWVERSE_CPU_DISPLAY_REFERENCE")!=1 &&
        packet && packet->info.key.document_generation==m_generation && packet->info.key.device_epoch==epoch &&
        (enabled || (m_compositionEditBase && !m_compositionFinalFallback));
    for(int view=0;view<2;++view){
        const bool suppressed=gpu && (view==0 ? !m_cpuSamplingRequired && !m_eyedropperTool : !m_overviewExplicit);
        if(m_cpuViewSuppressed[view]==suppressed)continue;
        m_cpuViewSuppressed[view]=suppressed;
        // A new mailbox request cancels in-flight CPU work. Keep the last
        // complete display while GPU preparation or CPU recovery is pending.
        m_requests[view]=0;
        const bool automatic=m_requestingAutomaticOverview;
        m_requestingAutomaticOverview=true;
        requestViewport(view,m_requestedRegions[view],m_requestedPixels[view],m_viewEnabled[view]);
        m_requestingAutomaticOverview=automatic;
    }
}
void PaintCoreClient::requestViewport(int view, QRectF region, QSize pixels, bool enabled) {
    if (!m_ready || m_closing || view < 0 || view > 1) return;
    if(view==1 && !m_requestingAutomaticOverview){
        if(m_overviewExplicit!=enabled)m_requests[view]=0;
        m_overviewExplicit=enabled;
        if(enabled)m_cpuViewSuppressed[view]=false;
    }
    if (enabled) {
        if (!std::isfinite(region.x()) || !std::isfinite(region.y()) || !std::isfinite(region.width()) ||
            !std::isfinite(region.height()) || region.isEmpty() || pixels.isEmpty()) return;
        const double ratio = std::min({1., 8192. / pixels.width(), 8192. / pixels.height(),
            std::sqrt(double(CompositionBudget::cpuViewPixels()) / (static_cast<double>(pixels.width()) * pixels.height()))});
        pixels = {std::max(1, static_cast<int>(pixels.width() * ratio)), std::max(1, static_cast<int>(pixels.height() * ratio))};
    }
    if (m_requests[view] && m_viewEnabled[view] == enabled && m_requestedRegions[view] == region && m_requestedPixels[view] == pixels) return;
    auto viewport = dto<PaintViewport>(); viewport.view_id = static_cast<uint32_t>(view); viewport.enabled = enabled ? 1 : 0;
    if(m_cpuViewSuppressed[view] && !(view==1 && m_overviewExplicit))viewport.enabled=0;
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
        if(view==0 && enabled && !highestCanvasQuality() && m_compositionEditBase &&
           (m_compositionEditBase->region!=region || m_compositionEditBase->pixels!=pixels)){
            const double scale=std::min(region.width()/pixels.width(),region.height()/pixels.height());
            const int lod=scale>=1?int(std::floor(std::log2(scale))):0;
            if(pixels.width()>8192 || pixels.height()>8192 || quint64(pixels.width())*pixels.height()>CompositionBudget::viewPixels() || lod>6){
                m_compositionFinalFallback=true;m_compositionEditFallbackReason=QStringLiteral("CPU: composition edit viewport budget");
            }else{
                // The bounded source plan contains every visible layer tile,
                // not just the initial viewport. Retarget only Qt's display
                // projection; the immutable Rust source/lease identity stays.
                const auto project=[&](const CompositionPacketPtr &source){
                    auto next=std::make_shared<CompositionPacket>(*source);
                    next->region=region;next->pixels=pixels;next->lod=lod;next->geometrySequence=++m_compositionGeometry;
                    return CompositionPacketPtr(std::move(next));
                };
                m_compositionEditBase=project(m_compositionEditBase);
                if(m_compositionEditFrame)m_compositionEditFrame=project(m_compositionEditFrame);
            }
            emit frameChanged();
        }
        if(view==0 && enabled && !highestCanvasQuality() && m_adjustmentBase && (m_adjustmentBase->region!=region || m_adjustmentBase->pixels!=pixels)){
            const double scale=std::min(region.width()/pixels.width(),region.height()/pixels.height());const int lod=scale>=1?int(std::floor(std::log2(scale))):0;
            if(pixels.width()>8192 || pixels.height()>8192 || quint64(pixels.width())*pixels.height()>CompositionBudget::viewPixels() || lod>6){
                m_adjustmentBase.reset();m_adjustmentFrame.reset();m_adjustmentFinalRevision=0;emit stateChanged();
            }else{
                const auto project=[&](const CompositionPacketPtr &source){auto next=std::make_shared<CompositionPacket>(*source);next->region=region;next->pixels=pixels;next->lod=lod;next->geometrySequence=++m_compositionGeometry;return CompositionPacketPtr(std::move(next));};
                m_adjustmentBase=project(m_adjustmentBase);if(m_adjustmentFrame)m_adjustmentFrame=project(m_adjustmentFrame);
            }
            emit frameChanged();
        }
        if(view==0 && enabled && m_compositionPacket && !m_compositionEditBase && !m_adjustmentBase && !highestCanvasQuality() &&
           m_compositionPacket->info.key.revision==m_revision && quint64(pixels.width())*pixels.height()<=CompositionBudget::viewPixels()){
            auto next=std::make_shared<CompositionPacket>(*m_compositionPacket);
            next->region=region;next->pixels=pixels;
            const double scale=std::min(region.width()/pixels.width(),region.height()/pixels.height());
            next->lod=scale>=1?std::clamp(int(std::floor(std::log2(scale))),0,6):0;
            next->geometrySequence=++m_compositionGeometry;m_compositionPacket=std::move(next);emit frameChanged();
        }
        requestLayerMovePreparation();
    } else if (!error.isEmpty()) showError(error);
}
void PaintCoreClient::showError(const QString &error) {
    if(m_layerMoveActive) {
        InteractionTrace::point("move.backend.error",double(m_layerMoveSequence));
        m_layerMoveGpuPresentation=false;m_layerMoveAnchor.reset();m_residentPreview.reset();
        emit frameChanged();
    }
    m_error=error;emit errorChanged();
}
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
void PaintCoreClient::inspectCompute() {
    if(m_worker)QMetaObject::invokeMethod(m_worker,[worker=m_worker]{worker->inspectCompute();},Qt::QueuedConnection);
}
bool PaintCoreClient::saveComputeDevice(const QString &id) {
    if(id.isEmpty() || id.size()>1024 || m_closing || m_drawing || m_liquifyActive || m_transformActive || m_layerMoveActive || m_adjustmentActive || m_computeBusy || !m_worker)return false;
    m_computeBusy=true;emit computeChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,id]{worker->saveCompute(id);},Qt::QueuedConnection);
    return true;
}
void PaintCoreClient::setBrushColor(const QColor &c) {
    InteractionTrace::Scope trace("setBrushColor");
    // Picking a colour is only a preview: the swatch row advances when the pixel stroke commits.
    if (c.isValid() && c != m_color) m_brushLibrary->setActiveColors(c,m_secondaryColor);
}
void PaintCoreClient::setSecondaryBrushColor(const QColor &c){if(c.isValid() && c!=m_secondaryColor)m_brushLibrary->setActiveColors(m_color,c);}
void PaintCoreClient::swapBrushColors(){
    // Swapping selects a colour; the recent swatch row only advances on a committed stroke.
    m_brushLibrary->setActiveColors(m_secondaryColor,m_color);
}
void PaintCoreClient::setBrushWriteBlend(int mode) {m_brushLibrary->setWriteBlend(mode);}
void PaintCoreClient::setBrushRadius(qreal r) {m_brushLibrary->setRadius(r);}
void PaintCoreClient::setBrushOpacity(qreal o) {m_brushLibrary->setOpacity(o);}
void PaintCoreClient::setBrushSpacing(qreal s) {m_brushLibrary->setSpacing(s);}
// Active tools are exclusive; the remembered marquee shape is separate from these flags.
void PaintCoreClient::setEraser(bool e) {
    if(deferTransformAction([this ,e]{setEraser(e);}))return;
    if(m_drawing || m_liquifyActive || m_transformActive || (e==m_eraser && !m_moveTool && !m_selectionTool && !m_eyedropperTool && !m_bucketTool && !m_smudgeTool && m_brushLibrary->toolContext()==(e?1:0))) return;
    if(m_moveTool)clearLayerMovePreparation();
    m_moveTool=false;m_selectionTool=0;m_eyedropperTool=false;m_bucketTool=false;m_smudgeTool=false;
    m_eraser=e; m_brushLibrary->setEraser(e); emit brushChanged();
}
void PaintCoreClient::setMoveTool(bool enabled) {
    if(deferTransformAction([this,enabled]{setMoveTool(enabled);}))return;
    if(m_drawing || m_liquifyActive || m_transformActive || enabled==m_moveTool) return;
    if(enabled){m_selectionTool=0;m_eyedropperTool=false;m_bucketTool=false;m_smudgeTool=false;m_eraser=false;}
    m_moveTool=enabled; emit brushChanged();
    if(enabled)requestLayerMovePreparation();else clearLayerMovePreparation();
}
void PaintCoreClient::setSelectionTool(int tool) {
    if(deferTransformAction([this,tool]{setSelectionTool(tool);}))return;
    if(tool<0 || tool>5 || m_drawing || m_liquifyActive || m_transformActive || tool==m_selectionTool) return;
    m_selectionTool=tool;
    if(tool>0) {
        if(tool<=2)m_marqueeShape=tool;else if(tool<=4)m_lassoShape=tool;
        if(m_moveTool)clearLayerMovePreparation();
        m_moveTool=false;m_eyedropperTool=false;m_bucketTool=false;m_smudgeTool=false;m_eraser=false;
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
    setMoveTool(true);
}
void PaintCoreClient::activateMarquee() {if(deferTransformAction([this]{activateMarquee();}))return;
    if(m_drawing || m_liquifyActive)return;
    // Activating the slot restores its remembered shape.
    const int shape = (m_selectionTool==1 || m_selectionTool==2) ? m_selectionTool : m_marqueeShape;
    setMoveTool(false);
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
void PaintCoreClient::activateSmudge() {if(deferTransformAction([this]{activateSmudge();}))return;if(m_drawing || m_liquifyActive || m_transformActive)return;if(m_moveTool)clearLayerMovePreparation();m_moveTool=false;m_selectionTool=0;m_eyedropperTool=false;m_bucketTool=false;m_eraser=false;m_smudgeTool=true;m_brushLibrary->setToolContext(2);emit brushChanged();}
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
    if(!layerContentVisible(m_active))return false;
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
    if(points.size()<3)return false;
    if(std::all_of(points.cbegin(),points.cend(),[](QPointF p){return qIsFinite(p.x()) && qIsFinite(p.y());})) {
        const auto a=points.first();
        const auto b=std::find_if(points.cbegin(),points.cend(),[a](QPointF p){return p!=a;});
        if(b==points.cend() || std::none_of(points.cbegin(),points.cend(),[a,b](QPointF p){
            return std::abs((b->x()-a.x())*(p.y()-a.y())-(b->y()-a.y())*(p.x()-a.x()))>1e-9;
        }))return false;
    }
    QVector<PaintSelectionPoint> path;
    path.reserve(points.size());for(const auto &p:points)path.append({p.x(),p.y()});
    auto request=dto<PaintSelectionShapeEdit>();
    request.shape=PAINT_SELECTION_POLYGON;request.tool=polygonal?2u:1u;request.operation=static_cast<uint32_t>(operation);
    request.antialias=m_selectionAntialias;request.feather=m_selectionFeather;
    request.point_count=static_cast<uint32_t>(path.size());request.points=path.constData();
    return submitSelectionShape(request);
}

bool PaintCoreClient::selectAll(){return submitSelection(PAINT_SELECTION_ALL);}
bool PaintCoreClient::selectLayerPixels(quint64 id,int operation) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy || m_closing || operation<0 || operation>3)return false;
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;
        status=paint_session_select_layer_alpha(m_connection->core,m_connection->session,id?id:m_active,static_cast<uint32_t>(operation),&sequence);
        if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}
    m_pendingLayer=m_pendingMutation=sequence;m_modified=true;emit stateChanged();return true;
}
bool PaintCoreClient::selectionPixels(int kind,qreal amount,int color,qreal opacity) {
    if(!m_ready || !selectionEnabled() || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy || m_closing || kind<0 || kind>4)return false;
    QColor c=color==1?QColor(Qt::white):color==2?QColor(Qt::black):m_color;
    uint64_t sequence=0;
    QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;
    const auto status=paint_session_selection_pixels(m_connection->core,m_connection->session,static_cast<uint32_t>(kind),amount,linear(c.redF()),linear(c.greenF()),linear(c.blueF()),float(c.alphaF()*opacity),&sequence);
    lock.unlock();
    if(status!=PAINT_OK){showError(immediateError(status));return false;}
    m_pendingLayer=m_pendingMutation=sequence;m_modified=true;emit stateChanged();return true;
}
bool PaintCoreClient::clearSelection(){return submitSelection(PAINT_SELECTION_CLEAR);}
bool PaintCoreClient::invertSelection(){return submitSelection(PAINT_SELECTION_INVERT);}
void PaintCoreClient::persistLiquifyOptions() {
    emit liquifyOptionsChanged();
    const QVariantMap values{{"tool",m_liquifyTool},{"size",m_liquifySize},{"strength",m_liquifyStrength},{"density",m_liquifyDensity},{"speed",m_liquifySpeed},{"pressure",m_liquifyPressure}};
    if(m_worker && !m_closing)QMetaObject::invokeMethod(m_worker,[worker=m_worker,values]{worker->saveLiquifyOptions(values);},Qt::QueuedConnection);
}
void PaintCoreClient::setLiquifyTool(int v){if(v<0 || v>3 || m_drawing || v==m_liquifyTool)return;m_liquifyTool=v;persistLiquifyOptions();}
void PaintCoreClient::setLiquifySize(int v){if(v<1 || v>15000 || m_drawing || v==m_liquifySize)return;m_liquifySize=v;persistLiquifyOptions();}
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
    m_liquifyCommitting=kind==PAINT_LIQUIFY_COMMIT;
    if(kind==PAINT_LIQUIFY_BEGIN || kind==PAINT_LIQUIFY_BEGIN_DEFERRED){m_residentPreviewFloor=m_revision+1;m_residentPreview.reset();m_liquifyActive=true;m_liquifyFocus=false;emit liquifyOptionsChanged();}
    emit stateChanged();return true;
}
bool PaintCoreClient::beginLiquify() {
    if(!layerContentVisible(m_active))return false;
    if(m_liquifyActive)return false;
    for(const auto &value:m_layers){const auto l=value.toMap();if(l.value("id").toULongLong()==m_active && (l.value("group").toBool() || l.value("effectiveLocks").toUInt())) {showError(QStringLiteral("液化需要未锁定的像素图层或蒙版"));return false;}}
    clearLayerBlendPreview();clearError();return submitLiquifyControl(residentGpuPreviewEnabled()?PAINT_LIQUIFY_BEGIN_DEFERRED:PAINT_LIQUIFY_BEGIN);
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
bool PaintCoreClient::layerContentVisible(quint64 id) const {
    // A child of a hidden group is hidden even when its own eye is enabled.
    for(int depth=0;id && depth<=m_layers.size();++depth){
        QVariantMap node;
        for(const auto &value:m_layers)if(value.toMap().value("id").toULongLong()==id){node=value.toMap();break;}
        if(node.isEmpty() || !node.value("visible").toBool())return false;
        id=node.value("parent").toULongLong();
    }
    return id==0;
}
bool PaintCoreClient::selectedContentVisible() const {
    if(!layerContentVisible(m_active))return false;
    for(auto id:m_selectedLayers)if(!layerContentVisible(id))return false;
    return true;
}
bool PaintCoreClient::beginStroke(const InputSample &s) {
    if(!layerContentVisible(m_active))return false;
    if (!m_ready || m_drawing || m_wetActionBusy || layerEditBusy() || m_pendingLiquify) return false;
    if(m_brushLibrary->brushSelectionBlocksPainting())return false;
    if(m_liquifyActive) {m_dropStroke=false;m_hasLiquifyPending=false;m_liquifyLast=s;m_liquifyClock.restart();if(!submitLiquifyStrokeMarker(PAINT_LIQUIFY_STROKE_BEGIN))return false;if(!submitLiquifyDab(s,s,16)){submitLiquifyStrokeMarker(PAINT_LIQUIFY_STROKE_END);return false;}m_drawing=true;emit stateChanged();return true;}
    if(m_residentPreview){m_residentPreview.reset();emit frameChanged();}
    for(const auto &value:m_layers) { const auto layer=value.toMap(); if(layer.value("id").toULongLong()!=m_active) continue;
        if(layer.value("group").toBool()) {showError(QStringLiteral("请选中组内像素图层绘画，或先新建图层"));return false;}
        if(layer.value("effectiveLocks").toUInt() & PAINT_LOCK_ALL) {showError(QStringLiteral("当前图层或所属组已完全锁定，请先解锁"));return false;}
    }
    if(m_pendingStrokeColors.size()>=4096) {showError(QStringLiteral("正在完成先前笔触，请稍后重试"));return false;}
    m_dropStroke = false; if (!submit(Begin, s)) return false;
    const auto capturedDual=m_brushLibrary->dualBrush();
    m_strokeSecondImageEligible=!m_eraser && (!m_smudgeTool || m_smudgeFingerPainting) && s.tool!=PAINT_TOOL_ERASER && capturedDual.value("enabled").toBool();
    m_strokeColor = m_smudgeTool && !m_smudgeFingerPainting ? QColor() : m_color;
    const auto air= m_brushLibrary->airbrush();
    m_airbrushEnabled=air.value("enabled").toBool() && !m_smudgeTool;
    m_airbrushIntervalNs=quint64(qMax(1,qRound(air.value("intervalMs",50).toDouble()*1000000.)));
    m_airbrushSample=s;m_airbrushTimestamp=s.timestamp;
    if(m_airbrushEnabled){m_airbrushTimer->setInterval(qMax(1,qRound(air.value("intervalMs",50).toDouble())));m_airbrushTimer->start();}else m_airbrushTimer->stop();
    m_drawing = true;requestBrushCursorImages();emit stateChanged(); return true;
}
void PaintCoreClient::strokeTo(const InputSample &s) {
    if(!m_drawing || m_dropStroke)return;
    InputSample adjusted=s;
    if(m_airbrushEnabled && adjusted.timestamp<m_airbrushTimestamp)adjusted.timestamp=m_airbrushTimestamp;
    m_airbrushSample=adjusted;
    if(m_liquifyActive){m_liquifyPending=adjusted;m_hasLiquifyPending=true;flushLiquifyInput();}
    else submit(Move,adjusted);
}
void PaintCoreClient::airbrushTick() {
    if(!m_drawing || m_dropStroke || m_liquifyActive || !m_airbrushEnabled)return;
    InputSample sample=m_airbrushSample;
    const quint64 next=m_airbrushTimestamp+m_airbrushIntervalNs;
    sample.timestamp=next;
    if(submit(AirbrushTick,sample))m_airbrushTimestamp=next;else m_dropStroke=true;
}
void PaintCoreClient::flushLiquifyInput(bool force) {
    if(!m_hasLiquifyPending || m_dropStroke || (!force && m_lastLiquifyDab>m_completedSequence))return;
    const auto sample=m_liquifyPending;m_hasLiquifyPending=false;
    if(submitLiquifyDab(m_liquifyLast,sample,std::clamp(int(m_liquifyClock.restart()),0,50)))m_liquifyLast=sample;
    else m_dropStroke=true;
}
void PaintCoreClient::liquifyTick() {
    if(!m_drawing || !m_liquifyActive || m_dropStroke || m_lastLiquifyDab>m_completedSequence)return;
    if(m_hasLiquifyPending){flushLiquifyInput();return;}
    if(m_liquifyTool==1 || m_liquifyTool==2) {
        if(!submitLiquifyDab(m_liquifyLast,m_liquifyLast,std::clamp(int(m_liquifyClock.restart()),0,50)))m_dropStroke=true;
    }
}
void PaintCoreClient::endStroke() {
    if (!m_drawing) return;
    if(m_liquifyActive){flushLiquifyInput(true);submitLiquifyStrokeMarker(PAINT_LIQUIFY_STROKE_END);m_drawing=false;m_pendingLayer=m_lastLiquifyDab>m_completedSequence?m_lastLiquifyDab:0;emit stateChanged();return;}
    if (!m_dropStroke) submit(End);
    m_airbrushTimer->stop();m_airbrushEnabled=false;m_drawing = false;if(m_strokePose)paint_brush_pose_clear(m_strokePose);m_strokePoseBound=false;m_strokeOutlineResource.clear();m_strokeOutlinePath.clear();m_strokeSecondOutlineResource.clear();m_strokeSecondOutlinePath.clear();m_strokeSecondOutlineReady=false;requestBrushOutlines();emit brushOutlineChanged();emit stateChanged();
}
void PaintCoreClient::cancelStroke() {
    if (!m_drawing) return;
    if(m_liquifyActive){flushLiquifyInput(true);submitLiquifyStrokeMarker(PAINT_LIQUIFY_STROKE_END);m_drawing=false;m_pendingLayer=m_lastLiquifyDab>m_completedSequence?m_lastLiquifyDab:0;emit stateChanged();return;}
    if (!m_dropStroke) submit(Cancel);
    m_airbrushTimer->stop();m_airbrushEnabled=false;m_dropStroke = true;m_drawing = false;if(m_strokePose)paint_brush_pose_clear(m_strokePose);m_strokePoseBound=false;m_strokeOutlineResource.clear();m_strokeOutlinePath.clear();m_strokeSecondOutlineResource.clear();m_strokeSecondOutlinePath.clear();m_strokeSecondOutlineReady=false;requestBrushOutlines();emit brushOutlineChanged();emit stateChanged();
}
void PaintCoreClient::restoreLongTaskDelay(int value) {
    value=std::clamp(value,0,10000);if(m_longTaskDelay==value)return;m_longTaskDelay=value;emit longTaskDelayChanged();emit computationProgressChanged();
}
void PaintCoreClient::setLongTaskDelay(int value) {
    value=std::clamp(value,0,10000);if(m_longTaskDelay==value)return;restoreLongTaskDelay(value);
    if(!m_closing){auto settings=m_settingsFile.isEmpty()?std::make_unique<QSettings>(QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse"):std::make_unique<QSettings>(m_settingsFile,QSettings::IniFormat);settings->setValue("performance/taskProgressDelayMs",value);settings->sync();if(settings->status()!=QSettings::NoError)showError(QStringLiteral("无法保存进度显示延迟"));}
}
bool PaintCoreClient::selectHistory(int depth) {
    if(!m_ready || m_closing || m_drawing || m_fileBusy || layerEditBusy() || m_layerMoveActive || m_pendingLiquify || m_pendingTransform>m_completedSequence || depth<0 || depth>m_undo+m_redo)return false;
    if(depth==m_undo)return true;
    auto request=dto<PaintHistorySelect>();request.depth=static_cast<uint32_t>(depth);request.document_generation=m_generation;request.revision=m_revision;
    request.scope=m_transformActive?PAINT_HISTORY_SCOPE_TRANSFORM:m_liquifyActive?PAINT_HISTORY_SCOPE_LIQUIFY:PAINT_HISTORY_SCOPE_DOCUMENT;
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;status=paint_session_select_history(m_connection->core,m_connection->session,&request,&sequence);if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}
    m_pendingLayer=sequence;
    if(m_transformActive){m_lastTransform=sequence;m_pendingTransform=sequence;m_residentPreviewFloor=m_revision+1;}
    else if(m_liquifyActive){m_pendingLiquify=sequence;m_residentPreviewFloor=m_revision+1;}
    else {m_pendingMutation=sequence;m_modified=true;}
    emit stateChanged();return true;
}
void PaintCoreClient::undo() { if (m_ready && !m_drawing && !m_layerMoveActive) {if(m_transformActive)submitTransform(PAINT_TRANSFORM_UNDO);else if(m_liquifyActive)submitLiquifyControl(PAINT_LIQUIFY_UNDO);else submit(Undo);} }
void PaintCoreClient::redo() { if (m_ready && !m_drawing && !m_layerMoveActive) {if(m_transformActive)submitTransform(PAINT_TRANSFORM_REDO);else if(m_liquifyActive)submitLiquifyControl(PAINT_LIQUIFY_REDO);else submit(Redo);} }

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
void PaintCoreClient::deleteSelectionOrLayers() {
    if(selectionEnabled())submitLayerAction(PAINT_LAYER_ACTION_CLEAR_SELECTED_PIXELS);else requestSelectedLayerRemoval();
}
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
    m_blendPreviewLayer=id;m_blendPreviewMode=blend;emit frameChanged();
    for(int view=0;view<2;++view)if(m_viewEnabled[view]) {
        m_requests[view]=0;requestViewport(view,m_requestedRegions[view],m_requestedPixels[view]);
    }
}
void PaintCoreClient::clearLayerBlendPreview() {
    if(!m_blendPreviewLayer)return;
    m_blendPreviewLayer=0;emit frameChanged();
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
bool PaintCoreClient::beginLayerMove(QPointF position,bool pick) {
    InteractionTrace::Scope trace("move.begin");
    if(!m_ready || m_closing || m_fileBusy || m_drawing || m_liquifyActive || m_transformActive || layerEditBusy())return false;
    if(pick && (!std::isfinite(position.x()) || !std::isfinite(position.y()) || position.x()<0 || position.y()<0 || position.x()>=m_width || position.y()>=m_height))return false;
    if(!selectedContentVisible())return false;
    const auto targets=currentMoveTargets();
    const bool composition=prepareCompositionMove(pick);
    const bool prepared=residentGpuPreviewEnabled() && !pick && m_movePreparationCompleted &&
        m_movePreparation && m_movePreparationGeneration==m_generation &&
        m_movePreparationRevision==m_revision &&
        (m_movePreparationIds.size()>1 || m_movePreparationLayer==m_active) &&
        m_movePreparationIds==targets;
    const bool deferred=!composition && (prepared || (pick && residentGpuPreviewEnabled()));
    const auto kind=pick?(deferred?PAINT_LAYER_MOVE_PICK_DEFERRED:PAINT_LAYER_MOVE_PICK):(deferred?PAINT_LAYER_MOVE_BEGIN_DEFERRED:PAINT_LAYER_MOVE_BEGIN);
    uint64_t sequence=0;
    PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session){if(composition)clearCompositionEdit();return false;}if(!pick && m_selectedLayers.size()>1){QVector<uint64_t> ids;for(auto id:m_selectedLayers)ids.append(id);status=paint_session_begin_layer_moves(m_connection->core,m_connection->session,ids.data(),uint32_t(ids.size()),deferred?1:0,&sequence);}else status=paint_session_layer_move(m_connection->core,m_connection->session,kind,m_active,pick?int(position.x()):0,pick?int(position.y()):0,&sequence);if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){if(composition)clearCompositionEdit();showError(error);return false;}
    if(trace.active)trace.event.value=double(sequence);
    InteractionTrace::point("move.begin.prepared",prepared);
    m_layerMoveSubmittedKind=kind;
    m_layerMoveSequence=sequence;m_layerMoveDelta={};m_layerMoveDirty=false;m_layerMoveFinish=false;
    if(composition)m_compositionBeginSequence=sequence;
    m_layerMoveActive=true;m_layerMoveStarting=true;m_layerMoveClosing=false;
    m_layerMoveGpuPresentation=deferred;m_layerMoveSource=prepared?m_movePreparation->sourceId:0;m_layerMoveAnchor.reset();
    m_residentPreviewFloor=m_revision+1;m_residentPreview.reset();m_layerMoveTimer->start();
    if(composition)QMetaObject::invokeMethod(m_worker,[worker=m_worker,sequence]{worker->watchCompositionAcknowledgement(sequence);},Qt::QueuedConnection);
    emit stateChanged();return true;
}
void PaintCoreClient::updateLayerMove(QPoint delta) {
    if(!m_layerMoveActive || m_layerMoveClosing || m_layerMoveFinish)return;
    if(delta==m_layerMoveDelta)return;
    InteractionTrace::point("move.changedDelta.x",delta.x());
    m_layerMoveDelta=delta;m_layerMoveDirty=true;
    refreshCompositionMove();
    refreshLayerMovePresentation();
}
void PaintCoreClient::refreshLayerMovePresentation() {
    if(!m_layerMoveGpuPresentation || !m_layerMoveAnchor || !m_residentPreview ||
       m_residentPreview->sourceId!=m_layerMoveAnchor->sourceId)return;
    const QPoint delta=m_layerMoveFinish && !m_layerMoveCommit?QPoint{}:m_layerMoveDelta;
    auto next=std::make_shared<ResidentPreviewFrame>(*m_residentPreview);
    // Keep core revision/source identity; only the four UVs follow newer input.
    // Always derive from zero, never accumulate float error or an older reply's delta.
    next->vertices=m_layerMoveAnchor->vertices;
    const auto &v=next->vertices;
    const float du=(v[1][2]-v[0][2])/float(next->documentSize.width());
    const float dv=(v[2][3]-v[0][3])/float(next->documentSize.height());
    for(auto &vertex:next->vertices){vertex[2]-=float(delta.x())*du;vertex[3]-=float(delta.y())*dv;}
    next->presentationDelta=delta;
    m_residentPreview=std::move(next);
    InteractionTrace::point("move.gui.directPresentation.x",delta.x());
    emit frameChanged();
}
void PaintCoreClient::finishLayerMove(QPoint delta,bool commit) {
    if(!m_layerMoveActive || m_layerMoveClosing)return;
    m_layerMoveDelta=delta;m_layerMoveFinish=true;m_layerMoveCommit=commit;
    refreshCompositionMove();
    if(!m_layerMoveAnchor)m_layerMoveGpuPresentation=false;
    refreshLayerMovePresentation();flushLayerMove();
}
void PaintCoreClient::flushLayerMove() {
    if(!m_layerMoveActive || m_layerMoveStarting || m_layerMoveClosing || m_completedSequence<m_layerMoveSequence || (!m_layerMoveDirty && !m_layerMoveFinish))return;
    // The first resident packet is the core-confirmed zero-delta baseline.
    // Closing never waits for a display asset, including a quick press/release.
    if(m_layerMoveGpuPresentation && !m_layerMoveAnchor && !m_layerMoveFinish)return;
    const auto kind=m_layerMoveFinish?(m_layerMoveCommit?PAINT_LAYER_MOVE_COMMIT:PAINT_LAYER_MOVE_CANCEL):PAINT_LAYER_MOVE_UPDATE;
    const auto delta=kind==PAINT_LAYER_MOVE_CANCEL?QPoint{}:m_layerMoveDelta;
    InteractionTrace::Scope trace("move.submit");
    InteractionTrace::point("move.submit.kind",kind);
    InteractionTrace::point("move.submit.dx",delta.x());
    InteractionTrace::point("move.submit.dy",delta.y());
    uint64_t sequence=0;
    PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return;status=paint_session_layer_move(m_connection->core,m_connection->session,kind,0,delta.x(),delta.y(),&sequence);if(status!=PAINT_OK && status!=PAINT_BUSY)error=immediateError(status);}
    if(trace.active)trace.event.value=double(sequence);
    InteractionTrace::point("move.submit.status",status);
    if(status==PAINT_BUSY)return;
    if(status!=PAINT_OK){showError(error);m_layerMoveFinish=true;m_layerMoveCommit=false;return;}
    m_layerMoveSubmittedKind=kind;
    m_layerMoveSequence=sequence;m_layerMoveDirty=false;
    if(m_layerMoveFinish){
        m_layerMoveClosing=true;if(m_layerMoveCommit && !delta.isNull())m_pendingMutation=sequence;
        if(m_compositionEditBase)QMetaObject::invokeMethod(m_worker,[worker=m_worker,sequence]{worker->watchCompositionAcknowledgement(sequence);},Qt::QueuedConnection);
    }
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
QString PaintCoreClient::tabletInputMode() const {
    auto settings=m_settingsFile.isEmpty()?std::make_unique<QSettings>(QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse"):std::make_unique<QSettings>(m_settingsFile,QSettings::IniFormat);
    const auto mode=settings->value("input/tabletApi","auto").toString();
    return mode=="windows-ink" || mode=="wintab"?mode:QStringLiteral("auto");
}
QString PaintCoreClient::tabletInputBackend() const {return activeTabletInputBackend();}
void PaintCoreClient::setTabletInputMode(const QString &mode) {
    if(m_closing || (mode!="auto" && mode!="windows-ink" && mode!="wintab") || mode==tabletInputMode())return;
    auto settings=m_settingsFile.isEmpty()?std::make_unique<QSettings>(QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse"):std::make_unique<QSettings>(m_settingsFile,QSettings::IniFormat);
    settings->setValue("input/tabletApi",mode);settings->sync();
    if(settings->status()!=QSettings::NoError){showError(QStringLiteral("无法保存手绘板输入设置"));return;}
    emit tabletInputSettingsChanged();
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
bool PaintCoreClient::canPasteLayers() const {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy)return false;
    const auto *clipboard=QGuiApplication::clipboard();
    const auto *mime=clipboard?clipboard->mimeData():nullptr;
    return mime && mime->hasFormat("application/x-drawverse-layers-v1");
}
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
    if((kind==PAINT_LAYER_ACTION_CLEAR_SELECTED_PIXELS || kind==PAINT_LAYER_ACTION_CUT_SELECTED_PIXELS) && !selectedContentVisible())return false;
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy || m_closing)return false;
    clearLayerBlendPreview();
    std::vector<uint64_t> ids(m_selectedLayers.begin(),m_selectedLayers.end());
    if(kind==PAINT_LAYER_ACTION_CLEAR_SELECTED_PIXELS)for(const auto &value:m_layers){const auto layer=value.toMap();if(layer.value("id").toULongLong()==m_active && layer.value("mask").toBool()){ids={m_active};break;}}
    const bool paste=kind==PAINT_LAYER_ACTION_PASTE;
    if(!paste && ids.empty())return false;
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;
        status=paint_session_layer_action(m_connection->core,m_connection->session,kind,paste?nullptr:ids.data(),paste?0:static_cast<uint32_t>(ids.size()),paste?m_active:0,
            paste?reinterpret_cast<const uint8_t*>(bytes.constData()):nullptr,paste?static_cast<uint64_t>(bytes.size()):0,&sequence);
        if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}
    m_pendingLayer=sequence;
    const bool copy=kind==PAINT_LAYER_ACTION_COPY || kind==PAINT_LAYER_ACTION_COPY_SELECTED_PIXELS;
    if(copy || kind==PAINT_LAYER_ACTION_CUT || kind==PAINT_LAYER_ACTION_CUT_SELECTED_PIXELS){m_pendingCopy=sequence;QMetaObject::invokeMethod(m_worker,[worker=m_worker,sequence]{worker->watchLayerCopy(sequence);},Qt::QueuedConnection);}
    if(!copy){m_pendingMutation=sequence;m_modified=true;}
    emit stateChanged();return true;
}
bool PaintCoreClient::copySelectedLayers(){return submitLayerAction(selectionEnabled()?PAINT_LAYER_ACTION_COPY_SELECTED_PIXELS:PAINT_LAYER_ACTION_COPY);}
bool PaintCoreClient::cutSelectedLayers(){
    for(const auto &value:m_layers){const auto layer=value.toMap();if(layer.value("id").toULongLong()==m_active && layer.value("mask").toBool()){
        showError(QStringLiteral("无法单独剪切蒙版，请选择所属图层"));return false;
    }}
    return submitLayerAction(selectionEnabled()?PAINT_LAYER_ACTION_CUT_SELECTED_PIXELS:PAINT_LAYER_ACTION_CUT);
}
bool PaintCoreClient::pasteLayers(){return canPasteLayers() && submitLayerAction(PAINT_LAYER_ACTION_PASTE,QGuiApplication::clipboard()->mimeData()->data("application/x-drawverse-layers-v1"));}
bool PaintCoreClient::duplicateSelectedLayers(){return submitLayerAction(PAINT_LAYER_ACTION_DUPLICATE);}
bool PaintCoreClient::mergeSelectedLayers(){return canMergeLayers() && submitLayerAction(PAINT_LAYER_ACTION_MERGE);}

bool PaintCoreClient::submitGroup(quint32 kind,quint64 id,quint64 parent,const QString &name) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy() || m_fileBusy) return false;
    const auto bytes=name.toUtf8();auto request=dto<PaintGroupRequest>();request.kind=kind;request.layer_id=id;request.parent_id=parent;
    if(kind==PAINT_GROUP_WRAP || kind==PAINT_GROUP_RENAME_LAYER) {request.name=reinterpret_cast<const uint8_t*>(bytes.constData());request.name_length=static_cast<uint64_t>(bytes.size());}
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session) return false;
        status=paint_session_group(m_connection->core,m_connection->session,&request,&sequence);
        if(status!=PAINT_OK) error=immediateError(status);
    }
    if(status!=PAINT_OK) {showError(error);return false;}
    m_pendingLayer=m_pendingMutation=sequence;m_modified=true;emit stateChanged();return true;
}
bool PaintCoreClient::groupLayer(quint64 id,const QString &name) {return submitGroup(PAINT_GROUP_WRAP,id,0,name);}
bool PaintCoreClient::setMaskLinked(quint64 id,bool linked) {
    if(!ready() || layerEditBusy() || m_transformActive || m_liquifyActive)return false;
    return submitGroup(PAINT_GROUP_MASK_LINK,id,linked?1:0,{});
}
bool PaintCoreClient::renameLayer(quint64 id,const QString &name) {
    const auto clean=name.trimmed();if(clean.isEmpty() || clean.toUtf8().size()>1024 || clean.contains(QChar(0)))return false;
    for(const auto &value:m_layers){const auto layer=value.toMap();if(layer.value("id").toULongLong()==id && layer.value("name").toString()==clean)return true;}
    return submitGroup(PAINT_GROUP_RENAME_LAYER,id,0,clean);
}
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
    return !ids.isEmpty() && !ids.contains(target) && dropLayers(ids,target,placement);
}
void PaintCoreClient::beginLayerDrag(quint64 id) {
    if(!m_ready || m_liquifyActive || m_transformActive || m_drawing || layerEditBusy())return;
    for(const auto &v:m_layers)if(v.toMap().value("id").toULongLong()==id) {
        if(v.toMap().value("mask").toBool() || (v.toMap().value("effectiveLocks").toUInt() & 4))return;
        if(!m_selectedLayers.contains(id))selectLayers(id,0);
        QDrag drag(this);auto *mime=new QMimeData;mime->setData("application/x-drawverse-layer",layerDragPayload(id).toUtf8());drag.setMimeData(mime);drag.exec(Qt::MoveAction);return;
    }
}
void PaintCoreClient::beginBrushDrag(const QString &payload) {
    if(m_closing || m_drawing || m_liquifyActive || m_transformActive)return;
    const int separator=payload.indexOf(':');
    if(separator<=0)return;
    const auto kind=payload.left(separator);const auto id=payload.mid(separator+1);
    if((kind!="preset" && kind!="group") || id.isEmpty())return;
    QDrag drag(this);auto *mime=new QMimeData;mime->setData("application/x-drawverse-brush",payload.toUtf8());drag.setMimeData(mime);drag.exec(Qt::MoveAction);
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
    if(m_ready && !m_drawing && !m_fileBusy && !m_layerMoveActive && submit(New,{},static_cast<quint64>(w),{},h)) {m_documentUrl={};m_sourceUrl={};emit stateChanged();}
}
void PaintCoreClient::newDocument(int w, int h) {
    if (w < 1 || h < 1 || w > 1000000 || h > 1000000) { showError(QStringLiteral("画布尺寸支持 1–1,000,000 像素")); return; }
    if (m_ready && !m_drawing && !m_fileBusy && !m_layerMoveActive && submit(NewWhite, {}, static_cast<quint64>(w), {}, h)) {
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
    m_brushImportGroupName=QFileInfo(path.toLocalFile()).completeBaseName().trimmed().left(80);
    if(m_brushImportGroupName.isEmpty())m_brushImportGroupName=QStringLiteral("导入画笔");
    if(!m_brushLibraryReady){if(!m_pendingBrushImport.isEmpty())return false;m_pendingBrushImport=path.toLocalFile();m_brushImportMessage.clear();m_brushFileBusy=true;emit brushFileChanged();return true;}
    m_brushImportMessage.clear();
    m_brushFileBusy=true;emit brushFileChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,path=path.toLocalFile()]{worker->brushFile(path,{},false);},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::importBrushTip(const QUrl &path,int interpretation) {
    if(m_closing || !m_ready || m_brushFileBusy || !path.isLocalFile() || path.toLocalFile().isEmpty() || interpretation<0 || interpretation>2)return false;
    const auto target=m_brushLibrary->selectedId();m_brushImportMessage.clear();m_brushFileBusy=true;emit brushFileChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,path=path.toLocalFile(),target,interpretation]{worker->importBrushTip(path,target,interpretation);},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::refreshBrushResources() {
    if(m_closing || !m_brushLibraryReady || m_brushResourcesBusy)return false;
    m_brushResourcesBusy=true;emit brushResourcesChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker]{worker->listBrushResources();},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::requestBrushResourcePreview(const QString &identity) {
    if(m_closing || !m_brushLibraryReady || identity.size()!=64 || m_brushResourcePreviews.contains(identity) || m_pendingBrushResourcePreviews.contains(identity) || m_pendingBrushResourcePreviews.size()>=32)return false;
    for(const auto c:identity)if(!((c>='0' && c<='9') || (c>='a' && c<='f')))return false;
    m_pendingBrushResourcePreviews.insert(identity);
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,identity]{worker->previewBrushResource(identity);},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::selectBrushResource(const QString &identity,int role) {
    if(role==3 || role==4)return !m_closing && m_brushLibraryReady && !m_brushFileBusy && m_brushResources.contains(identity) && m_brushLibrary->setOutlineTextureResource(m_brushLibrary->selectedId(),identity,role==4);
    if(m_closing || !m_ready || !m_brushLibraryReady || m_brushFileBusy || role<0 || role>2 || identity.size()!=64)return false;
    for(const auto c:identity)if(!((c>='0' && c<='9') || (c>='a' && c<='f')))return false;
    const auto target=m_brushLibrary->selectedId();m_brushImportMessage.clear();m_brushFileBusy=true;emit brushFileChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,identity,target,role]{worker->selectBrushResource(identity,target,role);},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::importBrushTexture(const QUrl &path,int interpretation,bool outline,bool selfOutline) {
    if(m_closing || !m_ready || m_brushFileBusy || !path.isLocalFile() || path.toLocalFile().isEmpty() || interpretation<0 || interpretation>2)return false;
    const auto target=m_brushLibrary->selectedId();m_brushImportMessage.clear();m_brushFileBusy=true;emit brushFileChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,path=path.toLocalFile(),target,interpretation,outline,selfOutline]{worker->importBrushTip(path,target,interpretation,outline?(selfOutline?4:3):1);},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::importDualBrushTip(const QUrl &path,int interpretation) {
    if(m_closing || !m_ready || m_brushFileBusy || !path.isLocalFile() || path.toLocalFile().isEmpty() || interpretation<0 || interpretation>2)return false;
    const auto target=m_brushLibrary->selectedId();m_brushImportMessage.clear();m_brushFileBusy=true;emit brushFileChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,path=path.toLocalFile(),target,interpretation]{worker->importBrushTip(path,target,interpretation,2);},Qt::QueuedConnection);return true;
}
void PaintCoreClient::persistBrushPreferences(const QByteArray &state) {
    InteractionTrace::Scope trace("persistBrushPreferences");
    if(!m_brushPreferencePersistenceEnabled || m_closing)return;
    {QMutexLocker lock(&m_brushCatalogConnection->mutex);m_brushCatalogConnection->preferences=state;}
    if(!synchronizeBrushCatalog(state))return;
    const bool tracing=InteractionTrace::enabled.load(std::memory_order_relaxed);
    const auto queuedAt=tracing?InteractionTrace::now():0;
    const int sourceOperation=tracing?InteractionTrace::operation:-1;
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,state,queuedAt,sourceOperation]{
        const int previous=InteractionTrace::operation;InteractionTrace::operation=sourceOperation;
        if(queuedAt)InteractionTrace::point("preferences.queue_us",double(InteractionTrace::now()-queuedAt)/1000);
        {InteractionTrace::Scope trace("worker.saveBrushPreferences");worker->saveBrushPreferences(state);}
        InteractionTrace::operation=previous;
    },Qt::QueuedConnection);
}
bool PaintCoreClient::restoreBrushPreferences(const QByteArray &state,const BrushLibrary *source) {
    InteractionTrace::Scope trace("preferences.receiver.restore");
    if(source && source->snapshot()==state && m_brushLibrary->restoreSharedSettings(*source))return true;
    if(!m_brushLibrary->restore(state))return false;
    // Other documents share catalog/preferences, while each client retains its active tool.
    // Receiving a different tool context must not start a persistence feedback loop.
    m_brushLibrary->setToolContext(m_smudgeTool?2:m_eraser?1:0,false);
    return true;
}
bool PaintCoreClient::synchronizeBrushCatalog(const QByteArray &state) {
    InteractionTrace::Scope trace("synchronizeBrushCatalog");
    {QMutexLocker lock(&m_brushCatalogConnection->mutex);if(m_brushCatalogConnection->preparation && m_brushCatalogConnection->requestedState==state)return true;}
    QByteArray source;
    if(state==m_brushLibrary->snapshot())source=m_brushLibrary->catalogSnapshot();
    else {
        const auto root=QJsonDocument::fromJson(state).object();
        QJsonObject catalog{{"version",root.value("version")},{"selected",root.value("selected")},{"presets",root.value("presets")}};if(root.contains("organization"))catalog["organization"]=root.value("organization");
        source=QJsonDocument(catalog).toJson(QJsonDocument::Compact);
    }
    QMutexLocker lock(&m_brushCatalogConnection->mutex);
    if(!m_brushCatalogConnection->library)return false;
    if(source==m_brushCatalogConnection->source && m_brushCatalogConnection->preparation){m_brushCatalogConnection->requestedState=state;return true;}
    uint64_t preparation=0;const auto status=paint_brush_library_prepare_state_groups(m_brushCatalogConnection->library,reinterpret_cast<const uint8_t*>(source.constData()),quint64(source.size()),&preparation);
    if(status!=PAINT_OK){lock.unlock();showError(immediateError(status));return false;}
    InteractionTrace::point("catalog.request.preparation",double(preparation));
    m_brushCatalogConnection->source=source;m_brushCatalogConnection->requestedState=state;m_brushCatalogConnection->preparation=preparation;return true;
}
bool PaintCoreClient::exportBrush(const QUrl &path,bool includeSource,const QString &expectedPreset) {
    if(m_closing || !m_ready || m_brushFileBusy || !path.isLocalFile() || path.toLocalFile().isEmpty())return false;
    if(!expectedPreset.isEmpty() && expectedPreset!=m_brushLibrary->selectedId()){showError(QStringLiteral("待导出的笔刷已更改，请重新选择导出"));return false;}
    const auto package=m_brushLibrary->exportPackage(includeSource);
    if(package.isEmpty()){showError(QStringLiteral("请在导出选项中选择包含原始 ABR 包；原始包可能包含其他预设和资源"));return false;}
    m_brushFileBusy=true;emit brushFileChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,path=path.toLocalFile(),package]{worker->brushFile(path,package,true);},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::exportAllBrushes(const QUrl &path,bool includeSource) {
    if(m_closing || !m_ready || m_brushFileBusy || !path.isLocalFile() || path.toLocalFile().isEmpty())return false;
    const auto package=m_brushLibrary->exportAllPackage(includeSource);
    if(package.isEmpty()){showError(QStringLiteral("无法导出全部画笔：请检查来源包含选项、组层数或画笔数量"));return false;}
    m_brushFileBusy=true;emit brushFileChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,path=path.toLocalFile(),package]{worker->brushFile(path,package,true);},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::exportSelectedBrushes(const QUrl &path,bool includeSource) {
    if(m_closing || !m_ready || m_brushFileBusy || !path.isLocalFile() || path.toLocalFile().isEmpty())return false;
    const auto package=m_brushLibrary->exportSelectionPackage(includeSource);
    if(package.isEmpty()){showError(QStringLiteral("无法导出选中项，请检查选择、来源选项或画笔容量"));return false;}
    m_brushFileBusy=true;emit brushFileChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,path=path.toLocalFile(),package]{worker->brushFile(path,package,true);},Qt::QueuedConnection);return true;
}
bool PaintCoreClient::saveDocument(const QUrl &path, int format) { return submitFile(path, PAINT_FILE_SAVE, format); }
bool PaintCoreClient::submitFile(const QUrl &path, int kind, int format) {
    if (!m_ready || m_layerMoveActive || m_liquifyActive || m_transformActive || m_drawing || m_fileBusy || m_closing) return false;
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
QVariantMap PaintCoreClient::renderSettings() const {
    auto settings=m_settingsFile.isEmpty()?std::make_unique<QSettings>(QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse"):std::make_unique<QSettings>(m_settingsFile,QSettings::IniFormat);
    return {{"memoryMiB",std::clamp(settings->value("performance/memoryMiB",defaultMemoryMiB()).toInt(),512,1048576)},
            {"gpuMiB",std::clamp(settings->value("performance/gpuMiB",0).toInt(),0,1048576)},
            {"highestQuality",settings->value("performance/highestQuality",false).toBool()}};
}
bool PaintCoreClient::saveRenderSettings(int memoryMiB,int gpuMiB,bool highestQuality) {
    if(memoryMiB<512 || memoryMiB>1048576 || gpuMiB<0 || gpuMiB>1048576)return false;
    auto settings=m_settingsFile.isEmpty()?std::make_unique<QSettings>(QSettings::IniFormat,QSettings::UserScope,"DrawVerse","DrawVerse"):std::make_unique<QSettings>(m_settingsFile,QSettings::IniFormat);
    settings->setValue("performance/memoryMiB",memoryMiB);settings->setValue("performance/gpuMiB",gpuMiB);settings->setValue("performance/highestQuality",highestQuality);settings->sync();
    if(settings->status()!=QSettings::NoError)return false;
    if(m_highestCanvasQuality!=highestQuality){m_highestCanvasQuality=highestQuality;emit stateChanged();}
    emit storageChanged();return true;
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
    if(kind!=PAINT_TRANSFORM_UPDATE && kind!=PAINT_TRANSFORM_COMMIT && kind!=PAINT_TRANSFORM_CANCEL && kind!=PAINT_TRANSFORM_CHECKPOINT && !selectedContentVisible())return false;
    InteractionTrace::Scope trace("transform.submit");
    InteractionTrace::point("transform.submit.kind",kind);
    if(!m_ready || m_closing || m_fileBusy || m_drawing || (kind==PAINT_TRANSFORM_BEGIN && (m_liquifyActive || m_transformActive || layerEditBusy())))return false;
    uint64_t sequence=0;PaintStatus status;QString error;
    {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;if(kind==PAINT_TRANSFORM_BEGIN){QList<quint64> ids=m_selectedLayers;if(ids.isEmpty())ids={m_active};QVector<uint64_t> targets;for(auto id:ids)targets.append(id);status=(residentGpuPreviewEnabled() && !m_compositionTransform && !m_compositionTransformCpu?paint_session_begin_transform_layers_deferred:paint_session_begin_transform_layers)(m_connection->core,m_connection->session,targets.data(),targets.size(),&sequence);}else status=paint_session_transform(m_connection->core,m_connection->session,kind,geometry,&sequence);if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){
        // Invalid intermediate handles keep the last valid geometry; the next move can recover.
        if(kind!=PAINT_TRANSFORM_UPDATE || status!=PAINT_INVALID_ARGUMENT || !error.contains("degenerate or crossed transform"))showError(error);
        return false;
    }m_lastTransform=sequence;
    if(trace.active)trace.event.value=double(sequence);
    if(kind!=PAINT_TRANSFORM_UPDATE)m_pendingTransform=sequence;
    if(kind==PAINT_TRANSFORM_BEGIN){m_transformClosing=false;m_transformPoints.clear();m_transformInitialPoints.clear();m_transformPublished=false;m_transformActive=true;m_residentPreviewFloor=m_revision+1;m_residentPreview.reset();emit transformChanged();}
    if(kind==PAINT_TRANSFORM_COMMIT || kind==PAINT_TRANSFORM_RASTERIZE || kind==PAINT_TRANSFORM_FLIP_IMAGE_HORIZONTAL || kind==PAINT_TRANSFORM_FLIP_IMAGE_VERTICAL)m_pendingMutation=sequence;
    if(kind==PAINT_TRANSFORM_FLIP_IMAGE_HORIZONTAL || kind==PAINT_TRANSFORM_FLIP_IMAGE_VERTICAL)m_pendingLayer=sequence;
    if(kind==PAINT_TRANSFORM_UPDATE && !m_compositionTransform)
        QMetaObject::invokeMethod(m_worker,[worker=m_worker]{worker->transformInputActivity();},Qt::QueuedConnection);
    if(m_compositionTransform && kind!=PAINT_TRANSFORM_UPDATE){
        if(kind==PAINT_TRANSFORM_BEGIN)m_compositionBeginSequence=sequence;
        QMetaObject::invokeMethod(m_worker,[worker=m_worker,sequence]{worker->watchCompositionAcknowledgement(sequence);},Qt::QueuedConnection);
    }
    // UPDATE changes geometry only; transformChanged is emitted by the caller.
    // Avoid invalidating every state-bound QML control on each pointer sample.
    if(kind!=PAINT_TRANSFORM_UPDATE)emit stateChanged();return true;
}
bool PaintCoreClient::beginTransform(){
    if(!selectedContentVisible())return false;
    clearLayerBlendPreview();const bool planned=m_compositionPacket && gpuCompositionEnabled();
    if(const auto device=m_transformPreviewDevice.lock()){
        const auto budget=CompositionBudget::gpuBytes.load(),reserved=CompositionBudget::gpuReservedBytes.load();
        setTransformPreviewLimits(device->maxTextureEdge.load(),budget-std::min(budget,reserved));
    }
    m_transformMeshPreview=preferTransformMeshPreview();
    m_connection->transformMeshAssets.store(m_transformMeshPreview);
    if(!m_transformMeshPreview)prepareCompositionTransform();
    m_compositionTransformCpu=planned && !m_compositionTransform && !m_transformMeshPreview;
    if(submitTransform(PAINT_TRANSFORM_BEGIN))return true;
    if(m_compositionTransform)clearCompositionEdit();m_compositionTransformCpu=false;m_transformMeshPreview=false;m_connection->transformMeshAssets.store(false);return false;
}
bool PaintCoreClient::updateTransform(const QVariantList &points,bool warp,int interpolation,qreal skewH,qreal skewV){if(!m_transformActive || m_transformClosing || !m_transformPublished || points.size()!=16)return false;auto g=dto<PaintTransform>();g.warp=warp;g.interpolation=interpolation;g.skew_h=qIsFinite(skewH)?skewH:m_transformSkewH;g.skew_v=qIsFinite(skewV)?skewV:m_transformSkewV;for(int i=0;i<16;++i){const auto p=points[i].toPointF();g.points[i*2]=p.x();g.points[i*2+1]=p.y();}if(!submitTransform(PAINT_TRANSFORM_UPDATE,&g))return false;m_transformPoints=points;m_transformWarp=warp;m_transformInterpolation=interpolation;m_transformSkewH=g.skew_h;m_transformSkewV=g.skew_v;refreshCompositionTransform(&g);refreshTransformMeshPreview();if(m_transformMeshPreview)emit frameChanged();emit transformChanged();return true;}
bool PaintCoreClient::checkpointTransform(){return m_transformActive && !m_transformClosing && submitTransform(PAINT_TRANSFORM_CHECKPOINT);}
bool PaintCoreClient::finishTransform(bool commit){
    if(!m_transformActive || m_transformClosing || !submitTransform(commit?PAINT_TRANSFORM_COMMIT:PAINT_TRANSFORM_CANCEL))return false;
    m_transformClosing=true;emit transformChanged();
    if(!commit && m_compositionTransform){auto frame=std::make_shared<CompositionPacket>(*m_compositionEditBase);frame->preview=true;frame->geometrySequence=++m_compositionGeometry;frame->editSequence=m_compositionBeginSequence;m_compositionEditFrame=std::move(frame);emit frameChanged();}
    return true;
}
bool PaintCoreClient::resizeCanvas(int x,int y,int width,int height,int dpi,bool deletePixels,bool selectedColor){
    if(!m_ready || m_closing || m_fileBusy || m_drawing || layerEditBusy() || m_transformActive || m_liquifyActive)return false;
    clearLayerBlendPreview();clearError();auto request=dto<PaintCanvasResize>();request.x=x;request.y=y;request.width=width>0?uint32_t(width):0;request.height=height>0?uint32_t(height):0;request.dpi=dpi>0?uint32_t(dpi):0;request.flags=deletePixels?1u:0u;
    const QColor color=selectedColor?m_color:QColor(Qt::white);request.fill[0]=linear(color.redF());request.fill[1]=linear(color.greenF());request.fill[2]=linear(color.blueF());request.fill[3]=color.alphaF();
    uint64_t sequence=0;PaintStatus status;QString error;{QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;status=paint_session_resize_canvas(m_connection->core,m_connection->session,&request,&sequence);if(status!=PAINT_OK)error=immediateError(status);}
    if(status!=PAINT_OK){showError(error);return false;}m_pendingCanvasResize=sequence;m_pendingLayer=sequence;m_pendingMutation=sequence;m_modified=true;emit stateChanged();return true;
}
bool PaintCoreClient::flipImage(bool horizontal){if(m_transformActive || m_liquifyActive || layerEditBusy())return false;clearLayerBlendPreview();return submitTransform(horizontal?PAINT_TRANSFORM_FLIP_IMAGE_HORIZONTAL:PAINT_TRANSFORM_FLIP_IMAGE_VERTICAL);}
bool PaintCoreClient::rasterizeImage(){return !m_transformActive && submitTransform(PAINT_TRANSFORM_RASTERIZE);}

bool PaintCoreClient::deferTransformAction(std::function<void()> action){if(!m_transformActive)return false;m_afterTransform=std::move(action);if(!finishTransform(true))m_afterTransform={};return true;}

QVariantList PaintCoreClient::pressureCurveSamples(const QVariantList &points,bool smooth,const QVariantList &nodeModes) const {
    const auto json=nodeModes.isEmpty()?QJsonDocument(QJsonArray::fromVariantList(points)).toJson(QJsonDocument::Compact):QJsonDocument(QJsonObject{{"curve",QJsonArray::fromVariantList(points)},{"nodeModes",QJsonArray::fromVariantList(nodeModes)}}).toJson(QJsonDocument::Compact);
    float samples[257]{};
    if(paint_pressure_curve_samples(reinterpret_cast<const uint8_t*>(json.constData()),uint64_t(json.size()),nodeModes.isEmpty()?(smooth?1u:0u):2u,samples)!=PAINT_OK)return {};
    QVariantList result;result.reserve(257);for(float value:samples)result.append(value);return result;
}

bool PaintCoreClient::cleanWetBrush(){return wetBrushAction(false);}
bool PaintCoreClient::loadWetBrush(){return wetBrushAction(true);}
bool PaintCoreClient::wetBrushAction(bool reload){
    if(m_closing || !m_ready || m_drawing || m_wetActionBusy || m_liquifyActive || m_transformActive || !m_brushLibrary->wetMix().value("enabled").toBool())return false;
    m_brushLibrary->flushPersistence();
    if(!synchronizeBrushCatalog(m_brushLibrary->snapshot()))return false;
    quint64 preparation=0;{QMutexLocker lock(&m_brushCatalogConnection->mutex);preparation=m_brushCatalogConnection->preparation;}
    const auto id=m_brushLibrary->selectedId();const auto color=m_color;const auto background=m_secondaryColor;const auto radius=m_brushLibrary->radius();const auto after=m_pendingMutation;m_wetActionBusy=true;emit wetActionChanged();
    QMetaObject::invokeMethod(m_worker,[worker=m_worker,reload,id,preparation,color,background,radius,after]{worker->queueWetAction(reload,id,preparation,color,background,radius,after);},Qt::QueuedConnection);return true;
}

bool PaintCoreClient::beginAdjustment(int kind) {
 if(!layerContentVisible(m_active))return false;
 if(kind<0 || kind>2 || !m_ready || m_closing || m_drawing || m_fileBusy || layerEditBusy() || m_transformActive || m_liquifyActive)return false;
 clearLayerBlendPreview();uint64_t sequence=0;PaintStatus status;
 {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;status=paint_session_adjustment(m_connection->core,m_connection->session,0,kind,nullptr,&sequence);}
 if(status!=PAINT_OK){showError(immediateError(status));return false;}
 m_adjustmentBase.reset();m_adjustmentFrame.reset();m_adjustmentSettings.reset();m_adjustmentFinalRevision=0;
 m_adjustmentCommitRequested=m_adjustmentMaterialized=false;
 if(!selectionEnabled() && !m_compositionEditBase && m_compositionDeviceAvailable && gpuCompositionEnabled() && m_compositionPacket &&
    m_compositionPacket->info.key.revision==m_revision && m_compositionPacket->info.key.document_generation==m_generation){
    for(int i=0;i<m_compositionPacket->nodes.size();i++){const auto &n=m_compositionPacket->nodes[i];if(n.layer_id==m_active && n.kind==PAINT_COMPOSITION_RASTER){
        m_adjustmentBase=m_compositionPacket;auto adjustment=std::make_shared<CompositionAdjustment>();adjustment->node=i;adjustment->neutral=true;
        auto next=std::make_shared<CompositionPacket>(*m_adjustmentBase);next->adjustment=std::move(adjustment);next->preview=true;next->geometrySequence=++m_compositionGeometry;m_adjustmentFrame=std::move(next);break;
    }}
 }
 m_adjustmentActive=true;m_adjustmentFailed=false;m_pendingAdjustment=sequence;emit stateChanged();emit frameChanged();return true;
}
bool PaintCoreClient::updateAdjustmentControls(int kind,const QVariantMap &values){
 if(!m_adjustmentActive || m_pendingAdjustment || kind<0 || kind>2)return false;
 auto settings=std::make_unique<PaintAdjustment>();settings->struct_size=sizeof(PaintAdjustment);settings->kind=kind;
 const bool neutral=values.value("neutral").toBool();
 if(kind==2 && !neutral){
    const double hue=values.value("hue").toDouble(),sat=values.value("saturation").toDouble(),light=values.value("lightness").toDouble();
    if(!std::isfinite(hue) || std::abs(hue)>360 || !std::isfinite(sat) || std::abs(sat)>100 || !std::isfinite(light) || std::abs(light)>100)return false;
    settings->parameters[0]=hue/360.;settings->parameters[1]=sat/100.;settings->parameters[2]=light/100.;settings->parameters[3]=values.value("colorize").toBool()?1.f:0.f;
 }
 std::array<std::array<float,257>,4> samples{};
 if(kind==0 && !neutral){
    const auto curves=values.value("curves").toList(),modes=values.value("modes").toList();if(curves.size()!=4 || modes.size()!=4)return false;
    for(int c=0;c<4;c++){
        const auto points=curves[c].toList(),nodeModes=modes[c].toList();
        const auto json=nodeModes.isEmpty()?QJsonDocument(QJsonArray::fromVariantList(points)).toJson(QJsonDocument::Compact):QJsonDocument(QJsonObject{{"curve",QJsonArray::fromVariantList(points)},{"nodeModes",QJsonArray::fromVariantList(nodeModes)}}).toJson(QJsonDocument::Compact);
        if(paint_pressure_curve_samples(reinterpret_cast<const uint8_t*>(json.constData()),uint64_t(json.size()),nodeModes.isEmpty()?1u:2u,samples[c].data())!=PAINT_OK)return false;
    }
 }
 std::array<double,5> levels{0,255,1,0,255};
 if(kind==1 && !neutral){const auto list=values.value("levels").toList();if(list.size()!=5)return false;for(int i=0;i<5;i++){levels[i]=list[i].toDouble();if(!std::isfinite(levels[i]))return false;}
    if(levels[0]<0 || levels[1]>255 || levels[0]>=levels[1] || levels[2]<.01 || levels[2]>10 || levels[3]<0 || levels[3]>255 || levels[4]<0 || levels[4]>255)return false;
 }
 const auto sample=[](const auto &table,double x){const double f=std::clamp(x,0.,1.)*256;const int a=std::min(255,int(f));return double(table[a])*(1-(f-a))+double(table[a+1])*(f-a);};
 for(int i=0;i<=4096;i++){
    const double x=double(i)/4096.;double r=x,g=x,b=x;
    if(!neutral && kind==0){r=sample(samples[0],sample(samples[1],x));g=sample(samples[0],sample(samples[2],x));b=sample(samples[0],sample(samples[3],x));}
    else if(!neutral && kind==1 && levels!=std::array<double,5>{0,255,1,0,255}){const double v=std::pow(std::clamp((x*255-levels[0])/(levels[1]-levels[0]),0.,1.),1/levels[2]);r=g=b=(levels[3]+(levels[4]-levels[3])*v)/255;}
    settings->lut[i][0]=float(r);settings->lut[i][1]=float(g);settings->lut[i][2]=float(b);settings->lut[i][3]=1.f;
 }
 return applyAdjustmentSettings(std::move(settings));
}
bool PaintCoreClient::updateAdjustment(int kind,const QVariantList &lut,qreal hue,qreal saturation,qreal lightness,bool colorize) {
 if(!m_adjustmentActive || m_pendingAdjustment || kind<0 || kind>2 || lut.size()!=4097 || !std::isfinite(hue) || std::abs(hue)>360 || !std::isfinite(saturation) || std::abs(saturation)>100 || !std::isfinite(lightness) || std::abs(lightness)>100 || (colorize && kind!=2))return false;
 auto settings=std::make_unique<PaintAdjustment>();settings->struct_size=sizeof(PaintAdjustment);settings->kind=kind;
 settings->parameters[0]=hue/360.;settings->parameters[1]=saturation/100.;settings->parameters[2]=lightness/100.;settings->parameters[3]=colorize?1.f:0.f;
 for(int i=0;i<4097;i++){const auto row=lut[i].toList();if(row.size()!=3)return false;for(int c=0;c<3;c++){const float v=row[c].toFloat();if(!std::isfinite(v) || v<0 || v>1)return false;settings->lut[i][c]=v;}settings->lut[i][3]=1.;}
 return applyAdjustmentSettings(std::move(settings));
}
bool PaintCoreClient::applyAdjustmentSettings(std::unique_ptr<PaintAdjustment> settings){
 const int kind=int(settings->kind);const bool colorize=settings->parameters[3]==1.f;
 // Local preview is immutable and touches the target before masks/blending.
 if(m_adjustmentBase){
    auto adjustment=std::make_shared<CompositionAdjustment>();adjustment->kind=kind==2?1:0;
    std::copy_n(settings->parameters,4,adjustment->parameters.begin());
    adjustment->lut=QByteArray(reinterpret_cast<const char*>(settings->lut),sizeof(settings->lut));
    for(int i=0;i<m_adjustmentBase->nodes.size();i++)if(m_adjustmentBase->nodes[i].layer_id==m_active && m_adjustmentBase->nodes[i].kind==PAINT_COMPOSITION_RASTER){adjustment->node=i;break;}
    auto next=std::make_shared<CompositionPacket>(*m_adjustmentBase);bool neutral=!colorize;
    if(kind==2)neutral=neutral && settings->parameters[0]==0 && settings->parameters[1]==0 && settings->parameters[2]==0;
    else for(int i=0;i<4097 && neutral;i++)for(int c=0;c<3;c++)if(settings->lut[i][c]!=float(i)/4096.f)neutral=false;
    adjustment->neutral=neutral;next->adjustment=std::move(adjustment);
    next->preview=true;next->geometrySequence=++m_compositionGeometry;m_adjustmentFrame=std::move(next);
    const bool failed=m_adjustmentFailed;m_adjustmentSettings=std::move(settings);m_adjustmentMaterialized=false;m_adjustmentFailed=false;if(failed)emit stateChanged();emit frameChanged();return true;
 }
 uint64_t sequence=0;PaintStatus status;
 {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;status=paint_session_adjustment(m_connection->core,m_connection->session,1,kind,settings.get(),&sequence);}
 if(status!=PAINT_OK){showError(immediateError(status));return false;}
 m_pendingAdjustment=sequence;emit stateChanged();return true;
}
bool PaintCoreClient::finishAdjustment(bool commit) {
 if(!m_adjustmentActive || m_pendingAdjustment)return false;uint64_t sequence=0;PaintStatus status;
 if(commit && m_adjustmentBase && m_adjustmentSettings && !m_adjustmentMaterialized){
    {QMutexLocker lock(&m_connection->mutex);status=paint_session_adjustment(m_connection->core,m_connection->session,1,m_adjustmentSettings->kind,m_adjustmentSettings.get(),&sequence);}
    if(status!=PAINT_OK){showError(immediateError(status));return false;}
    m_adjustmentMaterialized=true;m_adjustmentCommitRequested=true;m_pendingAdjustment=sequence;emit stateChanged();return true;
 }
 m_adjustmentCommitRequested=commit;
 {QMutexLocker lock(&m_connection->mutex);if(!m_connection->session)return false;status=paint_session_adjustment(m_connection->core,m_connection->session,commit?2:3,0,nullptr,&sequence);}
 if(status!=PAINT_OK){showError(immediateError(status));return false;}
 m_pendingAdjustment=sequence;if(commit)m_pendingMutation=sequence;emit stateChanged();return true;
}
