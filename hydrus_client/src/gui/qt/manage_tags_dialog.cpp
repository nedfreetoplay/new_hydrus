#include "manage_tags_dialog.hpp"

// WS-15.1.3c1: manage tags dialog moved from bridge.cpp (move-only split;
// inline methods outlined, no logic change).

#include "bridge.hpp"
#include "support/support.hpp"

#include <QComboBox>
#include <QDialogButtonBox>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QPushButton>
#include <QRegularExpression>
#include <QVBoxLayout>

HydrusManageTagsDialog::HydrusManageTagsDialog(HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window), data(state) {
    setWindowTitle(QStringLiteral("manage tags"));
    resize(460, 600);

    auto *layout = new QVBoxLayout(this);

    services = new QComboBox(this);
    layout->addWidget(services);

    summary = new QLabel(this);
    layout->addWidget(summary);

    tags = new QListWidget(this);
    tags->setSelectionMode(QAbstractItemView::ExtendedSelection);
    layout->addWidget(tags, 1);

    auto *input_row = new QHBoxLayout();
    input = new QLineEdit(this);
    input->setPlaceholderText(QStringLiteral("tag1, tag2, namespace:tag"));
    input_row->addWidget(input, 1);

    auto *add = new QPushButton(QStringLiteral("add"), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    input_row->addWidget(add);
    input_row->addWidget(remove);
    layout->addLayout(input_row);

    auto *close_box = new QDialogButtonBox(QDialogButtonBox::Close, this);
    connect(close_box, &QDialogButtonBox::rejected, this, &QDialog::accept);
    layout->addWidget(close_box);

    for (const QJsonValue &value : data.value(QStringLiteral("services")).toArray()) {
        const QString name = value.toObject().value(QStringLiteral("name")).toString();

        if (!name.isEmpty()) {
            services->addItem(name);
        }
    }

    summary->setText(QStringLiteral("%1 file(s) selected")
        .arg(data.value(QStringLiteral("hashes")).toArray().size()));

    connect(services, &QComboBox::currentTextChanged, this, [this](const QString &) {
        rebuild();
    });
    connect(add, &QPushButton::clicked, this, [this]() { add_tags(); });
    connect(remove, &QPushButton::clicked, this, [this]() { remove_tags(); });
    connect(input, &QLineEdit::returnPressed, this, [this]() { add_tags(); });

    rebuild();
}

QJsonObject HydrusManageTagsDialog::current_service() const {
    const QString name = services->currentText();

    for (const QJsonValue &value : data.value(QStringLiteral("services")).toArray()) {
        const QJsonObject service = value.toObject();

        if (service.value(QStringLiteral("name")).toString() == name) {
            return service;
        }
    }

    return {};
}

void HydrusManageTagsDialog::rebuild() {
    tags->clear();

    for (const QJsonValue &value : current_service().value(QStringLiteral("tags")).toArray()) {
        const QJsonObject tag = value.toObject();
        const QString name = tag.value(QStringLiteral("tag")).toString();
        const int count = tag.value(QStringLiteral("count")).toInt();
        auto *item = new QListWidgetItem(
            count > 1 ? QStringLiteral("%1 (%2)").arg(name).arg(count) : name,
            tags);
        item->setData(Qt::UserRole, name);
    }
}

void HydrusManageTagsDialog::send(const QStringList &values, const QString &operation) {
    if (values.isEmpty() || services->currentText().isEmpty()) {
        return;
    }

    QJsonArray hashes = data.value(QStringLiteral("hashes")).toArray();
    QJsonArray array;

    for (const QString &value : values) {
        array.append(value);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("hashes"), hashes);
    payload.insert(QStringLiteral("service"), services->currentText());
    payload.insert(operation, array);
    main->notify(
        QStringLiteral("apply_tags"),
        QString(),
        QString::fromUtf8(QJsonDocument(payload).toJson(QJsonDocument::Compact)));
}

void HydrusManageTagsDialog::add_tags() {
    QStringList values;

    for (const QString &part : input->text().split(QRegularExpression(QStringLiteral("[,;\\n]")))) {
        const QString tag = part.trimmed();

        if (!tag.isEmpty()) {
            values.append(tag);
        }
    }

    if (values.isEmpty()) {
        return;
    }

    send(values, QStringLiteral("add"));

    for (const QString &tag : values) {
        bool exists = false;

        for (int index = 0; index < tags->count(); ++index) {
            if (tags->item(index)->data(Qt::UserRole).toString() == tag) {
                exists = true;
                break;
            }
        }

        if (!exists) {
            auto *item = new QListWidgetItem(tag, tags);
            item->setData(Qt::UserRole, tag);
        }
    }

    input->clear();
}

void HydrusManageTagsDialog::remove_tags() {
    QStringList values;

    for (QListWidgetItem *item : tags->selectedItems()) {
        values.append(item->data(Qt::UserRole).toString());
    }

    if (values.isEmpty()) {
        return;
    }

    send(values, QStringLiteral("remove"));

    for (QListWidgetItem *item : tags->selectedItems()) {
        delete item;
    }
}
