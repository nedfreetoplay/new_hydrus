#include "widgets.hpp"

#include "icons.hpp"

#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QListWidgetItem>
#include <QObject>
#include <QPushButton>
#include <QStyle>
#include <QToolButton>
#include <QVBoxLayout>
#include <QWidget>

QString page_hash_from_item(const QListWidgetItem *item) {    if (item == nullptr) {
        return {};
    }

    return item->data(Qt::UserRole).toString();
}

void update_line_edit(QLineEdit *edit, const QString &value) {
    if (edit != nullptr && !edit->hasFocus() && edit->text() != value) {
        edit->setText(value);
    }
}

QWidget *field_row(const QString &label, QWidget *field, QWidget *parent) {
    auto *row = new QWidget(parent);
    auto *layout = new QHBoxLayout(row);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(6);

    auto *caption = new QLabel(label, row);
    caption->setMinimumWidth(90);
    layout->addWidget(caption);
    layout->addWidget(field, 1);

    return row;
}

QFrame *make_static_box(
    const QString &title,
    QWidget *parent,
    QVBoxLayout **content_layout,
    QPushButton **expand_button) {
    auto *box = new QFrame(parent);
    box->setFrameStyle(QFrame::Box | QFrame::Raised);

    auto *layout = new QVBoxLayout(box);
    layout->setContentsMargins(4, 2, 4, 4);
    layout->setSpacing(0);

    auto *header_row = new QHBoxLayout();
    header_row->setContentsMargins(0, 0, 0, 0);
    header_row->setSpacing(0);

    auto *header = new QLabel(title, box);
    QFont header_font = header->font();
    header_font.setBold(true);
    header->setFont(header_font);
    header->setAlignment(Qt::AlignHCenter);

    header_row->addSpacing(30);
    header_row->addStretch(1);
    header_row->addWidget(header, 0, Qt::AlignHCenter);
    header_row->addStretch(1);

    if (expand_button != nullptr) {
        auto *button = new QPushButton(box);
        button->setFlat(true);
        button->setFixedSize(30, 22);
        button->setFocusPolicy(Qt::NoFocus);
        button->setIcon(box->style()->standardIcon(QStyle::SP_ArrowUp));
        button->setIconSize(QSize(12, 12));
        header_row->addWidget(button, 0, Qt::AlignRight | Qt::AlignVCenter);
        *expand_button = button;
    } else {
        header_row->addSpacing(30);
    }

    layout->addLayout(header_row);

    auto *content = new QWidget(box);
    auto *content_vbox = new QVBoxLayout(content);
    content_vbox->setContentsMargins(4, 2, 4, 4);
    content_vbox->setSpacing(4);
    layout->addWidget(content, 1);

    *content_layout = content_vbox;

    return box;
}

QToolButton *make_icon_button(QWidget *parent, const QString &kind, const QString &tooltip) {
    auto *button = new QToolButton(parent);
    button->setIcon(hydrus_small_icon(kind));
    button->setIconSize(QSize(16, 16));
    button->setFixedSize(24, 24);
    button->setAutoRaise(true);
    button->setFocusPolicy(Qt::NoFocus);
    button->setToolTip(tooltip);
    return button;
}

void make_on_off_button(
    QPushButton *button,
    const QString &on_label,
    const QString &off_label,
    bool start_on) {
    button->setObjectName(QStringLiteral("HydrusOnOffButton"));
    button->setText(start_on ? on_label : off_label);
    button->setProperty("hydrus_on", start_on);

    QObject::connect(button, &QPushButton::clicked, button, [button, on_label, off_label]() {
        const bool on = !button->property("hydrus_on").toBool();
        button->setProperty("hydrus_on", on);
        button->setText(on ? on_label : off_label);
        button->style()->unpolish(button);
        button->style()->polish(button);
    });
}
