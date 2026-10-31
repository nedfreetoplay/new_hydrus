// WS-15.2.2b: typed metadata-panel seam bodies (Rust structs, no JSON).
// Covers HydrusMetadataPanel plus the metadata half of the still-JSON tag
// dialog (converted at the boundary until its own subtask).
//
// Same rules as typed_seams.cpp: this translation unit includes ONLY the
// generated header <hydrus_client/src/gui/qt/api.cxx.h> (plus Qt and the
// declaration-only support headers). The generated header pulls the exported
// header set, so including local topic headers here would define every widget
// class twice.

#include <hydrus_client/src/gui/qt/api.cxx.h>

#include "support/pairs.hpp"
#include "support/sort_options.hpp"
#include "support/strings.hpp"
#include "support/widgets.hpp"

#include <QByteArray>
#include <QCheckBox>
#include <QComboBox>
#include <QCompleter>
#include <QDialog>
#include <QDialogButtonBox>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLayoutItem>
#include <QLineEdit>
#include <QListWidget>
#include <QPushButton>
#include <QSignalBlocker>
#include <QSlider>
#include <QVBoxLayout>

// The class definitions (members, other methods) come from the exported
// bridge.hpp chain included above; the typed declarations live in the topic
// headers next to forward declarations of the structs.

namespace {

// WS-15.2.2b: converts the metadata half of the still-JSON tag dialog to the
// typed MetadataState consumed by HydrusMetadataPanel. Field reads mirror
// HydrusMetadataPanel::set_state one to one; the dialog envelope converts in
// its own subtask.
MetadataState metadata_state_from_json(const QJsonObject &metadata) {
    MetadataState typed;

    const auto assign_text = [](rust::String &field, const QJsonValue &text) {
        const QByteArray bytes = text.toString().toUtf8();
        field = rust::String(bytes.constData());
    };

    assign_text(typed.hash, metadata.value(QStringLiteral("hash")));

    for (const QJsonValue &value : metadata.value(QStringLiteral("tag_services")).toArray()) {
        const QJsonObject service = value.toObject();
        TagServiceState row;
        assign_text(row.key, service.value(QStringLiteral("key")));
        assign_text(row.name, service.value(QStringLiteral("name")));

        for (const QJsonValue &tag : service.value(QStringLiteral("tags")).toArray()) {
            const QByteArray bytes = tag.toString().toUtf8();
            row.tags.push_back(rust::String(bytes.constData()));
        }

        for (const QJsonValue &tag : service.value(QStringLiteral("display_tags")).toArray()) {
            const QByteArray bytes = tag.toString().toUtf8();
            row.display_tags.push_back(rust::String(bytes.constData()));
        }

        const QJsonObject sources = service.value(QStringLiteral("display_tag_sources")).toObject();

        for (auto it = sources.begin(); it != sources.end(); ++it) {
            TagSource source;
            const QByteArray display_bytes = it.key().toUtf8();
            source.display = rust::String(display_bytes.constData());

            for (const QJsonValue &tag : it.value().toArray()) {
                const QByteArray bytes = tag.toString().toUtf8();
                source.storage.push_back(rust::String(bytes.constData()));
            }

            row.sources.push_back(source);
        }

        typed.tag_services.push_back(row);
    }

    for (const QJsonValue &value : metadata.value(QStringLiteral("ratings")).toArray()) {
        const QJsonObject rating = value.toObject();
        RatingState row;
        assign_text(row.key, rating.value(QStringLiteral("key")));
        assign_text(row.name, rating.value(QStringLiteral("name")));
        const QString kind = rating.value(QStringLiteral("kind")).toString();

        if (kind == QStringLiteral("numerical")) {
            row.kind = RatingKind::Numerical;
            row.value_num = rating.value(QStringLiteral("value")).toDouble(0.0);
        } else if (kind == QStringLiteral("like")) {
            row.kind = RatingKind::Like;
            assign_text(row.value_text, rating.value(QStringLiteral("value")));
        } else {
            row.kind = RatingKind::IncDec;
            row.value_int = static_cast<int64_t>(rating.value(QStringLiteral("value")).toInt(0));
        }

        typed.ratings.push_back(row);
    }

    for (const QJsonValue &value : metadata.value(QStringLiteral("notes")).toArray()) {
        const QJsonObject note = value.toObject();
        NoteState row;
        assign_text(row.name, note.value(QStringLiteral("name")));
        assign_text(row.text, note.value(QStringLiteral("text")));
        typed.notes.push_back(row);
    }

    return typed;
}

} // namespace

// --- HydrusMetadataPanel ---------------------------------------------------

