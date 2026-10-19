#include "canvas/SelectionOverlay.h"
#include <QElapsedTimer>
#include <QPainter>
#include <QtTest>
#include <cmath>

class SelectionTests: public QObject {
    Q_OBJECT
private slots:
    void diagonalsGrayEdgesAndCombinations() {
        SelectionOverlay overlay;overlay.setDocumentRect({0,0,32,32});
        const auto step=[](int op,int shape,QRectF rect,QByteArray mask=QByteArray{}) {
            return QVariantMap{{"operation",op},{"shape",shape},{"x",rect.x()},{"y",rect.y()},{"width",rect.width()},{"height",rect.height()},{"mask",mask}};
        };
        QByteArray mask(32*32,'\0');quint32 random=1;
        for(int i=0;i<mask.size();++i){random=random*1664525+1013904223;mask[i]=char(random>>24);}
        const QVariantList base{step(0,3,{0,0,32,32},mask)};
        overlay.setSteps(base);
        for(int y=0;y<32;++y)for(int x=0;x<32;++x)
            QCOMPARE(overlay.documentPath().contains({x+.5,y+.5}),static_cast<unsigned char>(mask[y*32+x])>=128);
        for(int op:{1,2,3,4}) {
            overlay.setSteps(base+QVariantList{step(op,0,{8,8,16,16})});
            const auto path=overlay.documentPath();
            for(int y=0;y<32;++y)for(int x=0;x<32;++x){
                const bool selected=static_cast<unsigned char>(mask[y*32+x])>=128;
                const bool rect=x>=8 && x<24 && y>=8 && y<24;
                const bool expected=op==1?selected||rect:op==2?selected&&!rect:op==3?selected&&rect:!selected;
                QCOMPARE(path.contains({x+.5,y+.5}),expected);
            }
        }
    }
    void complexMaskBoundary() {
        const int w=256,h=192;
        QByteArray mask(w*h,'\0');
        for(int y=0;y<h;++y)for(int x=0;x<w;++x)
            if((x%7<5 && y%7<5) && !(x%7==2 && y%7==2))mask[y*w+x]=char(255);
        SelectionOverlay overlay;overlay.setDocumentRect({0,0,w,h});
        QElapsedTimer timer;timer.start();
        overlay.setSteps({QVariantMap{{"operation",0},{"shape",3},{"x",0},{"y",0},{"width",w},{"height",h},{"mask",mask}}});
        qInfo("complex mask build: %.3f ms, %d elements",timer.nsecsElapsed()/1e6,overlay.documentPath().elementCount());
        const auto path=overlay.documentPath();
        for(int y=0;y<h;++y)for(int x=0;x<w;++x)
            QCOMPARE(path.contains({x+.5,y+.5}),static_cast<unsigned char>(mask[y*w+x])>=128);
    }
    void denseLassoAndClip() {
        QVariantList points;
        for(int i=0;i<4096;++i){const auto a=i*6.283185307179586/4096.;const auto r=80.+(i%2)*10.;points.append(QPointF(128+r*std::cos(a),128+r*std::sin(a)));}
        SelectionOverlay overlay;overlay.setDocumentRect({0,0,256,256});
        QElapsedTimer timer;timer.start();overlay.setSteps({QVariantMap{{"operation",0},{"shape",2},{"points",points}}});
        qInfo("dense lasso build: %.3f ms, %d elements",timer.nsecsElapsed()/1e6,overlay.documentPath().elementCount());
        QVERIFY(overlay.documentPath().contains({128,128}));QVERIFY(!overlay.documentPath().contains({8,8}));
        overlay.setSteps({QVariantMap{{"operation",0},{"shape",2},{"points",QVariantList{QPointF(-10,-10),QPointF(60,-10),QPointF(60,60),QPointF(-10,60)}}}});
        QVERIFY(overlay.documentPath().contains({1,1}));QVERIFY(!overlay.documentPath().contains({-1,1}));
    }
};
QTEST_MAIN(SelectionTests)
#include "selection_tests.moc"
