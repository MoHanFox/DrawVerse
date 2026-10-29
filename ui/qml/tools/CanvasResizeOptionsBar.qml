import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../controls"
import "../theme"
RowLayout {
    id:root;objectName:"canvasResizeOptionsBar";property var editor:null;spacing:8
    enabled:editor && !editor.submitting
    Binding {target:root.editor;property:"inputsValid";value:widthField.acceptableInput && heightField.acceptableInput && dpiField.acceptableInput;when:root.editor!==null}
    Label {text:"调整画布";color:Theme.text}
    CompactComboBox {objectName:"canvasResizeUnits";model:["像素","比例"];currentIndex:root.editor?root.editor.units:0;onActivated:if(root.editor)root.editor.units=currentIndex;Layout.preferredWidth:100}
    CompactTextField {id:widthField;objectName:"canvasResizeWidth";Layout.preferredWidth:75;text:root.editor?String(root.editor.units===0?root.editor.pixelWidth:root.editor.ratioWidth):"";validator:DoubleValidator {bottom:root.editor && root.editor.units===0?1:0;top:1000000;decimals:3} onTextEdited:applyValue();function applyValue(){if(!root.editor || !acceptableInput)return;if(root.editor.units===0)root.editor.setSize(Number(text),root.editor.pixelHeight);else{root.editor.ratioWidth=Number(text);if(root.editor.ratioWidth>0 && root.editor.ratioHeight>0)root.editor.setSize(root.editor.pixelWidth,root.editor.pixelWidth/root.editor.ratio)}}}
    Button {objectName:"canvasResizeSwap";text:"⇆";implicitWidth:28;onClicked:root.editor.swap()}
    CompactTextField {id:heightField;objectName:"canvasResizeHeight";Layout.preferredWidth:75;text:root.editor?String(root.editor.units===0?root.editor.pixelHeight:root.editor.ratioHeight):"";validator:DoubleValidator {bottom:root.editor && root.editor.units===0?1:0;top:1000000;decimals:3} onTextEdited:applyValue();function applyValue(){if(!root.editor || !acceptableInput)return;if(root.editor.units===0)root.editor.setSize(root.editor.pixelWidth,Number(text));else{root.editor.ratioHeight=Number(text);if(root.editor.ratioWidth>0 && root.editor.ratioHeight>0)root.editor.setSize(root.editor.pixelWidth,root.editor.pixelWidth/root.editor.ratio)}}}
    Label {text:"DPI";color:Theme.text}
    CompactTextField {id:dpiField;objectName:"canvasResizeDpi";Layout.preferredWidth:65;text:root.editor?String(root.editor.dpi):"72";validator:IntValidator {bottom:1;top:12000} onTextEdited:if(root.editor && acceptableInput)root.editor.dpi=Number(text)}
    CompactCheckBox {objectName:"canvasResizeDelete";text:"删除裁剪像素";checked:root.editor?root.editor.deletePixels:false;onToggled:root.editor.deletePixels=checked}
    Label {text:"填充";color:Theme.text}
    CompactComboBox {objectName:"canvasResizeFill";Layout.preferredWidth:115;model:["填充白色","所选颜色"];currentIndex:root.editor && root.editor.selectedColor?1:0;onActivated:root.editor.selectedColor=currentIndex===1}
    Item {Layout.fillWidth:true}
    Button {objectName:"canvasResizeCancel";text:"取消";onClicked:root.editor.finished()}
    Button {objectName:"canvasResizeConfirm";text:"确定";onClicked:root.editor.confirm()}
}
