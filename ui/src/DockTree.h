#pragma once
#include <QJsonObject>
#include <QSet>
#include <QStringList>

// Value trees have no shared pointers/cycles. A removed leaf promotes its sibling.
namespace DockTree {
using Node=QJsonObject;
Node leaf(const QString &group);
Node split(Node first,Node second,const QString &axis,double ratio=.5,const QString &preferred={});
QStringList leaves(const Node &node);
bool remove(Node &node,const QString &group);
bool insert(Node &node,const QString &target,const QString &group,const QString &edge);
bool insertAround(Node &node,const QStringList &targets,const QString &group,const QString &edge,bool row=false);
bool insertBlock(Node &node,const QStringList &targets,Node block,const QString &edge,bool row=false);
bool setRatio(Node &node,const QString &id,double ratio);
bool validate(const Node &node,const QSet<QString> &allowed,QSet<QString> &seen,QSet<QString> &splits,int depth=0);
}
