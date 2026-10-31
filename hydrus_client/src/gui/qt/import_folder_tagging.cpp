#include "import_folder.hpp"

// WS-15.1.3b: import folder filename tagging moved from bridge.cpp. The helpers were file-static there; the three shared with the local import tagging dialog are now exported via import_folder.hpp. build_import_folder_tagging_section is the "metadata import" layout layer of show_import_folder; its this/&dialog captures became main/dialog parameters, logic and texts unchanged.

#include "bridge.hpp"
#include "support/support.hpp"

#include <QCheckBox>
#include <QDialog>
#include <QDir>
#include <QFileInfo>
#include <QFormLayout>
#include <QGridLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QInputDialog>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QListWidgetItem>
#include <QMessageBox>
#include <QObject>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QRegularExpression>
#include <QVBoxLayout>
#include <QWidget>

// One tag from a directory/regex rule: an empty namespace means the value is
// added unnamespaced.
QString join_filename_tag(const QString &tag_namespace, const QString &value) {
    const QString cleaned = value.trimmed();

    if (cleaned.isEmpty()) {
        return QString();
    }

    return tag_namespace.trimmed().isEmpty() ? cleaned : QStringLiteral("%1:%2").arg(tag_namespace.trimmed(), cleaned);
}

// Mirrors FilenameTaggingOptions::get_tags closely enough to preview rules
// while the dialog is open. The Rust side does the real cleaning and storage.
QString filename_tagging_preview(const QJsonObject &options, const QString &path_text) {
    QStringList tags;

    for (const QJsonValue &value : options.value(QStringLiteral("tags_for_all")).toArray()) {
        tags << value.toString();
    }

    const QFileInfo info(path_text.trimmed());

    if (options.value(QStringLiteral("add_filename")).toBool() && !info.completeBaseName().isEmpty()) {
        tags << join_filename_tag(options.value(QStringLiteral("filename_namespace")).toString(), info.completeBaseName());
    }

    QStringList directories = QDir::fromNativeSeparators(info.absolutePath()).split(QLatin1Char('/'), Qt::SkipEmptyParts);

    for (const QString &key : {QStringLiteral("0"), QStringLiteral("1"), QStringLiteral("2"),
                               QStringLiteral("-3"), QStringLiteral("-2"), QStringLiteral("-1")}) {
        const QJsonObject rule = options.value(QStringLiteral("directories")).toObject().value(key).toObject();

        if (!rule.value(QStringLiteral("enabled")).toBool()) {
            continue;
        }

        int index = key.toInt();
        const int resolved = index < 0 ? directories.size() + index : index;

        if (resolved < 0 || resolved >= directories.size()) {
            continue;
        }

        tags << join_filename_tag(rule.value(QStringLiteral("namespace")).toString(), directories.at(resolved));
    }

    auto collect = [&tags](const QString &pattern, const QString &tag_namespace, const QString &text) {
        const QRegularExpression regex(pattern);

        if (!regex.isValid()) {
            return;
        }

        QRegularExpressionMatchIterator iterator = regex.globalMatch(text);

        while (iterator.hasNext()) {
            const QRegularExpressionMatch match = iterator.next();

            if (regex.captureCount() > 0) {
                for (int group = 1; group <= regex.captureCount(); ++group) {
                    if (match.capturedStart(group) >= 0) {
                        tags << join_filename_tag(tag_namespace, match.captured(group));
                    }
                }
            } else {
                tags << join_filename_tag(tag_namespace, match.captured(0));
            }
        }
    };

    const QString path = QDir::fromNativeSeparators(path_text.trimmed());

    for (const QJsonValue &value : options.value(QStringLiteral("regexes")).toArray()) {
        collect(value.toString(), QString(), path);
    }

    for (const QJsonValue &value : options.value(QStringLiteral("quick_namespaces")).toArray()) {
        const QJsonObject rule = value.toObject();
        collect(rule.value(QStringLiteral("regex")).toString(), rule.value(QStringLiteral("namespace")).toString(), path);
    }

    QStringList cleaned;

    for (const QString &tag : tags) {
        const QString trimmed = tag.trimmed();

        if (!trimmed.isEmpty() && !cleaned.contains(trimmed, Qt::CaseInsensitive)) {
            cleaned << trimmed;
        }
    }

    cleaned.sort(Qt::CaseInsensitive);

    return cleaned.join(QStringLiteral(", "));
}

