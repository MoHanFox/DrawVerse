import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "."

ColumnLayout {
    id: root
    objectName: "layersPanel"
    spacing: 3
    property var selected: {
        const layers=PaintClient.layers
        for(let i=0;i<layers.length;i++) if(layers[i].id===PaintClient.activeLayer) return layers[i]
        return {id:0,opacity:1,fill:1,locks:0,effectiveLocks:0,blendMode:0,group:false,mask:false,parent:0}
    }
    property var control: selected.mask ? PaintClient.layers.find(function(l){return l.id===selected.parent}) || selected : selected
    property bool editable: PaintClient.ready && !PaintClient.drawing && !PaintClient.layerEditBusy && !PaintClient.fileBusy
    property bool unlocked: editable && !(control.effectiveLocks & 4)
    property bool parentLocked: {
        let parent=control.parent
        const all=PaintClient.layers
        while(parent!==0) {
            const ancestor=all.find(function(layer) { return layer.id===parent })
            if(!ancestor) return false
            if((ancestor.locks & 4)!==0) return true
            parent=ancestor.parent
        }
        return false
    }
    property var masksByOwner: {
        const map={};PaintClient.layers.forEach(function(l){if(l.mask)map[l.parent]=l});return map
    }
    property var clippingBases: {
        const map={};PaintClient.layers.forEach(function(l){if(l.clipped && l.clipBase)map[l.clipBase]=true});return map
    }
    function lowerSibling(layer) {
        const siblings=PaintClient.layers.filter(function(l){return !l.mask && l.parent===layer.parent})
        const i=siblings.findIndex(function(l){return l.id===layer.id})
        return i>=0 && i+1<siblings.length ? siblings[i+1] : null
    }
    function canClip(layer) { return layer.clipped || lowerSibling(layer)!==null }
    function upperSibling(layer) {
        const siblings=PaintClient.layers.filter(function(l){return !l.mask && l.parent===layer.parent})
        const i=siblings.findIndex(function(l){return l.id===layer.id})
        return i>0 ? siblings[i-1] : null
    }
    property var dragHovered: null
    function dropToRoot(payload) {
        const source=Number(payload.split(":")[1])
        const roots=PaintClient.layers.filter(function(l){return l.parent===0 && !l.mask && l.id!==source})
        return roots.length ? PaintClient.acceptLayerDrop(payload,roots[roots.length-1].id,2) : PaintClient.acceptLayerDrop(payload,0,0)
    }
    function canMoveInto(group) {
        if(group.id===selected.id || group.id===selected.parent || (group.effectiveLocks & 4)!==0) return false
        let parent=group.parent
        const all=PaintClient.layers
        while(parent!==0) {
            if(parent===selected.id) return false
            const ancestor=all.find(function(layer) { return layer.id===parent })
            if(!ancestor) return false
            parent=ancestor.parent
        }
        return true
    }
    TextField {
        id: search
        objectName: "layerSearch"
        Layout.fillWidth: true; implicitHeight: 20
        placeholderText: "搜索图层名称"; placeholderTextColor: Theme.muted; selectByMouse: true
        color: Theme.text; font.pixelSize: 9
        background: Rectangle {color:Theme.input;border.color:Theme.line;radius:2}
    }
    RowLayout {
        Layout.fillWidth: true; spacing: 4
        CompactComboBox {
            id: blend
            objectName: "layerBlendMode"
            Layout.fillWidth: true; Layout.minimumWidth: 48; implicitHeight: 20
            enabled: root.unlocked
            model: ["正常","溶解","变暗","正片叠底","颜色加深","线性加深","深色","变亮","滤色","颜色减淡","线性减淡（添加）","浅色","叠加","柔光","强光","亮光","线性光","点光","实色混合","差值","排除","减去","划分","色相","饱和度","颜色","明度"]
            currentIndex: root.control.blendMode
            onOptionHovered:index=>{if(index>=0)PaintClient.previewLayerBlend(root.control.id,index);else PaintClient.clearLayerBlendPreview()}
            onMenuClosed:PaintClient.clearLayerBlendPreview()
            onActivated: {PaintClient.clearLayerBlendPreview();PaintClient.setLayerBlend(root.control.id,currentIndex)}
            ToolTip.visible: hovered; ToolTip.text: "图层混合模式"
        }
        Label { text: "不透明度"; color: Theme.muted; font.pixelSize: 9 }
        LayerPercent {
            objectName: "layerOpacity"; Layout.preferredWidth: 52
            fraction: root.control.opacity; enabled: root.unlocked
            onCommitted: fraction => PaintClient.setLayerProperties(root.control.id,root.control.visible,fraction)
        }
    }
    RowLayout {
        Layout.fillWidth: true; spacing: 3
        Label { text: "锁定："; color: Theme.muted; font.pixelSize: 9 }
        LayerLock { objectName: "lockTransparency"; kind: 1; selected: (root.control.locks & 1)!==0; enabled: root.editable && !root.parentLocked && !root.control.group; onClicked: PaintClient.setLayerLocks(root.control.id,root.control.locks ^ 1) }
        LayerLock { objectName: "lockPosition"; kind: 2; selected: (root.control.locks & 2)!==0; enabled: root.editable && !root.parentLocked; onClicked: PaintClient.setLayerLocks(root.control.id,root.control.locks ^ 2) }
        LayerLock { objectName: "lockAll"; kind: 4; selected: (root.control.locks & 4)!==0; enabled: root.editable && !root.parentLocked; onClicked: PaintClient.setLayerLocks(root.control.id,root.control.locks ^ 4) }
        Item { Layout.fillWidth: true }
        Label { text: "填充"; color: Theme.muted; font.pixelSize: 9 }
        LayerPercent { objectName: "layerFill"; Layout.preferredWidth: 52; enabled: root.unlocked; fraction: root.control.fill; onCommitted: fraction => PaintClient.setLayerFill(root.control.id,fraction) }
    }
    RowLayout {
        visible:root.selected.mask;Layout.fillWidth:true
        Label {text:"密度";color:Theme.muted;font.pixelSize:10}
        LayerPercent {objectName:"maskDensity";Layout.preferredWidth:73;fraction:root.selected.opacity;enabled:root.unlocked;onCommitted:fraction=>PaintClient.setLayerProperties(root.selected.id,root.selected.visible,fraction)}
    }
    Rectangle { Layout.fillWidth: true; height: 1; color: Theme.line }
    ListView {
        id: layerList
        objectName: "layerList"
        Layout.fillWidth: true; Layout.fillHeight: true; clip: true; spacing: 1; cacheBuffer: 34
        model: {
            const all=PaintClient.layers.filter(function(l){return !l.mask}); const query=search.text.toLowerCase()
            if(query.length>0) return all.filter(function(layer) { return layer.name.toLowerCase().indexOf(query)>=0 })
            const collapsed=PaintClient.collapsedGroups
            if(collapsed.length===0) return all
            const parents={}
            all.forEach(function(layer) { parents[layer.id]=layer.parent })
            return all.filter(function(layer) {
                let parent=layer.parent
                while(parent!==0) { if(collapsed.indexOf(parent)>=0) return false; parent=parents[parent] || 0 }
                return true
            })
        }
        ScrollBar.vertical: ScrollBar {}
        currentIndex: {
            const rows=layerList.model
            for(let i=0;i<rows.length;i++)if(rows[i].id===root.control.id)return i
            return -1
        }
        function ensureSelectedVisible() { if(currentIndex>=0)positionViewAtIndex(currentIndex,ListView.Contain) }
        onCurrentIndexChanged: Qt.callLater(ensureSelectedVisible)
        onHeightChanged: Qt.callLater(ensureSelectedVisible)
        DropArea {
            objectName: "layerEmptyDrop"
            x:0; y:Math.max(0,layerList.contentHeight-layerList.contentY)
            width:layerList.width; height:Math.max(0,layerList.height-y)
            keys:["application/x-drawverse-layer"];enabled:root.editable
            Rectangle {anchors.fill:parent;color:parent.containsDrag?Theme.selected:"transparent"}
            onDropped:d=>{if(root.dropToRoot(d.getDataAsString("application/x-drawverse-layer")))d.acceptProposedAction()}
        }
        delegate:
                Rectangle {
                    id: row
                    required property var modelData
                    objectName: "layerRow:"+modelData.id
                    width: layerList.width; height: 34
                    property var maskNode: root.masksByOwner[modelData.id] || null
                    property bool active: PaintClient.activeLayer===modelData.id || (maskNode!==null && PaintClient.activeLayer===maskNode.id)
                    Component.onCompleted: { if(!modelData.group) PaintClient.requestLayerPreview(modelData.id) }
                    Component.onDestruction: { if(!modelData.group) PaintClient.releaseLayerPreview(modelData.id) }
                    color: active ? Theme.selected : Theme.surface
                    MouseArea {
                        objectName:"layerDragHandle:"+row.modelData.id
                        anchors.fill:parent;property point origin;property bool clippingGesture:false
                        onPressed:mouse=>{
                            origin=Qt.point(mouse.x,mouse.y);clippingGesture=false
                            if(root.editable && (mouse.modifiers & Qt.AltModifier) && mouse.y<5) {
                                const upper=root.upperSibling(row.modelData)
                                if(upper && !(upper.effectiveLocks & 4)){clippingGesture=true;PaintClient.setLayerClipping(upper.id,!upper.clipped)}
                            }
                        }
                        onPositionChanged:mouse=>{if(pressed && !clippingGesture && root.editable && !row.modelData.mask && Math.abs(mouse.x-origin.x)+Math.abs(mouse.y-origin.y)>10) PaintClient.beginLayerDrag(row.modelData.id)}
                        onClicked:if(!clippingGesture)PaintClient.selectLayer(row.modelData.id)
                    }
                    MouseArea {
                        id: clippingBoundary
                        objectName:"clippingBoundary:"+row.modelData.id
                        x:0;y:parent.height-5;width:parent.width;height:6;z:5
                        enabled:root.editable && root.canClip(row.modelData) && !(row.modelData.effectiveLocks & 4)
                        hoverEnabled:true
                        cursorShape:Qt.PointingHandCursor
                        onPressed:mouse=>{
                            if(mouse.modifiers & Qt.AltModifier)PaintClient.setLayerClipping(row.modelData.id,!row.modelData.clipped)
                            else mouse.accepted=false
                        }
                        ToolTip.visible:containsMouse
                        ToolTip.text:row.modelData.clipped ? "Alt / Option 点击释放剪贴蒙版" : "Alt / Option 点击创建剪贴蒙版"
                    }
                    property int dropPlacement: 0
                    property point dropPoint
                    function updateDrop(d) {dropPoint=Qt.point(d.x,d.y);root.dragHovered=row;dropPlacement=modelData.group && d.y>11 && d.y<height-11 ? 0 : d.y<height/2 ? 1:2}
                    DropArea {
                        id:layerDrop;objectName:"layerDrop:"+row.modelData.id
                        anchors.fill:parent;keys:["application/x-drawverse-layer"];enabled:root.editable && !row.modelData.mask
                        onEntered:d=>row.updateDrop(d)
                        onPositionChanged:d=>row.updateDrop(d)
                        onExited:{if(root.dragHovered===row)root.dragHovered=null}
                        onDropped:d=>{root.dragHovered=null;if(PaintClient.acceptLayerDrop(d.getDataAsString("application/x-drawverse-layer"),row.modelData.id,row.dropPlacement))d.acceptProposedAction()}
                    }
                    Rectangle {anchors.fill:parent;color:"transparent";border.width:2;border.color:Theme.accent;visible:layerDrop.containsDrag && row.dropPlacement===0}
                    Rectangle {width:parent.width;height:2;y:row.dropPlacement===1?0:parent.height-2;color:Theme.accent;visible:layerDrop.containsDrag && row.dropPlacement!==0}
                    Timer {interval:600;running:layerDrop.containsDrag && row.dropPlacement===0 && PaintClient.collapsedGroups.indexOf(row.modelData.id)>=0;onTriggered:PaintClient.toggleGroupExpanded(row.modelData.id)}
                    RowLayout {
                        anchors.fill: parent; anchors.leftMargin: 3; anchors.rightMargin: 4; spacing: 3
                        ToolButton {
                            objectName: "layerVisibility:"+row.modelData.id
                            implicitWidth: 20; implicitHeight: 20; enabled: root.editable
                            padding: 2
                            onClicked: PaintClient.setLayerProperties(row.modelData.id,!row.modelData.visible,row.modelData.opacity)
                            ToolTip.visible: hovered; ToolTip.text: row.modelData.visible ? "隐藏图层" : "显示图层"
                            contentItem: Icon {name:"eye";visible:row.modelData.visible}
                        }
                        Item { objectName:"layerIndent:"+row.modelData.id; Layout.preferredWidth: row.modelData.depth*12; height:24
                            Rectangle {visible:row.modelData.depth>0;anchors.right:parent.right;width:1;height:parent.height;color:Theme.line}
                            Rectangle {visible:row.modelData.depth>0;anchors.right:parent.right;anchors.verticalCenter:parent.verticalCenter;width:8;height:1;color:Theme.line}
                        }
                        ToolButton {
                            objectName: "groupExpand:"+row.modelData.id
                            implicitWidth: 16; implicitHeight: 20; visible: row.modelData.group
                            text: PaintClient.collapsedGroups.indexOf(row.modelData.id)>=0 ? "▸" : "▾"
                            onClicked: PaintClient.toggleGroupExpanded(row.modelData.id)
                            ToolTip.visible: hovered; ToolTip.text: "展开 / 折叠图层组"
                        }
                        Label { objectName:"clippingArrow:"+row.modelData.id;visible:row.modelData.clipped;text:"↳";color:Theme.muted;font.pixelSize:17;Layout.preferredWidth:14 }
                        LayerThumbnail {
                            visible:!row.modelData.group
                            node:row.modelData;selected:PaintClient.activeLayer===row.modelData.id
                            onClicked:modifiers=>PaintClient.selectLayer(row.modelData.id)
                        }
                        Item {
                            visible:row.maskNode!==null && !row.modelData.group
                            Layout.preferredWidth:10;implicitHeight:24
                            Canvas {
                                anchors.fill:parent
                                onPaint:{const c=getContext("2d");c.reset();c.strokeStyle=Theme.muted;c.lineWidth=1.3;c.lineCap="round";c.beginPath();c.moveTo(2,12);c.lineTo(2,6);c.quadraticCurveTo(5,2,8,6);c.lineTo(8,12);c.stroke();c.beginPath();c.moveTo(2,12);c.lineTo(2,18);c.quadraticCurveTo(5,22,8,18);c.lineTo(8,12);c.stroke();c.beginPath();c.moveTo(5,8);c.lineTo(5,16);c.stroke()}
                            }
                            ToolTip.visible:linkHover.containsMouse;ToolTip.text:"蒙版随图层一起移动"
                            MouseArea{id:linkHover;anchors.fill:parent;hoverEnabled:true;acceptedButtons:Qt.NoButton}
                        }
                        LayerThumbnail {
                            objectName:"inlineMask:"+row.modelData.id
                            visible:row.maskNode!==null
                            node:row.maskNode;selected:row.maskNode!==null && PaintClient.activeLayer===row.maskNode.id
                            property var requestedMask: 0
                            onNodeChanged: {
                                if(requestedMask)PaintClient.releaseLayerPreview(requestedMask)
                                requestedMask=node ? node.id : 0
                                if(requestedMask)PaintClient.requestLayerPreview(requestedMask)
                            }
                            Component.onDestruction:if(requestedMask)PaintClient.releaseLayerPreview(requestedMask)
                            onClicked:modifiers=>{
                                if(modifiers & Qt.ShiftModifier)PaintClient.setLayerProperties(row.maskNode.id,!row.maskNode.visible,row.maskNode.opacity)
                                else PaintClient.selectLayer(row.maskNode.id)
                            }
                        }
                        Label { objectName:"layerName:"+row.modelData.id; text:row.modelData.name; Layout.fillWidth: true; elide: Text.ElideRight; color: Theme.text; font.pixelSize: 9; font.bold: row.modelData.group; font.underline:root.clippingBases[row.modelData.id]===true }
                        LayerLock { kind: (row.modelData.effectiveLocks & 4) ? 4 : (row.modelData.effectiveLocks & 2) ? 2 : 1; passiveIcon: true; selected: true; visible: row.modelData.effectiveLocks!==0; enabled: false; implicitWidth: 18; implicitHeight: 18 }
                    }
                }
    }
    Timer {
        interval:60;repeat:true;running:root.dragHovered!==null
        onTriggered:{const row=root.dragHovered;if(!row)return;const p=row.mapToItem(layerList,row.dropPoint.x,row.dropPoint.y);const delta=p.y<20?-16:p.y>layerList.height-20?16:0;if(delta)layerList.contentY=Math.max(0,Math.min(Math.max(0,layerList.contentHeight-layerList.height),layerList.contentY+delta))}
    }
    DropArea {
        objectName:"layerRootDrop";Layout.fillWidth:true;Layout.preferredHeight:10
        keys:["application/x-drawverse-layer"];enabled:root.editable
        Rectangle {anchors.fill:parent;color:parent.containsDrag?Theme.selected:"transparent";border.color:parent.containsDrag?Theme.accent:"transparent"}
        Label {anchors.centerIn:parent;text:parent.containsDrag?"移出到顶层底部":"";color:Theme.text;font.pixelSize:10}
        onDropped:d=>{if(root.dropToRoot(d.getDataAsString("application/x-drawverse-layer")))d.acceptProposedAction()}
    }
    RowLayout {
        Label { text: PaintClient.layers.filter(function(l){return !l.mask}).length+" 图层"; font.pixelSize: 9; color: Theme.muted; Layout.fillWidth: true }
        IconButton { objectName:"addLayerMask";glyph:"mask";tooltip:"添加白色蒙版";implicitHeight:28;implicitWidth:28;enabled:root.unlocked && !root.selected.mask && !PaintClient.layers.some(function(l){return l.mask && l.parent===root.selected.id});onClicked:PaintClient.addMask(root.selected.id);ToolTip.visible:hovered;ToolTip.text:"添加白色蒙版（黑隐藏，白显示）" }
        IconButton { objectName: "groupLayer"; glyph: "folder"; tooltip: "新建图层组"; implicitHeight: 20; implicitWidth: 28; enabled: root.unlocked && !root.selected.mask; onClicked: PaintClient.groupLayer(root.selected.id,"组 "+(PaintClient.layers.filter(function(layer) { return layer.group }).length+1)); ToolTip.visible: hovered; ToolTip.text: "将所选图层或组放入新组（隔离合成）" }
        IconButton { objectName: "layerGroupMenu"; glyph: "menu"; tooltip: "图层菜单"; implicitWidth: 28; implicitHeight: 20; enabled: root.unlocked; onClicked: groupMenu.popup() }
        IconButton { glyph: "plus"; tooltip: "新建图层"; implicitWidth: 28; implicitHeight: 20; enabled: root.editable; objectName:"addDefaultLayer";onClicked: PaintClient.addDefaultLayer(); ToolTip.visible: hovered; ToolTip.text: "新建图层" }
        IconButton { objectName: "deleteLayer"; glyph: "trash"; tooltip: "删除图层"; implicitHeight: 20; enabled: root.unlocked && PaintClient.layers.length>1; onClicked: PaintClient.removeLayer(root.selected.id) }
    }
    GlassMenu {
        id: groupMenu
        objectName: "groupOperationsMenu"
        GlassMenuItem { objectName:"toggleLayerClipping";text:root.control.clipped ? "释放剪贴蒙版" : "创建剪贴蒙版";enabled:root.unlocked && root.canClip(root.control);onTriggered:PaintClient.setLayerClipping(root.control.id,!root.control.clipped) }
        GlassMenuItem { objectName:"disableLayerMask";text:root.masksByOwner[root.control.id] && root.masksByOwner[root.control.id].visible ? "停用图层蒙版" : "启用图层蒙版";enabled:root.unlocked && root.masksByOwner[root.control.id]!==undefined;onTriggered:{const m=root.masksByOwner[root.control.id];PaintClient.setLayerProperties(m.id,!m.visible,m.opacity)} }
        GlassMenuItem { objectName:"deleteLayerMask";text:"删除图层蒙版";enabled:root.unlocked && root.masksByOwner[root.control.id]!==undefined;onTriggered:PaintClient.removeLayer(root.masksByOwner[root.control.id].id) }
        MenuSeparator {}
        GlassMenuItem { objectName: "ungroupLayer"; text: "解组（移除组属性）"; enabled: root.selected.group && !PaintClient.layers.some(function(l){return l.mask && l.parent===root.selected.id}); onTriggered: PaintClient.ungroupLayer(root.selected.id) }
        GlassMenuItem { objectName: "moveLayerOut"; text: "移出到上一级"; enabled: root.selected.parent!==0 && !root.selected.mask; onTriggered: {
            const parent=PaintClient.layers.find(function(layer) { return layer.id===root.selected.parent })
            if(parent) PaintClient.reparentLayer(root.selected.id,parent.parent)
        } }
        MenuSeparator {}
        Instantiator {
            model: groupMenu.visible ? PaintClient.layers.filter(function(layer) { return layer.group }) : []
            delegate: GlassMenuItem {
                required property var modelData
                objectName: "moveLayerInto:"+modelData.id
                text: "移入："+modelData.name; enabled: !root.selected.mask && root.canMoveInto(modelData)
                onTriggered: PaintClient.reparentLayer(root.selected.id,modelData.id)
            }
            onObjectAdded: (index,object) => groupMenu.insertItem(index+7,object)
            onObjectRemoved: (index,object) => groupMenu.removeItem(object)
        }
    }
}
