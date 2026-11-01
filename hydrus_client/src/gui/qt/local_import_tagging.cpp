
// WS-15.1.3c2: local import filename tagging dialog moved verbatim from bridge.cpp (move-only split).

#include <hydrus_client/src/gui/qt/api.cxx.h>
#include "support/support.hpp"

#include <QHash>
#include <QHBoxLayout>
#include <QInputDialog>
#include <QJsonObject>
#include <QLabel>
#include <QListWidget>
#include <QMessageBox>
#include <QPushButton>
#include <QVBoxLayout>

// Typed reverse seam helper (WS-15.2.4c): flattens the tagging editor's
// `filename_tagging` object into typed rows. Declared in bridge_api.hpp;
// shared with import_folder_editor.cpp.
namespace {
rust::String rust_string(const QString &text) {
    const QByteArray bytes = text.toUtf8();
    return rust::String(bytes.constData(), static_cast<size_t>(bytes.size()));
}
}  // namespace

rust::Vec<FilenameTaggingEntry> filename_tagging_entries(const QJsonObject &tagging) {
    rust::Vec<FilenameTaggingEntry> entries;

    for (auto iterator = tagging.constBegin(); iterator != tagging.constEnd(); ++iterator) {
        const QJsonObject options = iterator.value().toObject();
        FilenameTaggingEntry entry;
        entry.service_key = rust_string(iterator.key());

        for (const QJsonValue &tag_value : options.value(QStringLiteral("tags_for_all")).toArray()) {
            entry.tags_for_all.push_back(rust_string(tag_value.toString()));
        }

        entry.add_filename = options.value(QStringLiteral("add_filename")).toBool(false);
        entry.filename_namespace =
            rust_string(options.value(QStringLiteral("filename_namespace")).toString());

        const QJsonObject directories = options.value(QStringLiteral("directories")).toObject();

        for (auto directory = directories.constBegin(); directory != directories.constEnd();
             ++directory) {
            DirectoryTaggingRow row;
            row.slot = directory.key().toInt();
            const QJsonObject rule = directory.value().toObject();
            row.enabled = rule.value(QStringLiteral("enabled")).toBool(false);
            row.tag_namespace = rust_string(rule.value(QStringLiteral("namespace")).toString());
            entry.directories.push_back(row);
        }

        for (const QJsonValue &rule_value :
             options.value(QStringLiteral("quick_namespaces")).toArray()) {
            const QJsonObject rule = rule_value.toObject();
            QuickNamespaceRow row;
            row.tag_namespace = rust_string(rule.value(QStringLiteral("namespace")).toString());
            row.regex = rust_string(rule.value(QStringLiteral("regex")).toString());
            entry.quick_namespaces.push_back(row);
        }

        for (const QJsonValue &regex_value : options.value(QStringLiteral("regexes")).toArray()) {
            entry.regexes.push_back(rust_string(regex_value.toString()));
        }

        entries.push_back(entry);
    }

    return entries;
}

