#include "DockTree.h"
#include <QUuid>
#include <algorithm>
#include <cmath>

namespace DockTree {
Node leaf(const QString &group) { return {{"group",group}}; }
Node split(Node first,Node second,const QString &axis,double ratio,const QString &preferred) {
    if(first.isEmpty()) return second;
    if(second.isEmpty()) return first;
    return {{"id",QUuid::createUuid().toString(QUuid::WithoutBraces)},{"axis",axis},{"ratio",std::clamp(ratio,.05,.95)},
            {"first",first},{"second",second},{"preferred",preferred}};
}
QStringList leaves(const Node &node) {
    if(node.isEmpty()) return {};
    if(node.contains("group")) return {node.value("group").toString()};
    return leaves(node.value("first").toObject())+leaves(node.value("second").toObject());
}
bool remove(Node &node,const QString &group) {
    if(node.isEmpty()) return false;
    if(node.contains("group")) { if(node.value("group").toString()!=group) return false; node={}; return true; }
    auto first=node.value("first").toObject(),second=node.value("second").toObject();
    if(!remove(first,group) && !remove(second,group)) return false;
    if(first.isEmpty()) node=second;
    else if(second.isEmpty()) node=first;
    else { node.insert("first",first); node.insert("second",second); }
    return true;
}
bool insert(Node &node,const QString &target,const QString &group,const QString &edge) {
    if(node.isEmpty()) return false;
    if(node.value("group").toString()==target) {
        const bool before=edge=="left" || edge=="before" || edge=="top";
        const bool horizontal=edge=="left" || edge=="right";
        const auto added=leaf(group);
        const double ratio=group=="__canvas"?(before?.7:.3):(before?.3:.7);
        node=split(before?added:node,before?node:added,horizontal?"horizontal":"vertical",ratio,
                   horizontal && group=="__toolstrip" ? (before?"toolsFirst":"toolsSecond") : QString());
        return true;
    }
    auto first=node.value("first").toObject(),second=node.value("second").toObject();
    if(insert(first,target,group,edge)) { node.insert("first",first); return true; }
    if(insert(second,target,group,edge)) { node.insert("second",second); return true; }
    return false;
}
bool insertAround(Node &node,const QStringList &targets,const QString &group,const QString &edge,bool row) {
    return insertBlock(node,targets,leaf(group),edge,row);
}
bool insertBlock(Node &node,const QStringList &targets,Node block,const QString &edge,bool row) {
    if(node.isEmpty() || targets.isEmpty())return false;
    if(leaves(node)==targets) {
        const bool before=edge=="left" || edge=="before" || edge=="top";
        const bool horizontal=edge=="left" || edge=="right";
        node=split(before?block:node,before?node:block,horizontal?"horizontal":"vertical",before?.3:.7);
        if(row && horizontal)node.insert("role","row");
        return true;
    }
    auto first=node.value("first").toObject(),second=node.value("second").toObject();
    if(insertBlock(first,targets,block,edge,row)){node.insert("first",first);return true;}
    if(insertBlock(second,targets,block,edge,row)){node.insert("second",second);return true;}
    return false;
}
bool setRatio(Node &node,const QString &id,double ratio) {
    if(node.isEmpty() || node.contains("group")) return false;
    if(node.value("id").toString()==id) { node.insert("ratio",std::clamp(ratio,.05,.95)); node.insert("preferred",QString()); return true; }
    auto first=node.value("first").toObject(),second=node.value("second").toObject();
    if(setRatio(first,id,ratio)) { node.insert("first",first); return true; }
    if(setRatio(second,id,ratio)) { node.insert("second",second); return true; }
    return false;
}
bool validate(const Node &node,const QSet<QString> &allowed,QSet<QString> &seen,QSet<QString> &splits,int depth) {
    if(depth>128) return false; // At most 64 utility leaves plus canvas/tools.
    if(node.isEmpty()) return depth==0;
    if(node.contains("group")) {
        const auto id=node.value("group").toString();
        if(node.size()!=1 || !allowed.contains(id) || seen.contains(id)) return false;
        seen.insert(id); return true;
    }
    const auto id=node.value("id").toString(),axis=node.value("axis").toString(),preferred=node.value("preferred").toString();
    const auto ratio=node.value("ratio").toDouble(-1);
    const auto role=node.value("role").toString();
    if((node.contains("role") && (role!="row" || axis!="horizontal" || !preferred.isEmpty())) ||
       (role=="row" && (leaves(node).contains("__canvas") || leaves(node).contains("__toolstrip"))) ||
       id.isEmpty() || id.size()>128 || splits.contains(id) || (axis!="horizontal" && axis!="vertical") ||
       !std::isfinite(ratio) || ratio<.05 || ratio>.95 ||
       !QStringList{"","left","right","toolsFirst","toolsSecond"}.contains(preferred)) return false;
    splits.insert(id);
    return validate(node.value("first").toObject(),allowed,seen,splits,depth+1)
        && validate(node.value("second").toObject(),allowed,seen,splits,depth+1);
}
}