void HydrusMetadataPanel::set_state(const MetadataState &metadata) {
    hash = to_qstring(metadata.hash);

    tag_services_data.clear();

    for (const TagServiceState &service : metadata.tag_services) {
        HydrusTagServiceData row;
        row.key = to_qstring(service.key);
        row.name = to_qstring(service.name);

        for (const rust::String &tag : service.tags) {
            row.tags.append(to_qstring(tag));
        }

        for (const rust::String &tag : service.display_tags) {
            row.display_tags.append(to_qstring(tag));
        }

        for (const TagSource &source : service.sources) {
            QStringList storage;

            for (const rust::String &tag : source.storage) {
                storage.append(to_qstring(tag));
            }

            row.sources.insert(to_qstring(source.display), storage);
        }

        tag_services_data.append(row);
    }

    const QString previous = services->currentData().toString();
    QSignalBlocker blocker(services);
    services->clear();

    for (const HydrusTagServiceData &row : tag_services_data) {
        services->addItem(row.name, row.key);
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

    ratings_data.clear();

    for (const RatingState &rating : metadata.ratings) {
        HydrusRatingData row;
        row.key = to_qstring(rating.key);
        row.name = to_qstring(rating.name);
        row.kind = static_cast<int>(rating.kind);
        row.value_num = rating.value_num;
        row.value_text = to_qstring(rating.value_text);
        row.value_int = static_cast<long long>(rating.value_int);
        ratings_data.append(row);
    }

    rebuild_ratings();

    notes->clear();

    for (const NoteState &note : metadata.notes) {
        QString text = to_qstring(note.text).simplified();

        if (text.size() > 100) {
            text = text.left(100) + QStringLiteral("...");
        }

        notes->addItem(QStringLiteral("%1: %2").arg(to_qstring(note.name), text));
    }

    if (metadata.notes.empty()) {
        notes->addItem(QStringLiteral("no notes"));
    }
}

void HydrusMetadataPanel::rebuild_tags() {
    tags->clear();

    const QString key = services->currentData().toString();
    const QString filter = tag_filter->text().trimmed();

    for (const HydrusTagServiceData &row : tag_services_data) {
        if (row.key != key) {
            continue;
        }

        const bool show_display = display_tags->isChecked();
        const QStringList &names = show_display ? row.display_tags : row.tags;

        for (const QString &text : names) {
            if (filter.isEmpty() || text.contains(filter, Qt::CaseInsensitive)) {
                auto *item = new QListWidgetItem(text, tags);

                if (show_display) {
                    const QStringList storage = row.sources.value(text);

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

void HydrusMetadataPanel::rebuild_ratings() {
    while (QLayoutItem *item = ratings_layout->takeAt(0)) {
        if (QWidget *widget = item->widget()) {
            widget->deleteLater();
        }

        delete item;
    }

    if (ratings_data.isEmpty()) {
        auto *label = new QLabel(QStringLiteral("No rating services. Create one in services."), ratings);
        label->setWordWrap(true);
        ratings_layout->addWidget(label);
    }

    for (const HydrusRatingData &rating : ratings_data) {
        const QString service = rating.key;
        const int kind = rating.kind;
        const QString name = rating.name;

        auto *row = new QWidget(ratings);
        auto *row_layout = new QVBoxLayout(row);
        row_layout->setContentsMargins(0, 0, 0, 0);
        row_layout->setSpacing(2);

        auto *caption = new QLabel(name, row);
        caption->setStyleSheet(QStringLiteral("font-weight: bold;"));
        row_layout->addWidget(caption);

        if (kind == 0) {
            const double current = rating.value_num;
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
        } else if (kind == 1) {
            auto *buttons = new QWidget(row);
            auto *buttons_layout = new QHBoxLayout(buttons);
            buttons_layout->setContentsMargins(0, 0, 0, 0);
            auto *like = new QPushButton(QStringLiteral("like"), buttons);
            auto *dislike = new QPushButton(QStringLiteral("dislike"), buttons);
            auto *clear = new QPushButton(QStringLiteral("clear"), buttons);
            const QString current = rating.value_text;
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
            auto *label = new QLabel(QStringLiteral("value: %1").arg(rating.value_int), buttons);
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

void HydrusMainWindow::show_tag_dialog(const QJsonObject &state) {
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("manage metadata"));
    dialog.resize(460, 620);

    auto *layout = new QVBoxLayout(&dialog);
    auto *panel = new HydrusMetadataPanel(this, &dialog);
    panel->set_suggestions(suggestions());
    panel->set_state(
        metadata_state_from_json(state.value(QStringLiteral("metadata")).toObject()));
    layout->addWidget(panel, 1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::accept);
    layout->addWidget(buttons);

    dialog.exec();
}

