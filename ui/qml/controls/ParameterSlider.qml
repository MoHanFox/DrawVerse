import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../theme"

ColumnLayout {
    id: root
    property string title: ""
    property string unit: ""
    property string tooltip: ""
    property string sliderName: objectName
    property string inputName: sliderName+"Input"
    property real value: 0
    property real from: 0
    property real to: 100
    property real sliderTo: to
    property int decimals: 0
    readonly property real factor: Math.pow(10,decimals)
    signal committed(real value)
    Layout.fillWidth: true
    Layout.minimumWidth: 0
    spacing: 2
    RowLayout {
        Layout.fillWidth: true
        spacing: 3
        Label {objectName:root.sliderName+"Label";text:root.title;color:Theme.text;font.pixelSize:10;Layout.fillWidth:true;Layout.minimumWidth:0;elide:Text.ElideRight}
        CompactSpinBox {
            objectName: root.inputName
            Layout.preferredWidth: 82
            Layout.minimumWidth: 66
            implicitHeight: 20
            editable: true
            from: Math.round(root.from*root.factor)
            to: Math.round(root.to*root.factor)
            stepSize: root.factor
            value: Math.round(root.value*root.factor)
            validator: DoubleValidator {bottom:root.from;top:root.to;decimals:root.decimals;locale:"C"}
            textFromValue: function(value,locale) {return Number(value/root.factor).toFixed(root.decimals).replace(/(\.[0-9]*?)0+$/, "$1").replace(/\.$/, "")}
            valueFromText: function(text,locale) {return Math.round(Number(text)*root.factor)}
            onValueModified: root.committed(value/root.factor)
            tooltip: root.tooltip
        }
        Label {visible:root.unit.length>0;text:root.unit;color:Theme.muted;font.pixelSize:9}
    }
    CompactSlider {
        objectName: root.sliderName
        Layout.fillWidth: true
        from: root.from
        to: root.sliderTo
        value: Math.min(to,root.value)
        onMoved: root.committed(value)
        tooltip: root.tooltip
    }
}
