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

void HydrusMetadataPanel::set_state(const QJsonObject &metadata) {
    data = metadata;
    hash = metadata.value(QStringLiteral("hash")).toString();

    const QString previous = services->currentData().toString();
    QSignalBlocker blocker(services);
    services->clear();

    const QJsonArray tag_services = metadata.value(QStringLiteral("tag_services")).toArray();

    for (const QJsonValue &value : tag_services) {
        const QJsonObject service = value.toObject();
        services->addItem(
            service.value(QStringLiteral("name")).toString(),
            service.value(QStringLiteral("key")).toString());
    }

    if (services->count() == 0) {
        services->addItem(QStringLiteral("no local tag service"), QString());
    }

    if (!previous.isEmpty()) {
        const int index = services->findData(previous);

        if (index >= 0) {
            services->setCurrentIndex(index);
        }
    }

    blocker.unblock();
    rebuild_tags();
    rebuild_ratings(metadata.value(QStringLiteral("ratings")).toArray());

    notes->clear();

    for (const QJsonValue &value : metadata.value(QStringLiteral("notes")).toArray()) {
        const QJsonObject note = value.toObject();
        QString text = note.value(QStringLiteral("text")).toString().simplified();

        if (text.size() > 100) {
            text = text.left(100) + QStringLiteral("...");
        }

        notes->addItem(QStringLiteral("%1: %2").arg(note.value(QStringLiteral("name")).toString(), text));
    }

    const QJsonArray note_array = metadata.value(QStringLiteral("notes")).toArray();

    if (note_array.isEmpty()) {
        notes->addItem(QStringLiteral("no notes"));
    }
}

void HydrusMetadataPanel::set_suggestions(const QStringList &suggestions) {
    completer->setModel(new QStringListModel(suggestions, completer));
}

void HydrusMetadataPanel::rebuild_tags() {
    tags->clear();

    const QString key = services->currentData().toString();
    const QJsonArray tag_services = data.value(QStringLiteral("tag_services")).toArray();
    const QString filter = tag_filter->text().trimmed();

    for (const QJsonValue &value : tag_services) {
        const QJsonObject service = value.toObject();

        if (service.value(QStringLiteral("key")).toString() != key) {
            continue;
        }

        const bool show_display = display_tags->isChecked();
        const QJsonArray names = service.value(
            show_display ? QStringLiteral("display_tags") : QStringLiteral("tags")).toArray();
        const QJsonObject sources = service.value(QStringLiteral("display_tag_sources")).toObject();

        for (const QJsonValue &tag : names) {
            const QString text = tag.toString();

            if (filter.isEmpty() || text.contains(filter, Qt::CaseInsensitive)) {
                auto *item = new QListWidgetItem(text, tags);

                if (show_display) {
                    const QStringList storage = json_to_strings(
                        sources.value(text).toArray());

                    if (!storage.isEmpty()) {
                        item->setToolTip(QStringLiteral("storage tags: %1").arg(
                            storage.join(QStringLiteral(", "))));
                    }
                }
            }
        }

        break;
    }
}

