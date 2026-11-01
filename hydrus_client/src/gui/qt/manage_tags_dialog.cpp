
// WS-15.1.3c1: manage tags dialog moved from bridge.cpp (move-only split;
// inline methods outlined, no logic change).

#include <hydrus_client/src/gui/qt/api.cxx.h>
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

HydrusManageTagsDialog::HydrusManageTagsDialog(HydrusMainWindow *main_window, const ManageTagsDialogState &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
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

    for (const ManageTagService &service : state.services) {
        const QString name = to_qstring(service.name);

        if (!name.isEmpty()) {
            services->addItem(name);
        }

        services_state.push_back(service);
    }

    for (const rust::String &hash : state.hashes) {
        hashes_state.push_back(std::string(hash.data(), hash.size()));
    }

    summary->setText(to_qstring(state.summary));

    connect(services, &QComboBox::currentTextChanged, this, [this](const QString &) {
        rebuild();
    });
    connect(add, &QPushButton::clicked, this, [this]() { add_tags(); });
    connect(remove, &QPushButton::clicked, this, [this]() { remove_tags(); });
    connect(input, &QLineEdit::returnPressed, this, [this]() { add_tags(); });

    rebuild();
}

const ManageTagService *HydrusManageTagsDialog::current_service() const {
    const QString name = services->currentText();

    for (const ManageTagService &service : services_state) {
        if (to_qstring(service.name) == name) {
            return &service;
        }
    }

    return nullptr;
}

void HydrusManageTagsDialog::rebuild() {
    tags->clear();

    const ManageTagService *service = current_service();

    if (service == nullptr) {
        return;
    }

    for (const ManageTagEntry &entry : service->tags) {
        const QString name = to_qstring(entry.tag);
        const int count = static_cast<int>(entry.count);
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

    QJsonArray hashes;

    for (const std::string &hash : hashes_state) {
        hashes.append(QString::fromStdString(hash));
    }
    QJsonArray array;

    for (const QString &value : values) {
        array.append(value);
    }

    QJsonObject payload;
    payload.insert(QStringLiteral("hashes"), hashes);
    payload.insert(QStringLiteral("service"), services->currentText());
    payload.insert(operation, array);
    main->notify(UiAction::ApplyTags,
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
