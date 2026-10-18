import QtQuick
import QtQuick.Controls
import QtQuick.Window
import QtQuick.Shapes
import DrawVerse 1.0
import "../theme"
Rectangle {
    id:root
    property var ownClient: PaintClient
    property string documentId: ""
    property bool initialDocument: false
    property var parkingParent
    property var mainWindow
    property bool savingBlocked: false
    property alias canvasView:canvas
    color:Theme.background;clip:true
    function park(){parent=parkingParent;visible=false}
    function attach(host){parent=host;width=Qt.binding(()=>host.width);height=Qt.binding(()=>host.height);visible=true;canvas.forceActiveFocus()}
    TransparencyGrid {objectName:"canvasTransparency";x:canvas.documentRect.x;y:canvas.documentRect.y;width:canvas.documentRect.width;height:canvas.documentRect.height}
    PaintCanvas {id:canvas;initialFitRatio:.76;objectName:root.initialDocument?"mainCanvas":"documentCanvas:"+root.documentId;anchors.fill:parent;client:root.ownClient;focus:true;enabled:!root.savingBlocked}
    Image {
        objectName:"secondaryBrushCursorImage";z:8
        x:canvas.secondaryCursorRect.x;y:canvas.secondaryCursorRect.y+(canvas.secondaryCursorRect.height-height)/2
        width:canvas.secondaryCursorRect.width;height:canvas.secondaryCursorRect.height*canvas.secondaryCursorRoundness
        rotation:canvas.secondaryCursorAngle
        opacity:root.ownClient.brushLibrary.cursorImageOpacity/100
        visible:canvas.secondaryCursorVisible && root.ownClient.brushLibrary.cursorImage && !root.ownClient.brushLibrary.cursorSimple && !root.ownClient.liquifyActive && root.ownClient.cursorSecondaryTipImage.length>0
        source:visible?root.ownClient.cursorSecondaryTipImage:""
        cache:false;smooth:true
    }
    Image {
        objectName:"brushCursorImage";z:8.5
        x:canvas.brushCursorRect.x;y:canvas.brushCursorRect.y+(canvas.brushCursorRect.height-height)/2
        width:canvas.brushCursorRect.width;height:canvas.brushCursorRect.height*canvas.brushCursorRoundness
        rotation:canvas.brushCursorAngle
        opacity:root.ownClient.brushLibrary.cursorImageOpacity/100
        visible:canvas.brushCursorVisible && root.ownClient.brushLibrary.cursorImage && !root.ownClient.brushLibrary.cursorSimple && !root.ownClient.liquifyActive && root.ownClient.cursorTipImage.length>0
        source:visible?root.ownClient.cursorTipImage:""
        cache:false;smooth:true
    }
    Shape {
        objectName:"secondaryBrushCursorOutline";z:9
        id:secondCursor
        readonly property bool imageOutline:!root.ownClient.liquifyActive && !root.ownClient.brushLibrary.cursorSimple && root.ownClient.cursorSecondaryHasImage && root.ownClient.cursorSecondaryOutlineReady
        readonly property bool simplified:root.ownClient.liquifyActive || root.ownClient.brushLibrary.cursorSimple || root.ownClient.cursorSecondaryHasImage && !root.ownClient.cursorSecondaryOutlineReady
        readonly property real aspect:simplified?1:canvas.secondaryCursorRoundness
        x:canvas.secondaryCursorRect.x;y:canvas.secondaryCursorRect.y+(canvas.secondaryCursorRect.height-height)/2
        width:canvas.secondaryCursorRect.width;height:canvas.secondaryCursorRect.height*aspect
        rotation:simplified?0:canvas.secondaryCursorAngle
        visible:canvas.secondaryCursorVisible && canvas.secondaryCursorRect.width>0 && (root.ownClient.liquifyActive || root.ownClient.brushLibrary.cursorOutline)
        ShapePath {
            strokeWidth:secondCursor.imageOutline?-1:root.ownClient.liquifyActive?1:root.ownClient.brushLibrary.cursorWidth;strokeColor:root.ownClient.liquifyActive?"black":root.ownClient.brushLibrary.cursorColor;fillColor:"transparent"
            PathAngleArc {centerX:secondCursor.width/2;centerY:secondCursor.height/2;radiusX:secondCursor.width/2;radiusY:secondCursor.height/2;startAngle:0;sweepAngle:360}
        }
        ShapePath {
            strokeWidth:secondCursor.imageOutline?-1:root.ownClient.liquifyActive?1:root.ownClient.brushLibrary.cursorWidth;strokeColor:root.ownClient.liquifyActive?"white":root.ownClient.brushLibrary.cursorSecondaryColor;fillColor:"transparent"
            PathAngleArc {centerX:secondCursor.width/2;centerY:secondCursor.height/2;radiusX:Math.max(.5,secondCursor.width/2-(root.ownClient.liquifyActive?1:root.ownClient.brushLibrary.cursorWidth));radiusY:Math.max(.5,secondCursor.height/2-(root.ownClient.liquifyActive?1:root.ownClient.brushLibrary.cursorWidth));startAngle:0;sweepAngle:360}
        }
        ShapePath {
            objectName:"secondaryImageBrushOuterPath"
            scale:Qt.size(secondCursor.width,secondCursor.height)
            strokeWidth:secondCursor.imageOutline?root.ownClient.brushLibrary.cursorWidth+2:-1;strokeColor:root.ownClient.brushLibrary.cursorColor;fillColor:"transparent"
            capStyle:ShapePath.RoundCap
            PathSvg {path:secondCursor.imageOutline?root.ownClient.cursorSecondaryOutlinePath:""}
        }
        ShapePath {
            objectName:"secondaryImageBrushInnerPath"
            scale:Qt.size(secondCursor.width,secondCursor.height)
            strokeWidth:secondCursor.imageOutline?root.ownClient.brushLibrary.cursorWidth:-1;strokeColor:root.ownClient.brushLibrary.cursorSecondaryColor;fillColor:"transparent"
            capStyle:ShapePath.RoundCap
            PathSvg {path:secondCursor.imageOutline?root.ownClient.cursorSecondaryOutlinePath:""}
        }
    }
    Shape {
        objectName:"brushCursorOutline";z:10
        id:tipCursor
        readonly property bool imageOutline:!root.ownClient.liquifyActive && !root.ownClient.brushLibrary.cursorSimple && root.ownClient.cursorHasImage && root.ownClient.cursorOutlineReady
        readonly property bool simplified:root.ownClient.liquifyActive || root.ownClient.brushLibrary.cursorSimple || root.ownClient.cursorHasImage && !root.ownClient.cursorOutlineReady
        readonly property real aspect:simplified?1:canvas.brushCursorRoundness
        x:canvas.brushCursorRect.x;y:canvas.brushCursorRect.y+(canvas.brushCursorRect.height-height)/2
        width:canvas.brushCursorRect.width;height:canvas.brushCursorRect.height*aspect
        rotation:simplified?0:canvas.brushCursorAngle
        visible:canvas.brushCursorVisible && canvas.brushCursorRect.width>0 && (root.ownClient.liquifyActive || root.ownClient.brushLibrary.cursorOutline)
        ShapePath {
            strokeWidth:tipCursor.imageOutline?-1:root.ownClient.liquifyActive?1:root.ownClient.brushLibrary.cursorWidth;strokeColor:root.ownClient.liquifyActive?"black":root.ownClient.brushLibrary.cursorColor;fillColor:"transparent"
            PathAngleArc {centerX:tipCursor.width/2;centerY:tipCursor.height/2;radiusX:tipCursor.width/2;radiusY:tipCursor.height/2;startAngle:0;sweepAngle:360}
        }
        ShapePath {
            strokeWidth:tipCursor.imageOutline?-1:root.ownClient.liquifyActive?1:root.ownClient.brushLibrary.cursorWidth;strokeColor:root.ownClient.liquifyActive?"white":root.ownClient.brushLibrary.cursorSecondaryColor;fillColor:"transparent"
            PathAngleArc {centerX:tipCursor.width/2;centerY:tipCursor.height/2;radiusX:Math.max(.5,tipCursor.width/2-(root.ownClient.liquifyActive?1:root.ownClient.brushLibrary.cursorWidth));radiusY:Math.max(.5,tipCursor.height/2-(root.ownClient.liquifyActive?1:root.ownClient.brushLibrary.cursorWidth));startAngle:0;sweepAngle:360}
        }
        ShapePath {
            objectName:"imageBrushOuterPath"
            scale:Qt.size(tipCursor.width,tipCursor.height)
            strokeWidth:tipCursor.imageOutline?root.ownClient.brushLibrary.cursorWidth+2:-1;strokeColor:root.ownClient.brushLibrary.cursorColor;fillColor:"transparent"
            capStyle:ShapePath.RoundCap
            PathSvg {path:tipCursor.imageOutline?root.ownClient.cursorOutlinePath:""}
        }
        ShapePath {
            objectName:"imageBrushInnerPath"
            scale:Qt.size(tipCursor.width,tipCursor.height)
            strokeWidth:tipCursor.imageOutline?root.ownClient.brushLibrary.cursorWidth:-1;strokeColor:root.ownClient.brushLibrary.cursorSecondaryColor;fillColor:"transparent"
            capStyle:ShapePath.RoundCap
            PathSvg {path:tipCursor.imageOutline?root.ownClient.cursorOutlinePath:""}
        }
    }
    Item {
        objectName:"brushCursorCenterCross";z:11
        x:canvas.cursorPosition.x-5;y:canvas.cursorPosition.y-5;width:10;height:10
        visible:canvas.brushCursorVisible && !root.ownClient.liquifyActive && root.ownClient.brushLibrary.cursorCenter
        Rectangle {x:0;y:4;width:10;height:3;color:"black"}
        Rectangle {x:4;y:0;width:3;height:10;color:"black"}
        Rectangle {x:1;y:5;width:8;height:1;color:"white"}
        Rectangle {x:5;y:1;width:1;height:8;color:"white"}
    }
    Loader {
        objectName:"selectionOutlineLoader";anchors.fill:parent
        active:root.ownClient.selectionEnabled || canvas.selectionPreviewPoints.length>0 || canvas.selectionPreview.width>0 && canvas.selectionPreview.height>0
        sourceComponent:SelectionOutline {objectName:"selectionOutline";enabledSelection:root.ownClient.selectionEnabled;steps:root.ownClient.selectionSteps;documentRect:canvas.documentRect;zoom:canvas.zoom;preview:canvas.selectionPreview;previewKind:canvas.selectionPreviewKind;previewPoints:canvas.selectionPreviewPoints}
    }
    EyedropperRing {
        objectName:"eyedropperSamplingRing";z:10
        x:canvas.cursorPosition.x-width/2;y:canvas.cursorPosition.y-height/2
        visible:canvas.samplingRingVisible
        sampledColor:canvas.sampledColor
        previousColor:canvas.previousSampleColor
    }
    Item {
        objectName:"eyedropperCursor";z:11
        x:canvas.cursorPosition.x-8;y:canvas.cursorPosition.y-8;width:17;height:17
        visible:canvas.eyedropperCursorVisible
        Rectangle {x:0;y:7;width:17;height:3;color:"black"}
        Rectangle {x:7;y:0;width:3;height:17;color:"black"}
        Rectangle {x:1;y:8;width:15;height:1;color:"white"}
        Rectangle {x:8;y:1;width:1;height:15;color:"white"}
    }
    TransformOverlay {id:transformOverlay;anchors.fill:parent;z:15;controller:root.ownClient;canvasView:canvas}
    Shortcut {sequence:"Ctrl+T";context:Qt.ApplicationShortcut;enabled:root.visible && canvas.activeFocus && canvas.Window.window!==root.mainWindow && !root.ownClient.transformActive && !root.ownClient.liquifyActive;onActivated:root.ownClient.beginTransform()}
    Shortcut {sequence:"Escape";context:Qt.ApplicationShortcut;enabled:root.visible && root.ownClient.transformActive;onActivated:root.ownClient.finishTransform(false)}
    Shortcut {sequences:["Return","Enter"];context:Qt.ApplicationShortcut;enabled:root.visible && root.ownClient.transformActive && !root.ownClient.textInputFocused;onActivated:root.ownClient.finishTransform(true)}
    Shortcut {sequence:"Escape";context:Qt.ApplicationShortcut;enabled:root.visible && root.ownClient.liquifyActive && !root.ownClient.drawing;onActivated:root.ownClient.finishLiquify(false)}
    Shortcut {sequences:["Return","Enter"];context:Qt.ApplicationShortcut;enabled:root.visible && !root.ownClient.textInputFocused && root.ownClient.liquifyActive && !root.ownClient.drawing && !root.ownClient.liquifyBusy;onActivated:root.ownClient.finishLiquify(true)}
    Shortcut {sequence:"[";context:Qt.ApplicationShortcut;enabled:root.visible && canvas.activeFocus && !root.ownClient.drawing;onActivated:{if(root.ownClient.liquifyActive)root.ownClient.liquifySize=Math.max(1,Math.min(root.ownClient.liquifySize-1,Math.round(root.ownClient.liquifySize/1.15)));else root.ownClient.brushRadius=Math.max(0.5,root.ownClient.brushRadius/1.15)}}
    Shortcut {sequence:"]";context:Qt.ApplicationShortcut;enabled:root.visible && canvas.activeFocus && !root.ownClient.drawing;onActivated:{if(root.ownClient.liquifyActive)root.ownClient.liquifySize=Math.min(2000,Math.max(root.ownClient.liquifySize+1,Math.round(root.ownClient.liquifySize*1.15)));else root.ownClient.brushRadius=Math.min(256,root.ownClient.brushRadius*1.15)}}
    BusyIndicator {objectName:"canvasBusy";anchors.centerIn:parent;running:!root.ownClient.ready;visible:running}
    Shortcut {sequences:[StandardKey.Undo];context:Qt.ApplicationShortcut;enabled:root.visible && canvas.activeFocus && canvas.Window.window!==root.mainWindow && !root.ownClient.drawing;onActivated:root.ownClient.undo()}
    Shortcut {sequences:["Ctrl+Shift+Z"];context:Qt.ApplicationShortcut;enabled:root.visible && canvas.activeFocus && canvas.Window.window!==root.mainWindow && !root.ownClient.drawing;onActivated:root.ownClient.redo()}
}