void HydrusMetadataPanel::rebuild_ratings(const QJsonArray &ratings_value) {
    while (QLayoutItem *item = ratings_layout->takeAt(0)) {
        if (QWidget *widget = item->widget()) {
            widget->deleteLater();
        }

        delete item;
    }

    if (ratings_value.isEmpty()) {
        auto *label = new QLabel(QStringLiteral("No rating services. Create one in services."), ratings);
        label->setWordWrap(true);
        ratings_layout->addWidget(label);
    }

    for (const QJsonValue &value : ratings_value) {
        const QJsonObject rating = value.toObject();
        const QString service = rating.value(QStringLiteral("key")).toString();
        const QString kind = rating.value(QStringLiteral("kind")).toString();
        const QString name = rating.value(QStringLiteral("name")).toString();

        auto *row = new QWidget(ratings);
        auto *row_layout = new QVBoxLayout(row);
        row_layout->setContentsMargins(0, 0, 0, 0);
        row_layout->setSpacing(2);

        auto *caption = new QLabel(name, row);
        caption->setStyleSheet(QStringLiteral("font-weight: bold;"));
        row_layout->addWidget(caption);

        if (kind == QStringLiteral("numerical")) {
            const double current = rating.value(QStringLiteral("value")).toDouble();
            auto *slider_row = new QWidget(row);
            auto *slider_layout = new QHBoxLayout(slider_row);
            slider_layout->setContentsMargins(0, 0, 0, 0);
            auto *slider = new QSlider(Qt::Horizontal, slider_row);
            slider->setRange(0, 100);
            slider->setValue(static_cast<int>(current * 100.0 + 0.5));
            auto *label = new QLabel(QStringLiteral("%1%").arg(slider->value()), slider_row);
            label->setMinimumWidth(42);
            slider_layout->addWidget(slider, 1);
            slider_layout->addWidget(label);
            connect(slider, &QSlider::valueChanged, label, [label](int value) {
                label->setText(QStringLiteral("%1%").arg(value));
            });
            connect(slider, &QSlider::sliderReleased, this, [this, service, slider]() {
                send_rating(service, QStringLiteral("set"), slider->value() / 100.0);
            });
            row_layout->addWidget(slider_row);
        } else if (kind == QStringLiteral("like")) {
            auto *buttons = new QWidget(row);
            auto *buttons_layout = new QHBoxLayout(buttons);
            buttons_layout->setContentsMargins(0, 0, 0, 0);
            auto *like = new QPushButton(QStringLiteral("like"), buttons);
            auto *dislike = new QPushButton(QStringLiteral("dislike"), buttons);
            auto *clear = new QPushButton(QStringLiteral("clear"), buttons);
            const QString current = rating.value(QStringLiteral("value")).toString();
            like->setCheckable(true);
            dislike->setCheckable(true);
            like->setChecked(current == QStringLiteral("like"));
            dislike->setChecked(current == QStringLiteral("dislike"));
            connect(like, &QPushButton::clicked, this, [this, service]() {
                send_rating(service, QStringLiteral("like"));
            });
            connect(dislike, &QPushButton::clicked, this, [this, service]() {
                send_rating(service, QStringLiteral("dislike"));
            });
            connect(clear, &QPushButton::clicked, this, [this, service]() {
                send_rating(service, QStringLiteral("clear"));
            });
            buttons_layout->addWidget(like);
            buttons_layout->addWidget(dislike);
            buttons_layout->addWidget(clear);
            buttons_layout->addStretch(1);
            row_layout->addWidget(buttons);
        } else {
            auto *buttons = new QWidget(row);
            auto *buttons_layout = new QHBoxLayout(buttons);
            buttons_layout->setContentsMargins(0, 0, 0, 0);
            auto *minus = new QPushButton(QStringLiteral("-1"), buttons);
            auto *plus = new QPushButton(QStringLiteral("+1"), buttons);
            auto *clear = new QPushButton(QStringLiteral("clear"), buttons);
            auto *label = new QLabel(QStringLiteral("value: %1").arg(rating.value(QStringLiteral("value")).toInt()), buttons);
            connect(minus, &QPushButton::clicked, this, [this, service]() {
                send_rating(service, QStringLiteral("set"), -1.0);
            });
            connect(plus, &QPushButton::clicked, this, [this, service]() {
                send_rating(service, QStringLiteral("set"), 1.0);
            });
            connect(clear, &QPushButton::clicked, this, [this, service]() {
                send_rating(service, QStringLiteral("clear"));
            });
            buttons_layout->addWidget(minus);
            buttons_layout->addWidget(plus);
            buttons_layout->addWidget(clear);
            buttons_layout->addWidget(label);
            buttons_layout->addStretch(1);
            row_layout->addWidget(buttons);
        }

        ratings_layout->addWidget(row);
    }

    ratings_layout->addStretch(1);
}

void HydrusMetadataPanel::send_tag(const QString &tag, bool add) {
    if (hash.isEmpty() || tag.trimmed().isEmpty()) {
        return;
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("hash"), hash);
    payload.insert(QStringLiteral("service"), services->currentData().toString());
    payload.insert(QStringLiteral("tag"), tag.trimmed());

    main->notify(
        add ? QStringLiteral("tag_add") : QStringLiteral("tag_remove"),
        QString(),
        json_payload(payload));

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

    QJsonObject payload;
    payload.insert(QStringLiteral("hash"), hash);
    payload.insert(QStringLiteral("service"), service);
    payload.insert(QStringLiteral("action"), action);
    payload.insert(QStringLiteral("value"), value);

    main->notify(QStringLiteral("rating_set"), QString(), json_payload(payload));
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

    QJsonObject payload;
    payload.insert(QStringLiteral("hash"), hash);
    main->notify(QStringLiteral("notes_manage"), QString(), json_payload(payload));
}
