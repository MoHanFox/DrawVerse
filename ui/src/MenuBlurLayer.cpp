#include "MenuBlurLayer.h"
#include <QGuiApplication>
#include <QWindow>
#include <QScopedValueRollback>
#include <algorithm>
#include <bit>
#ifdef Q_OS_WIN
#include <QtCore/qt_windows.h>
#include <dwmapi.h>
#include <DispatcherQueue.h>
#include <winrt/Windows.System.h>
#include <winrt/Windows.UI.Composition.h>
#include <winrt/Windows.UI.Composition.Desktop.h>
#include <windows.ui.composition.interop.h>
namespace {
constexpr auto blurClass=L"DrawVerseMenuBlurLayer";
LRESULT CALLBACK blurWindowProc(HWND window,UINT message,WPARAM wParam,LPARAM lParam) {
    if(message==WM_NCHITTEST)return HTTRANSPARENT;
    if(message==WM_MOUSEACTIVATE)return MA_NOACTIVATE;
    if(message==WM_NCCALCSIZE || message==WM_NCPAINT)return 0;
    if(message==WM_NCACTIVATE)return TRUE;
    if(message==WM_PAINT) {
        PAINTSTRUCT paint{};const auto dc=BeginPaint(window,&paint);
        Q_UNUSED(dc);
        EndPaint(window,&paint);return 0;
    }
    return DefWindowProcW(window,message,wParam,lParam);
}
bool accentBlur(HWND window,bool enabled) {
    struct AccentPolicy {int state,flags;DWORD gradient;int animation;};
    struct CompositionData {int attribute;void *data;SIZE_T size;};
    using Apply=BOOL(WINAPI *)(HWND,const CompositionData *);
    const auto apply=std::bit_cast<Apply>(GetProcAddress(GetModuleHandleW(L"user32.dll"),"SetWindowCompositionAttribute"));
    if(!apply)return false;
    AccentPolicy accent{enabled?3:0,0,0,0};CompositionData data{19,&accent,sizeof(accent)};
    return apply(window,&data)!=FALSE;
}
}
#endif
struct MenuBlurLayer::CompositionState {
#ifdef Q_OS_WIN
    winrt::Windows::UI::Composition::Compositor compositor{nullptr};
    winrt::Windows::UI::Composition::Desktop::DesktopWindowTarget target{nullptr};
    winrt::Windows::UI::Composition::SpriteVisual visual{nullptr};
    explicit CompositionState(HWND window) {
        // Qt supplies the STA and message pump. Keep one dispatcher for the UI
        // thread's lifetime, including teardown/recreation of the main window.
        static winrt::Windows::System::DispatcherQueueController queue=[] {
            winrt::Windows::System::DispatcherQueueController value{nullptr};
            if(!winrt::Windows::System::DispatcherQueue::GetForCurrentThread()) {
                const DispatcherQueueOptions options{sizeof(DispatcherQueueOptions),DQTYPE_THREAD_CURRENT,DQTAT_COM_NONE};
                winrt::check_hresult(CreateDispatcherQueueController(options,reinterpret_cast<PDISPATCHERQUEUECONTROLLER*>(winrt::put_abi(value))));
            }
            return value;
        }();
        compositor=winrt::Windows::UI::Composition::Compositor();
        auto interop=compositor.as<ABI::Windows::UI::Composition::Desktop::ICompositorDesktopInterop>();
        winrt::check_hresult(interop->CreateDesktopWindowTarget(window,FALSE,reinterpret_cast<ABI::Windows::UI::Composition::Desktop::IDesktopWindowTarget**>(winrt::put_abi(target))));
        visual=compositor.CreateSpriteVisual();visual.Brush(compositor.CreateHostBackdropBrush());target.Root(visual);
    }
    void resize(int width,int height,int radius) {
        visual.Size({float(width),float(height)});
        visual.Clip(compositor.CreateRectangleClip(0,0,float(width),float(height),{float(radius),float(radius)},{float(radius),float(radius)},{0,0},{0,0}));
    }
#endif
};
MenuBlurLayer::MenuBlurLayer(QObject *parent):QObject(parent) {}
MenuBlurLayer::~MenuBlurLayer() {clear();}
void MenuBlurLayer::clear() {
    QObject::disconnect(m_destroyed);
    m_composition.reset();
#ifdef Q_OS_WIN
    if(m_handle)DestroyWindow(reinterpret_cast<HWND>(m_handle));
#endif
    m_handle=0;m_window.clear();m_enabled=false;
}
void MenuBlurLayer::hide() {
#ifdef Q_OS_WIN
    if(m_handle)ShowWindow(reinterpret_cast<HWND>(m_handle),SW_HIDE);
#endif
}
bool MenuBlurLayer::setEnabled(QWindow *window,bool enabled,int height) {
#ifdef Q_OS_WIN
    if(!window || QGuiApplication::platformName()!="windows")return false;
    // The application HWND must never own a whole-window accent backdrop.
    if(!accentBlur(reinterpret_cast<HWND>(window->winId()),false))return false;
    if(m_window!=window) {
        clear();m_window=window;
        m_destroyed=connect(window,&QObject::destroyed,this,[this]{clear();});
    }
    m_height=std::clamp(height,1,128);m_enabled=enabled;
    if(!enabled){hide();return true;}
    if(!m_handle) {
        static const bool registered=[] {
            WNDCLASSEXW wc{};wc.cbSize=sizeof(wc);wc.lpfnWndProc=blurWindowProc;
            wc.hInstance=GetModuleHandleW(nullptr);wc.lpszClassName=blurClass;
            return RegisterClassExW(&wc)!=0 || GetLastError()==ERROR_CLASS_ALREADY_EXISTS;
        }();
        if(!registered)return false;
        const auto hwnd=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_NOACTIVATE|WS_EX_TRANSPARENT|WS_EX_NOREDIRECTIONBITMAP,blurClass,L"",WS_POPUP|WS_DISABLED,
            0,0,1,1,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
        if(!hwnd)return false;
        m_handle=reinterpret_cast<quintptr>(hwnd);
        const BOOL host=TRUE;
        if(FAILED(DwmSetWindowAttribute(hwnd,17,&host,sizeof(host)))){clear();return false;}
        try {m_composition=std::make_unique<CompositionState>(hwnd);}catch(const winrt::hresult_error &){clear();return false;}
        // The visual clip, rather than Accent's rectangular HWND backdrop,
        // clips the actual blur. The helper never becomes iconic/maximized.
    }
    sync();return true;
#else
    Q_UNUSED(window);Q_UNUSED(enabled);Q_UNUSED(height);return false;
#endif
}
void MenuBlurLayer::setRadius(int radius) {m_radius=std::clamp(radius,0,64);sync();}
void MenuBlurLayer::sync() {
#ifdef Q_OS_WIN
    if(!m_handle || m_syncing)return;
    QScopedValueRollback<bool> guard(m_syncing,true);
    if(!m_enabled || !m_window){hide();return;}
    const auto main=reinterpret_cast<HWND>(m_window->winId()),blur=reinterpret_cast<HWND>(m_handle);
    if(IsIconic(main) || !IsWindowVisible(main) || m_window->visibility()==QWindow::Minimized || m_window->visibility()==QWindow::Hidden){hide();return;}
    RECT bounds{};if(!GetWindowRect(main,&bounds)){hide();return;}
    const int width=bounds.right-bounds.left,height=qRound(m_height*m_window->devicePixelRatio());
    const int radius=(IsZoomed(main) || m_window->visibility()==QWindow::Maximized || m_window->visibility()==QWindow::FullScreen)?0:qRound(m_radius*m_window->devicePixelRatio());
    const auto region=radius?CreateRoundRectRgn(0,0,width+1,height+1,2*radius,2*radius):CreateRectRgn(0,0,width,height);
    if(!region){hide();return;}
    if(radius) {
        const auto bottom=CreateRectRgn(0,std::min(radius,height),width,height);
        CombineRgn(region,region,bottom,RGN_OR);DeleteObject(bottom);
    }
    if(!SetWindowRgn(blur,region,FALSE)){DeleteObject(region);hide();return;}
    m_composition->resize(width,height,radius);
    SetWindowPos(blur,main,bounds.left,bounds.top,width,height,SWP_NOACTIVATE|SWP_SHOWWINDOW);
#endif
}
