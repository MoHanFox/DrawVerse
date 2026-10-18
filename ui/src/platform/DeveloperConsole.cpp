#include "platform/DeveloperConsole.h"
#include <QtGlobal>
#ifdef Q_OS_WIN
#include <QtCore/qt_windows.h>
#include <QDebug>
#include <cstdio>

namespace {
QtMessageHandler previousMessageHandler=nullptr;
void consoleMessage(QtMsgType type,const QMessageLogContext &context,const QString &message) {
    const auto text=qFormatLogMessage(type,context,message).toUtf8();
    fprintf(stderr,"%s\n",text.constData());
    fflush(stderr);
}
}
#endif

bool setDeveloperConsoleVisible(bool visible) {
#ifdef Q_OS_WIN
    static bool owned=false;
    if(!visible) {
        if(owned) {
            qInstallMessageHandler(previousMessageHandler);
            if(!FreeConsole()){qInstallMessageHandler(consoleMessage);return false;}
            owned=false;
        }
        return true;
    }
    if(owned || GetConsoleWindow())return true;
    if(!AllocConsole())return false;
    owned=true;
    SetConsoleTitleW(L"DrawVerse Developer Console");
    SetConsoleOutputCP(CP_UTF8);
    // Closing a console with its title-bar button would terminate the application.
    if(const auto menu=GetSystemMenu(GetConsoleWindow(),FALSE))DeleteMenu(menu,SC_CLOSE,MF_BYCOMMAND);
    FILE *stream=nullptr;
    if(freopen_s(&stream,"CONOUT$","w",stdout) || freopen_s(&stream,"CONOUT$","w",stderr)) {
        FreeConsole();owned=false;return false;
    }
    previousMessageHandler=qInstallMessageHandler(consoleMessage);
    return true;
#else
    Q_UNUSED(visible);
    return true;
#endif
}
