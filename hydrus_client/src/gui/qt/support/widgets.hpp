#pragma once

#include <QString>

class QFrame;
class QLineEdit;
class QListWidgetItem;
class QPushButton;
class QToolButton;
class QVBoxLayout;
class QWidget;

// WS-15.1.1: small widget helpers moved verbatim from bridge.cpp.
QFrame *make_static_box(
    const QString &title,
    QWidget *parent,
    QVBoxLayout **content_layout,
    QPushButton **expand_button = nullptr);
QToolButton *make_icon_button(QWidget *parent, const QString &kind, const QString &tooltip);
void make_on_off_button(
    QPushButton *button,
    const QString &on_label,
    const QString &off_label,
    bool start_on);
QWidget *field_row(const QString &label, QWidget *field, QWidget *parent);
QString page_hash_from_item(const QListWidgetItem *item);
void update_line_edit(QLineEdit *edit, const QString &value);