QString filename_tagging_summary(const QJsonObject &options) {
    QStringList statements;

    const QJsonArray all_tags = options.value(QStringLiteral("tags_for_all")).toArray();

    if (!all_tags.isEmpty()) {
        statements << QStringLiteral("%1 tag(s) for all").arg(all_tags.size());
    }

    if (options.value(QStringLiteral("add_filename")).toBool()) {
        statements << QStringLiteral("filename as '%1'").arg(options.value(QStringLiteral("filename_namespace")).toString());
    }

    int directories = 0;
    const QJsonObject rules = options.value(QStringLiteral("directories")).toObject();

    for (const QJsonValue &rule : rules) {
        if (rule.toObject().value(QStringLiteral("enabled")).toBool()) {
            ++directories;
        }
    }

    if (directories > 0) {
        statements << QStringLiteral("%1 directory rule(s)").arg(directories);
    }

    const int quick = options.value(QStringLiteral("quick_namespaces")).toArray().size();

    if (quick > 0) {
        statements << QStringLiteral("%1 namespace regex(es)").arg(quick);
    }

    const int regexes = options.value(QStringLiteral("regexes")).toArray().size();

    if (regexes > 0) {
        statements << QStringLiteral("%1 regex(es)").arg(regexes);
    }

    return statements.isEmpty() ? QStringLiteral("no tags") : statements.join(QStringLiteral(", "));
}

QJsonObject default_filename_tagging_options() {
    QJsonObject directories;

    for (const QString &key : {QStringLiteral("0"), QStringLiteral("1"), QStringLiteral("2"),
                               QStringLiteral("-3"), QStringLiteral("-2"), QStringLiteral("-1")}) {
        directories.insert(key, QJsonObject{
            {QStringLiteral("enabled"), false},
            {QStringLiteral("namespace"), QString()},
        });
    }

    return QJsonObject{
        {QStringLiteral("tags_for_all"), QJsonArray()},
        {QStringLiteral("add_filename"), false},
        {QStringLiteral("filename_namespace"), QStringLiteral("filename")},
        {QStringLiteral("directories"), directories},
        {QStringLiteral("quick_namespaces"), QJsonArray()},
        {QStringLiteral("regexes"), QJsonArray()},
    };
}

