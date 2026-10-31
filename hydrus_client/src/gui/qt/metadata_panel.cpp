#include "metadata_panel.hpp"

#include "bridge.hpp"
#include "support/support.hpp"

#include <QCheckBox>
#include <QComboBox>
#include <QCompleter>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonValue>
#include <QLabel>
#include <QLayoutItem>
#include <QLineEdit>
#include <QListWidget>
#include <QPushButton>
#include <QSignalBlocker>
#include <QSlider>
#include <QStringListModel>
#include <QTabWidget>
#include <QVBoxLayout>

HydrusMetadataPanel::HydrusMetadataPanel(HydrusMainWindow *main_window, QWidget *parent)
    : QWidget(parent), main(main_window) {
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);

    auto *tabs = new QTabWidget(this);
    layout->addWidget(tabs);

    auto *tag_page = new QWidget(tabs);
    auto *tag_layout = new QVBoxLayout(tag_page);
    tag_layout->setContentsMargins(4, 4, 4, 4);
    tag_layout->setSpacing(4);

    services = new QComboBox(tag_page);
    tag_layout->addWidget(services);

    tag_filter = new QLineEdit(tag_page);
    tag_filter->setPlaceholderText(QStringLiteral("filter tags"));
    tag_layout->addWidget(tag_filter);

    display_tags = new QCheckBox(QStringLiteral("show display tags (siblings applied)"), tag_page);
    display_tags->setChecked(true);
    tag_layout->addWidget(display_tags);

    tags = new QListWidget(tag_page);
    tags->setSelectionMode(QAbstractItemView::ExtendedSelection);
    tag_layout->addWidget(tags, 1);

    tag_input = new QLineEdit(tag_page);
    tag_input->setPlaceholderText(QStringLiteral("add a tag"));
    completer = new QCompleter(main->suggestions(), this);
    completer->setCaseSensitivity(Qt::CaseInsensitive);
    completer->setFilterMode(Qt::MatchContains);
    tag_input->setCompleter(completer);
    tag_layout->addWidget(tag_input);

    auto *tag_buttons = new QHBoxLayout();
    auto *add_button = new QPushButton(QStringLiteral("add"), tag_page);
    auto *remove_button = new QPushButton(QStringLiteral("remove"), tag_page);
    tag_buttons->addWidget(add_button);
    tag_buttons->addWidget(remove_button);
    tag_buttons->addStretch(1);
    tag_layout->addLayout(tag_buttons);

    tabs->addTab(tag_page, QStringLiteral("tags"));

    auto *ratings_page = new QWidget(tabs);
    ratings_layout = new QVBoxLayout(ratings_page);
    ratings_layout->setContentsMargins(6, 6, 6, 6);
    ratings_layout->setSpacing(6);
    ratings = ratings_page;
    tabs->addTab(ratings_page, QStringLiteral("ratings"));

    auto *notes_page = new QWidget(tabs);
    auto *notes_layout = new QVBoxLayout(notes_page);
    notes_layout->setContentsMargins(4, 4, 4, 4);
    notes = new QListWidget(notes_page);
    notes_layout->addWidget(notes, 1);
    auto *edit_notes_button = new QPushButton(QStringLiteral("edit notes..."), notes_page);
    notes_layout->addWidget(edit_notes_button);
    tabs->addTab(notes_page, QStringLiteral("notes"));

    connect(services, QOverload<int>::of(&QComboBox::currentIndexChanged), this, [this](int) {
        rebuild_tags();
    });
    connect(tag_filter, &QLineEdit::textChanged, this, [this](const QString &) { rebuild_tags(); });
    connect(display_tags, &QCheckBox::clicked, this, [this](bool) { rebuild_tags(); });
    connect(add_button, &QPushButton::clicked, this, [this]() { add_tag(); });
    connect(remove_button, &QPushButton::clicked, this, [this]() { remove_selected_tags(); });
    connect(tag_input, &QLineEdit::returnPressed, this, [this]() { add_tag(); });
    connect(edit_notes_button, &QPushButton::clicked, this, [this]() { edit_notes(); });
    connect(tags, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem *) { remove_selected_tags(); });
}

// WS-15.2.2b: set_state/rebuild_tags/rebuild_ratings moved to typed_media.cpp
// (typed MetadataState; the QJsonObject seam is gone).

void HydrusMetadataPanel::set_suggestions(const QStringList &suggestions) {
    completer->setModel(new QStringListModel(suggestions, completer));
}

void HydrusMetadataPanel::send_tag(const QString &tag, bool add) {
    if (hash.isEmpty() || tag.trimmed().isEmpty()) {
        return;
    }

    const QByteArray hash_bytes = hash.toUtf8();
    const QByteArray service_bytes = services->currentData().toString().toUtf8();
    const QByteArray tag_bytes = tag.trimmed().toUtf8();
    app_tag_edit(
        *main,
        rust::Str(hash_bytes.constData(), hash_bytes.size()),
        rust::Str(service_bytes.constData(), service_bytes.size()),
        rust::Str(tag_bytes.constData(), tag_bytes.size()),
        add);

    if (add) {
        if (tags->findItems(tag.trimmed(), Qt::MatchExactly).isEmpty()) {
            tags->addItem(tag.trimmed());
        }
    } else {
        for (QListWidgetItem *item : tags->findItems(tag.trimmed(), Qt::MatchExactly)) {
            delete tags->takeItem(tags->row(item));
        }
    }
}

void HydrusMetadataPanel::send_rating(const QString &service, const QString &action, double value) {
    if (hash.isEmpty()) {
        return;
    }

    const QByteArray hash_bytes = hash.toUtf8();
    const QByteArray service_bytes = service.toUtf8();
    const QByteArray action_bytes = action.toUtf8();
    app_set_rating(
        *main,
        rust::Str(hash_bytes.constData(), hash_bytes.size()),
        rust::Str(service_bytes.constData(), service_bytes.size()),
        rust::Str(action_bytes.constData(), action_bytes.size()),
        value);
}

void HydrusMetadataPanel::add_tag() {
    const QString tag = tag_input->text().trimmed();

    if (tag.isEmpty()) {
        return;
    }

    send_tag(tag, true);
    tag_input->clear();
}

void HydrusMetadataPanel::remove_selected_tags() {
    QStringList selected;

    for (QListWidgetItem *item : tags->selectedItems()) {
        selected.append(item->text());
    }

    for (const QString &tag : selected) {
        send_tag(tag, false);
    }
}

void HydrusMetadataPanel::edit_notes() {
    if (hash.isEmpty()) {
        return;
    }

    const QByteArray hash_bytes = hash.toUtf8();
    app_manage_notes(*main, rust::Str(hash_bytes.constData(), hash_bytes.size()));
}
