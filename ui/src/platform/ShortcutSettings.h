#pragma once
#include <QObject>
#include <QSettings>
#include <QKeySequence>
#include <QVariantList>
#include <QCoreApplication>
#include <QDir>

// Stable command IDs are shared by menus, detached canvases and the editor.
class ShortcutSettings final : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantMap bindings READ bindings NOTIFY bindingsChanged)
    Q_PROPERTY(QVariantList commands READ commands NOTIFY changed)
    Q_PROPERTY(bool recording READ recording WRITE setRecording NOTIFY recordingChanged)
public:
    explicit ShortcutSettings(QObject *parent=nullptr, const QString &file={})
        : QObject(parent), settings(createSettings(file,this)) {
        struct Row {const char *id,*category,*label,*key;};
        const Row rows[]{
            {"new", "文档", "新建画布", "Ctrl+N"},
            {"open", "文档", "打开…", "Ctrl+O"},
            {"save", "文档", "保存", "Ctrl+S"},
            {"saveAs", "文档", "另存为 OpenRaster…", "Ctrl+Shift+S"},
            {"undo", "编辑", "撤销", "Ctrl+Z"},
            {"redo", "编辑", "重做", "Ctrl+Shift+Z"},
            {"transform", "编辑", "自由变换", "Ctrl+T"},
            {"curves", "调整", "曲线", "Ctrl+M"},
            {"levels", "调整", "色阶", "Ctrl+L"},
            {"hsv", "调整", "色相／饱和度", "Ctrl+U"},
            {"preferences", "编辑", "首选项…", "Ctrl+K"},
            {"clip", "编辑", "创建／释放剪贴蒙版", "Ctrl+Alt+G"},
            {"copy", "图层", "拷贝图层与组", "Ctrl+C"},
            {"cut", "图层", "剪切图层／选区内容", "Ctrl+X"},
            {"paste", "图层", "粘贴图层与组", "Ctrl+V"},
            {"duplicate", "图层", "复制图层", "Ctrl+J"},
            {"merge", "图层", "向下合并／合并图层／合并组", "Ctrl+E"},
            {"group", "图层", "从图层创建组", "Ctrl+G"},
            {"delete", "图层", "删除图层／当前蒙版", "Delete"},
            {"rectangle", "选择", "矩形选框", "M"},
            {"ellipse", "选择", "椭圆选框", "Shift+M"},
            {"lasso", "选择", "套索工具", "L"},
            {"cycleLasso", "选择", "切换套索子类", "Shift+L"},
            {"selectAll", "选择", "全选", "Ctrl+A"},
            {"deselect", "选择", "取消选择", "Ctrl+D"},
            {"invert", "选择", "反向选择", "Ctrl+Shift+I"},
            {"move", "工具", "移动工具", "V"},
            {"brush", "工具", "画笔", "B"},
            {"bucket", "工具", "油漆桶", "G"},
            {"wand", "工具", "魔棒", "W"},
            {"smudge", "工具", "涂抹", "R"},
            {"eraser", "工具", "橡皮擦", "E"},
            {"picker", "工具", "吸色", "I"},
            {"fit", "视图", "适合窗口", "F"},
            {"smaller", "画布", "减小笔刷", "["},
            {"larger", "画布", "增大笔刷", "]"},
        };
        for(const auto &r:rows) {
            QString key=QString::fromLatin1(r.key);
#ifdef Q_OS_MACOS
            key.replace("Ctrl+","Meta+");
#endif
            key=QKeySequence(key).toString(QKeySequence::PortableText);
            data.append(QVariantMap{{"id",r.id},{"category",QString::fromUtf8(r.category)},{"label",QString::fromUtf8(r.label)},{"defaultKey",key},{"key",key}});
        }
        for(qsizetype i=0;i<data.size();++i) {
            auto row=data[i].toMap();
            const auto id=row["id"].toString();
            const auto stored=settings->value("shortcuts/v1/"+id,row["defaultKey"]).toString();
            // Load all overrides together so swaps survive a restart.
            row["key"]=stored;data[i]=row;
        }
        // New defaults must not invalidate an existing user's explicit assignments.
        for(auto &value:data) {
            auto row=value.toMap();
            if(settings->contains("shortcuts/v1/"+row["id"].toString()))continue;
            for(const auto &otherValue:data) {
                const auto other=otherValue.toMap();
                if(row["id"]!=other["id"] && !row["key"].toString().isEmpty() && row["key"]==other["key"] && settings->contains("shortcuts/v1/"+other["id"].toString())) {
                    row["key"]=QString();value=row;break;
                }
            }
        }
        bool invalid=false;
        for(const auto &value:data) {const auto row=value.toMap();if(!validate(row["id"].toString(),row["key"].toString()).isEmpty())invalid=true;}
        if(invalid)for(auto &value:data){auto row=value.toMap();row["key"]=row["defaultKey"];value=row;}
    }
    QVariantList commands() const {return data;}
    QVariantMap bindings() const {QVariantMap result;for(const auto &v:data){const auto row=v.toMap();result.insert(row["id"].toString(),capturing ? QString() : row["key"]);}return result;}
    bool recording() const {return capturing;}
    void setRecording(bool value){if(capturing!=value){capturing=value;emit recordingChanged();emit bindingsChanged();}}
    Q_INVOKABLE QString binding(const QString &id) const {
        if(capturing)return {};
        for(const auto &v:data){const auto row=v.toMap();if(row["id"]==id)return row["key"].toString();}
        return {};
    }
    Q_INVOKABLE QString fromKey(int key,int modifiers) const {
        if(key==Qt::Key_unknown || key==Qt::Key_Control || key==Qt::Key_Shift || key==Qt::Key_Alt || key==Qt::Key_Meta)return {};
        return QKeySequence(QKeyCombination(Qt::KeyboardModifiers(modifiers & (Qt::ControlModifier|Qt::ShiftModifier|Qt::AltModifier|Qt::MetaModifier)),Qt::Key(key))).toString(QKeySequence::PortableText);
    }
    Q_INVOKABLE QString validate(const QString &id,const QString &text) const {
        if(text.isEmpty())return {};
        const auto seq=QKeySequence::fromString(text,QKeySequence::PortableText);
        if(seq.count()!=1 || seq[0].key()==Qt::Key_unknown || seq.toString(QKeySequence::PortableText)!=text)return QStringLiteral("请输入一个有效的按键组合");
        const int key=seq[0].key();
        if(key==Qt::Key_Escape || key==Qt::Key_Return || key==Qt::Key_Enter || key==Qt::Key_Space || key==Qt::Key_CapsLock || (key>=Qt::Key_Left && key<=Qt::Key_Down) || key==Qt::Key_Backspace || key==Qt::Key_Control || key==Qt::Key_Shift || key==Qt::Key_Alt || key==Qt::Key_Meta)return QStringLiteral("该键保留给画布交互或文本编辑");
        const auto mods=seq[0].keyboardModifiers();
        if((mods & (Qt::ControlModifier|Qt::MetaModifier)) && (key==Qt::Key_Plus || key==Qt::Key_Equal || key==Qt::Key_Minus))return QStringLiteral("该组合保留给画布缩放");
        for(const auto &v:data){const auto row=v.toMap();if(row["id"]!=id && !row["key"].toString().isEmpty() && seq.matches(QKeySequence(row["key"].toString()))!=QKeySequence::NoMatch)return QStringLiteral("已用于：")+row["label"].toString();}
        return {};
    }
    Q_INVOKABLE QString assign(const QString &id,const QString &text) {
        const auto error=validate(id,text);if(!error.isEmpty())return error;
        for(auto &v:data){auto row=v.toMap();if(row["id"]!=id)continue;
            const auto path="shortcuts/v1/"+id;const auto old=settings->value(path);settings->setValue(path,text);settings->sync();
            if(settings->status()!=QSettings::NoError){if(old.isValid())settings->setValue(path,old);else settings->remove(path);return QStringLiteral("快捷键配置保存失败");}
            row["key"]=text;v=row;emit changed();emit bindingsChanged();return {};
        }
        return QStringLiteral("未知命令");
    }
    Q_INVOKABLE QString reset() {
        const auto old=data;
        settings->remove("shortcuts/v1");settings->sync();
        if(settings->status()!=QSettings::NoError){for(const auto &v:old){const auto row=v.toMap();settings->setValue("shortcuts/v1/"+row["id"].toString(),row["key"]);}return QStringLiteral("快捷键配置保存失败");}
        for(auto &v:data){auto row=v.toMap();row["key"]=row["defaultKey"];v=row;}emit changed();emit bindingsChanged();return {};
    }
signals:
    void changed();
    void recordingChanged();
    void bindingsChanged();
private:
    static QSettings *createSettings(QString file,QObject *parent) {
        const auto args=QCoreApplication::arguments();const int index=args.indexOf("--profile-dir");
        if(file.isEmpty() && index>=0 && index+1<args.size())file=QDir(args[index+1]).filePath("DrawVerse.ini");
        return file.isEmpty() ? new QSettings(QSettings::IniFormat,QSettings::UserScope,"DrawVerse",QCoreApplication::applicationName(),parent) : new QSettings(file,QSettings::IniFormat,parent);
    }
    QSettings *settings;
    QVariantList data;
    bool capturing=false;
};