// The editor behind "add service"/"edit" on an import folder's filename
// tagging list. The shape matches importing::options::FilenameTaggingOptions.
bool edit_filename_tagging_options(QWidget *parent, const QString &service_name, const QJsonObject &initial, QJsonObject *result) {
    QDialog dialog(parent);
    dialog.setWindowTitle(QStringLiteral("filename tagging for %1").arg(service_name));
    dialog.resize(720, 760);

    auto *layout = new QVBoxLayout(&dialog);

    auto *help = new QLabel(
        QStringLiteral("Tags are read from the path of every successfully imported file. "
                       "Directory slots count from the start of the path (0, 1, 2) or from the end (-3, -2, -1)."),
        &dialog);
    help->setWordWrap(true);
    layout->addWidget(help);

    auto *form = new QFormLayout();

    auto *tags_for_all = new QPlainTextEdit(&dialog);
    tags_for_all->setPlaceholderText(QStringLiteral("one tag per line, added to every file"));
    tags_for_all->setMaximumHeight(70);
    tags_for_all->setPlainText([&initial]() {
        QStringList tags;
        for (const QJsonValue &value : initial.value(QStringLiteral("tags_for_all")).toArray()) {
            tags << value.toString();
        }
        return tags.join(QStringLiteral("\n"));
    }());
    form->addRow(QStringLiteral("tags for all files:"), tags_for_all);

    auto *add_filename = new QCheckBox(QStringLiteral("add the filename (without extension)"), &dialog);
    add_filename->setChecked(initial.value(QStringLiteral("add_filename")).toBool(true));
    auto *filename_namespace = new QLineEdit(initial.value(QStringLiteral("filename_namespace")).toString(QStringLiteral("filename")), &dialog);
    filename_namespace->setPlaceholderText(QStringLiteral("namespace (empty means no namespace)"));
    form->addRow(add_filename, filename_namespace);

    layout->addLayout(form);

    auto *directories = new QGroupBox(QStringLiteral("directories"), &dialog);
    auto *directory_grid = new QGridLayout(directories);
    directory_grid->addWidget(new QLabel(QStringLiteral("slot"), directories), 0, 0);
    directory_grid->addWidget(new QLabel(QStringLiteral("use"), directories), 0, 1);
    directory_grid->addWidget(new QLabel(QStringLiteral("namespace"), directories), 0, 2);

    QHash<QString, QCheckBox *> directory_checks;
    QHash<QString, QLineEdit *> directory_namespaces;
    const QJsonObject directory_rules = initial.value(QStringLiteral("directories")).toObject();

    int row = 1;
    for (const QString &key : {QStringLiteral("0"), QStringLiteral("1"), QStringLiteral("2"),
                               QStringLiteral("-3"), QStringLiteral("-2"), QStringLiteral("-1")}) {
        const QJsonObject rule = directory_rules.value(key).toObject();

        directory_grid->addWidget(new QLabel(key, directories), row, 0);

        auto *check = new QCheckBox(directories);
        check->setChecked(rule.value(QStringLiteral("enabled")).toBool());
        directory_grid->addWidget(check, row, 1);
        directory_checks.insert(key, check);

        auto *tag_namespace = new QLineEdit(rule.value(QStringLiteral("namespace")).toString(), directories);
        tag_namespace->setPlaceholderText(QStringLiteral("empty means no namespace"));
        directory_grid->addWidget(tag_namespace, row, 2);
        directory_namespaces.insert(key, tag_namespace);

        ++row;
    }

    layout->addWidget(directories);

    auto *quick_box = new QGroupBox(QStringLiteral("namespace regexes"), &dialog);
    auto *quick_layout = new QVBoxLayout(quick_box);
    auto *quick = new QTableWidget(quick_box);
    quick->setColumnCount(2);
    quick->setHorizontalHeaderLabels({QStringLiteral("namespace"), QStringLiteral("regex")});
    quick->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    quick->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    quick->verticalHeader()->setVisible(false);
    quick->setMinimumHeight(110);
    quick_layout->addWidget(quick);

    for (const QJsonValue &value : initial.value(QStringLiteral("quick_namespaces")).toArray()) {
        const QJsonObject rule = value.toObject();
        const int index = quick->rowCount();
        quick->insertRow(index);
        quick->setItem(index, 0, new QTableWidgetItem(rule.value(QStringLiteral("namespace")).toString()));
        quick->setItem(index, 1, new QTableWidgetItem(rule.value(QStringLiteral("regex")).toString()));
    }

    auto *quick_buttons = new QHBoxLayout();
    auto *add_quick = new QPushButton(QStringLiteral("add regex"), quick_box);
    auto *remove_quick = new QPushButton(QStringLiteral("remove regex"), quick_box);
    quick_buttons->addWidget(add_quick);
    quick_buttons->addWidget(remove_quick);
    quick_buttons->addStretch(1);
    quick_layout->addLayout(quick_buttons);
    layout->addWidget(quick_box);

    auto *regex_box = new QGroupBox(QStringLiteral("plain regexes"), &dialog);
    auto *regex_layout = new QVBoxLayout(regex_box);
    auto *regexes = new QPlainTextEdit(regex_box);
    regexes->setPlaceholderText(QStringLiteral("one regex per line; every match or capture group becomes a tag"));
    regexes->setMaximumHeight(80);
    regexes->setPlainText([&initial]() {
        QStringList patterns;
        for (const QJsonValue &value : initial.value(QStringLiteral("regexes")).toArray()) {
            patterns << value.toString();
        }
        return patterns.join(QStringLiteral("\n"));
    }());
    regex_layout->addWidget(regexes);
    layout->addWidget(regex_box);

    auto *example = new QLineEdit(initial.value(QStringLiteral("example_path")).toString(), &dialog);
    example->setPlaceholderText(QStringLiteral("example file path for the preview"));
    auto *preview = new QLabel(&dialog);
    preview->setWordWrap(true);
    preview->setObjectName(QStringLiteral("filename_tagging_preview"));

    auto *preview_box = new QGroupBox(QStringLiteral("preview"), &dialog);
    auto *preview_layout = new QVBoxLayout(preview_box);
    preview_layout->addWidget(example);
    preview_layout->addWidget(preview);
    layout->addWidget(preview_box);

    QJsonObject built;

    auto build = [&]() {
        QJsonObject options;
        QJsonArray all_tags;

        for (const QString &line : tags_for_all->toPlainText().split(QRegularExpression(QStringLiteral("[\\r\\n]+")), Qt::SkipEmptyParts)) {
            const QString trimmed = line.trimmed();
            if (!trimmed.isEmpty()) {
                all_tags.append(trimmed);
            }
        }

        options.insert(QStringLiteral("tags_for_all"), all_tags);
        options.insert(QStringLiteral("add_filename"), add_filename->isChecked());
        options.insert(QStringLiteral("filename_namespace"), filename_namespace->text().trimmed());

        QJsonObject directory_rules_out;
        for (auto iterator = directory_checks.constBegin(); iterator != directory_checks.constEnd(); ++iterator) {
            directory_rules_out.insert(iterator.key(), QJsonObject{
                {QStringLiteral("enabled"), iterator.value()->isChecked()},
                {QStringLiteral("namespace"), directory_namespaces.value(iterator.key())->text().trimmed()},
            });
        }
        options.insert(QStringLiteral("directories"), directory_rules_out);

        QJsonArray quick_rules;
        for (int index = 0; index < quick->rowCount(); ++index) {
            const QTableWidgetItem *tag_namespace = quick->item(index, 0);
            const QTableWidgetItem *pattern = quick->item(index, 1);

            if (pattern == nullptr || pattern->text().trimmed().isEmpty()) {
                continue;
            }

            quick_rules.append(QJsonObject{
                {QStringLiteral("namespace"), tag_namespace == nullptr ? QString() : tag_namespace->text().trimmed()},
                {QStringLiteral("regex"), pattern->text().trimmed()},
            });
        }
        options.insert(QStringLiteral("quick_namespaces"), quick_rules);

        QJsonArray patterns;
        for (const QString &line : regexes->toPlainText().split(QRegularExpression(QStringLiteral("[\\r\\n]+")), Qt::SkipEmptyParts)) {
            const QString trimmed = line.trimmed();
            if (!trimmed.isEmpty()) {
                patterns.append(trimmed);
            }
        }
        options.insert(QStringLiteral("regexes"), patterns);

        built = options;

        if (example->text().trimmed().isEmpty()) {
            preview->setText(QStringLiteral("set an example path to preview the tags"));
        } else {
            const QString tags = filename_tagging_preview(options, example->text());
            preview->setText(tags.isEmpty() ? QStringLiteral("no tags") : tags);
        }
    };

    QObject::connect(tags_for_all, &QPlainTextEdit::textChanged, &dialog, build);
    QObject::connect(add_filename, &QCheckBox::toggled, &dialog, build);
    QObject::connect(filename_namespace, &QLineEdit::textChanged, &dialog, build);
    QObject::connect(example, &QLineEdit::textChanged, &dialog, build);
    QObject::connect(regexes, &QPlainTextEdit::textChanged, &dialog, build);

    for (auto iterator = directory_checks.constBegin(); iterator != directory_checks.constEnd(); ++iterator) {
        QObject::connect(iterator.value(), &QCheckBox::toggled, &dialog, build);
        QObject::connect(directory_namespaces.value(iterator.key()), &QLineEdit::textChanged, &dialog, build);
    }

    QObject::connect(quick, &QTableWidget::cellChanged, &dialog, [build](int, int) { build(); });

    QObject::connect(add_quick, &QPushButton::clicked, &dialog, [quick]() {
        const int index = quick->rowCount();
        quick->insertRow(index);
        quick->setItem(index, 0, new QTableWidgetItem(QString()));
        quick->setItem(index, 1, new QTableWidgetItem(QString()));
        quick->editItem(quick->item(index, 0));
    });
    QObject::connect(remove_quick, &QPushButton::clicked, &dialog, [quick]() {
        const int index = quick->currentRow();
        if (index >= 0) {
            quick->removeRow(index);
        }
    });

    build();

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Save | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, [&dialog, build]() { build(); dialog.accept(); });

    if (dialog.exec() != QDialog::Accepted) {
        return false;
    }

    build();
    *result = built;

    return true;
}