HydrusLocalImportTaggingDialog::HydrusLocalImportTaggingDialog(
    HydrusMainWindow *main_window, const QJsonObject &state, QWidget *parent)
    : QDialog(parent), main(main_window) {
    setWindowTitle(QStringLiteral("filename tagging"));
    setObjectName(QStringLiteral("hydrus_local_import_tagging"));
    setAttribute(Qt::WA_DeleteOnClose, true);
    resize(640, 520);

    auto *layout = new QVBoxLayout(this);

    help = new QLabel(this);
    help->setWordWrap(true);
    layout->addWidget(help);

    list = new QListWidget(this);
    list->setObjectName(QStringLiteral("local_import_filename_tagging"));
    layout->addWidget(list, 1);

    auto *buttons = new QHBoxLayout();
    auto *add = new QPushButton(QStringLiteral("add tag service..."), this);
    auto *edit = new QPushButton(QStringLiteral("edit..."), this);
    auto *remove = new QPushButton(QStringLiteral("remove"), this);
    buttons->addWidget(add);
    buttons->addWidget(edit);
    buttons->addWidget(remove);
    buttons->addStretch(1);
    layout->addLayout(buttons);

    auto *footer = new QHBoxLayout();
    auto *cancel = new QPushButton(QStringLiteral("cancel"), this);
    auto *save = new QPushButton(QStringLiteral("save and import"), this);
    save->setObjectName(QStringLiteral("HydrusAccept"));
    footer->addStretch(1);
    footer->addWidget(cancel);
    footer->addWidget(save);
    layout->addLayout(footer);

    connect(add, &QPushButton::clicked, this, [this]() {
        QStringList available;

        for (auto iterator = service_name_to_key.constBegin(); iterator != service_name_to_key.constEnd(); ++iterator) {
            if (!filename_tagging.contains(iterator.value())) {
                available << iterator.key();
            }
        }

        if (available.isEmpty()) {
            QMessageBox::information(
                this,
                windowTitle(),
                QStringLiteral("Create a local tag service first, or every local tag service already has rules."));
            return;
        }

        available.sort();
        bool ok = false;
        const QString name = QInputDialog::getItem(
            this,
            windowTitle(),
            QStringLiteral("tag service:"),
            available,
            0,
            false,
            &ok);

        if (!ok || name.isEmpty()) {
            return;
        }

        edit_rules(service_name_to_key.value(name), name, QJsonObject());
    });
    connect(edit, &QPushButton::clicked, this, [this]() {
        const QListWidgetItem *item = list->currentItem();

        if (item == nullptr) {
            return;
        }

        const QString key = item->data(Qt::UserRole).toString();
        edit_rules(key, service_name_to_key.key(key, key), filename_tagging.value(key).toObject());
    });
    connect(remove, &QPushButton::clicked, this, [this]() {
        const QListWidgetItem *item = list->currentItem();

        if (item == nullptr) {
            return;
        }

        const QString key = item->data(Qt::UserRole).toString();

        if (QMessageBox::question(
                this,
                windowTitle(),
                QStringLiteral("Remove the filename tagging rules for this service?"))
            != QMessageBox::Yes) {
            return;
        }

        filename_tagging.remove(key);
        rebuild();
    });
    connect(cancel, &QPushButton::clicked, this, &QDialog::close);
    connect(save, &QPushButton::clicked, this, [this]() { this->save(); });

    set_state(state);
}

void HydrusLocalImportTaggingDialog::rebuild() {
    list->clear();

    for (auto iterator = filename_tagging.constBegin(); iterator != filename_tagging.constEnd(); ++iterator) {
        const QString name = service_name_to_key.key(iterator.key(), iterator.key());
        auto *item = new QListWidgetItem(
            QStringLiteral("%1 - %2").arg(name, filename_tagging_summary(iterator.value().toObject())),
            list);
        item->setData(Qt::UserRole, iterator.key());
    }
}

void HydrusLocalImportTaggingDialog::edit_rules(
    const QString &key, const QString &name, const QJsonObject &current) {
    QJsonObject initial = current.isEmpty() ? default_filename_tagging_options() : current;

    if (!example_path.isEmpty()) {
        initial.insert(QStringLiteral("example_path"), example_path);
    }

    QJsonObject edited;

    if (!edit_filename_tagging_options(this, name, initial, &edited)) {
        return;
    }

    if (filename_tagging_summary(edited) == QStringLiteral("no tags")) {
        filename_tagging.remove(key);
    } else {
        filename_tagging.insert(key, edited);
    }

    rebuild();
}

void HydrusLocalImportTaggingDialog::save() {
    app_local_import_tagging_save(*main, filename_tagging_entries(filename_tagging), import_after_save);
    close();
}

void HydrusLocalImportTaggingDialog::set_state(const QJsonObject &state) {
    data = state;
    filename_tagging = state.value(QStringLiteral("filename_tagging")).toObject();
    example_path = state.value(QStringLiteral("example_path")).toString();
    import_after_save = state.value(QStringLiteral("import_after_save")).toBool(false);
    service_name_to_key.clear();

    for (const QJsonValue &value : state.value(QStringLiteral("tag_services")).toArray()) {
        const QJsonObject service = value.toObject();
        service_name_to_key.insert(
            service.value(QStringLiteral("name")).toString(),
            service.value(QStringLiteral("key")).toString());
    }

    help->setText(import_after_save
        ? QStringLiteral("A tag service can read tags from each file's path. When you save, "
                         "the import starts with these rules and your existing import options.")
        : QStringLiteral("A tag service can read tags from each file's path."));
    rebuild();
}

void HydrusMainWindow::show_local_import_tagging(const LocalImportTaggingDialogState &typed) {
    const QJsonObject state = QJsonDocument::fromJson(to_qstring(typed.json).toUtf8()).object();
    if (local_import_tagging_dialog == nullptr) {
        local_import_tagging_dialog = new HydrusLocalImportTaggingDialog(this, state, this);
        connect(local_import_tagging_dialog, &QObject::destroyed, this, [this]() {
            local_import_tagging_dialog = nullptr;
        });
        local_import_tagging_dialog->show();
    } else {
        local_import_tagging_dialog->set_state(state);
        local_import_tagging_dialog->raise();
        local_import_tagging_dialog->activateWindow();
    }
}