void build_import_folder_tagging_section(
    HydrusMainWindow *main,
    QDialog *dialog,
    QWidget *content,
    QVBoxLayout *vbox,
    const QJsonObject &state,
    const QString &folder_path,
    QJsonObject &filename_tagging) {
    // metadata import
    QVBoxLayout *meta_box_layout = nullptr;
    auto *meta_box =
        make_static_box(QStringLiteral("metadata import"), content, &meta_box_layout);

    meta_box_layout->addWidget(new QLabel(QStringLiteral("filename tagging:"), meta_box));

    auto *tagging_list = new QListWidget(meta_box);
    tagging_list->setObjectName(QStringLiteral("import_folder_filename_tagging"));
    meta_box_layout->addWidget(tagging_list, 1);

    auto *tagging_buttons = new QHBoxLayout();
    auto *add_tagging = new QPushButton(QStringLiteral("add"), meta_box);
    auto *edit_tagging = new QPushButton(QStringLiteral("edit"), meta_box);
    auto *remove_tagging = new QPushButton(QStringLiteral("remove"), meta_box);
    tagging_buttons->addWidget(add_tagging);
    tagging_buttons->addWidget(edit_tagging);
    tagging_buttons->addWidget(remove_tagging);
    tagging_buttons->addStretch(1);
    meta_box_layout->addLayout(tagging_buttons);

    meta_box_layout->addWidget(new QLabel(QStringLiteral("sidecar importing:"), meta_box));

    vbox->addWidget(meta_box);

    const QJsonArray tag_services = state.value(QStringLiteral("tag_services")).toArray();
    QHash<QString, QString> service_name_to_key;

    for (const QJsonValue &value : tag_services) {
        const QJsonObject service = value.toObject();
        service_name_to_key.insert(service.value(QStringLiteral("name")).toString(), service.value(QStringLiteral("key")).toString());
    }


    auto refresh_tagging = [tagging_list, &filename_tagging, &service_name_to_key]() {
        tagging_list->clear();

        for (auto iterator = filename_tagging.constBegin(); iterator != filename_tagging.constEnd(); ++iterator) {
            const QString name = service_name_to_key.key(iterator.key(), iterator.key());
            auto *item = new QListWidgetItem(
                QStringLiteral("%1 - %2").arg(name, filename_tagging_summary(iterator.value().toObject())),
                tagging_list);
            item->setData(Qt::UserRole, iterator.key());
        }
    };

    auto open_tagging_editor = [main, dialog, &filename_tagging, &service_name_to_key, folder_path](const QString &key, const QString &name, const QJsonObject &current) {
        QJsonObject initial = current.isEmpty() ? default_filename_tagging_options() : current;
        initial.insert(QStringLiteral("example_path"), folder_path);

        QJsonObject edited;

        if (!edit_filename_tagging_options(dialog, name, initial, &edited)) {
            return;
        }

        if (filename_tagging_summary(edited) == QStringLiteral("no tags")) {
            filename_tagging.remove(key);
        } else {
            filename_tagging.insert(key, edited);
        }
    };

    QObject::connect(add_tagging, &QPushButton::clicked, dialog, [main, dialog, &filename_tagging, &service_name_to_key, open_tagging_editor, refresh_tagging]() {
        QStringList available;

        for (auto iterator = service_name_to_key.constBegin(); iterator != service_name_to_key.constEnd(); ++iterator) {
            if (!filename_tagging.contains(iterator.value())) {
                available << iterator.key();
            }
        }

        if (available.isEmpty()) {
            QMessageBox::information(
                main,
                QStringLiteral("filename tagging"),
                QStringLiteral("Create a local tag service first, or every local tag service already has rules."));
            return;
        }

        available.sort();

        bool ok = false;
        const QString name = QInputDialog::getItem(
            dialog,
            QStringLiteral("add filename tagging"),
            QStringLiteral("tag service:"),
            available,
            0,
            false,
            &ok);

        if (!ok || name.isEmpty()) {
            return;
        }

        open_tagging_editor(service_name_to_key.value(name), name, QJsonObject());
        refresh_tagging();
    });

    QObject::connect(edit_tagging, &QPushButton::clicked, dialog, [&open_tagging_editor, tagging_list, &filename_tagging, &service_name_to_key, refresh_tagging]() {
        const QListWidgetItem *item = tagging_list->currentItem();

        if (item == nullptr) {
            return;
        }

        const QString key = item->data(Qt::UserRole).toString();
        open_tagging_editor(key, service_name_to_key.key(key, key), filename_tagging.value(key).toObject());
        refresh_tagging();
    });

    QObject::connect(remove_tagging, &QPushButton::clicked, dialog, [tagging_list, &filename_tagging, refresh_tagging]() {
        const QListWidgetItem *item = tagging_list->currentItem();

        if (item == nullptr) {
            return;
        }

        const QString key = item->data(Qt::UserRole).toString();

        if (QMessageBox::question(
                nullptr,
                QStringLiteral("filename tagging"),
                QStringLiteral("remove filename tagging for this service?"))
            != QMessageBox::Yes) {
            return;
        }

        filename_tagging.remove(key);
        refresh_tagging();
    });

    refresh_tagging();

    if (tag_services.isEmpty()) {
        add_tagging->setEnabled(false);
        add_tagging->setToolTip(QStringLiteral("no local tag service exists yet"));
    }
}
